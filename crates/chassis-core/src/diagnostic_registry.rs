//! ADR ⇄ rule-ID registry plus governance helpers for diagnostic emission.
//!
//! Loads every `docs/adr/*.md` frontmatter and indexes the `enforces[].rule`
//! tokens that each accepted ADR carries (ADR-0011 grammar). The registry is
//! the source of truth that lets emitters and tests answer:
//!
//! - Does this `ruleId` resolve to an ADR's `enforces[]`?
//! - Does this `violated.convention` reference a real ADR file?
//!
//! ADRs whose `status` is `proposed` or `superseded` are excluded — only
//! accepted bindings are considered governance-active.
//!
//! [`INTERNAL_NON_WIRE_RULE_IDS`] documents the rule IDs that are deliberately
//! **not** emitted as wire-bound `Diagnostic` rows: callers raise them as typed
//! errors (e.g. [`crate::diff::DiffError::Parse`]) and the binding tests carve
//! them out so they do not need an ADR mapping.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Rule IDs that are internal-only and never appear on the diagnostic wire.
/// Each entry is documented in the ADR pointed at by the surface that raises
/// it (e.g. ADR-0019 carves out `CH-DIFF-PARSE-ERROR`).
pub const INTERNAL_NON_WIRE_RULE_IDS: &[&str] = &[
    // ADR-0019 §"Parse failures": diff returns `DiffError::Parse(String)` and
    // never produces a `Diagnostic` carrying this id.
    "CH-DIFF-PARSE-ERROR",
];

/// Snapshot of every `enforces[].rule` entry across accepted ADRs.
#[derive(Debug, Clone, Default)]
pub struct AdrRuleRegistry {
    /// rule id → ADR id that enforces it (first wins on duplicate, which is a
    /// governance violation surfaced separately by the existing ADR ⇄ kernel
    /// binding tests).
    rule_to_adr: BTreeMap<String, String>,
    /// All ADR ids encountered (regardless of status).
    adrs: BTreeSet<String>,
}

#[derive(Debug, Deserialize)]
struct AdrFrontmatter {
    id: Option<String>,
    status: Option<String>,
    enforces: Option<Vec<Enforce>>,
}

#[derive(Debug, Deserialize)]
struct Enforce {
    rule: String,
}

impl AdrRuleRegistry {
    /// Load `docs/adr/*.md` under `repo_root`. ADRs whose status is `proposed`
    /// or `superseded` contribute their id to [`Self::knows_adr`] but their
    /// `enforces[]` entries are skipped — the registry is intentionally the
    /// *active* binding set.
    pub fn load(repo_root: &Path) -> Result<Self, String> {
        let adr_dir = repo_root.join("docs/adr");
        let mut reg = AdrRuleRegistry::default();
        let entries =
            fs::read_dir(&adr_dir).map_err(|e| format!("read_dir {}: {e}", adr_dir.display()))?;
        for ent in entries {
            let ent = ent.map_err(|e| format!("dir entry: {e}"))?;
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let raw =
                fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let Some(fm) = extract_frontmatter(&raw) else {
                continue;
            };
            let meta: AdrFrontmatter = serde_yaml::from_str(&fm)
                .map_err(|e| format!("yaml frontmatter {}: {e}", path.display()))?;
            let Some(adr_id) = meta.id.clone() else {
                continue;
            };
            reg.adrs.insert(adr_id.clone());
            if status_is_inactive(&meta.status) {
                continue;
            }
            let Some(enforces) = meta.enforces else {
                continue;
            };
            for row in enforces {
                let rule = row.rule.trim().to_string();
                if rule.is_empty() {
                    continue;
                }
                reg.rule_to_adr
                    .entry(rule)
                    .or_insert_with(|| adr_id.clone());
            }
        }
        Ok(reg)
    }

    /// Try common locations for the repo root and load the registry. Used by
    /// integration tests that do not receive an explicit path.
    pub fn load_from_manifest_or_cwd() -> Result<Self, String> {
        for candidate in candidate_repo_roots() {
            if candidate.join("docs/adr").is_dir() {
                return Self::load(&candidate);
            }
        }
        Err("could not locate docs/adr from CARGO_MANIFEST_DIR or CWD".into())
    }

    /// ADR id that enforces `rule_id`, if any.
    pub fn adr_for_rule(&self, rule_id: &str) -> Option<&str> {
        self.rule_to_adr.get(rule_id).map(String::as_str)
    }

    /// True iff `adr_id` was observed in `docs/adr` (any status).
    pub fn knows_adr(&self, adr_id: &str) -> bool {
        self.adrs.contains(adr_id)
    }

    /// All bound rule ids in deterministic order.
    pub fn iter_rules(&self) -> impl Iterator<Item = &str> {
        self.rule_to_adr.keys().map(String::as_str)
    }

    /// All ADR ids in deterministic order.
    pub fn iter_adrs(&self) -> impl Iterator<Item = &str> {
        self.adrs.iter().map(String::as_str)
    }

    /// Total number of (rule, ADR) bindings.
    pub fn len(&self) -> usize {
        self.rule_to_adr.len()
    }

    /// True iff no rules have been registered.
    pub fn is_empty(&self) -> bool {
        self.rule_to_adr.is_empty()
    }
}

fn extract_frontmatter(raw: &str) -> Option<String> {
    let body = raw.strip_prefix("---\n")?;
    let end = body.find("\n---\n")?;
    Some(body[..end].to_string())
}

fn status_is_inactive(status: &Option<String>) -> bool {
    let Some(raw) = status else { return false };
    matches!(raw.to_ascii_lowercase().as_str(), "proposed" | "superseded")
}

fn candidate_repo_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(env) = std::env::var("CHASSIS_REPO_ROOT") {
        out.push(PathBuf::from(env));
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    out.push(manifest.join("../.."));
    out.push(manifest.join(".."));
    out.push(manifest.clone());
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd);
    }
    out.into_iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        AdrRuleRegistry::load_from_manifest_or_cwd()
            .map(|_| ())
            .unwrap();
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root")
    }

    #[test]
    fn loads_accepted_bindings_and_excludes_proposed() {
        let r = AdrRuleRegistry::load(&root()).expect("registry loads");
        assert!(!r.is_empty(), "registry should not be empty");
        // ADR-0021 enforces CH-RUST-METADATA-CONTRACT.
        assert_eq!(
            r.adr_for_rule("CH-RUST-METADATA-CONTRACT"),
            Some("ADR-0021")
        );
        // ADR-0019 enforces every CH-DIFF-* rule except the carve-out.
        assert_eq!(r.adr_for_rule("CH-DIFF-NAME-CHANGED"), Some("ADR-0019"));
        // ADR-0023 enforces the trace rules.
        assert_eq!(r.adr_for_rule("CH-TRACE-MALFORMED-CLAIM"), Some("ADR-0023"));
        // ADR-0020 enforces CH-EXEMPT-* rules.
        assert_eq!(r.adr_for_rule("CH-EXEMPT-EXPIRED"), Some("ADR-0020"));
        // ADR-0024 enforces drift rules.
        assert_eq!(r.adr_for_rule("CH-DRIFT-CLAIM-STALE"), Some("ADR-0024"));
    }

    #[test]
    fn unknown_rule_returns_none() {
        let r = AdrRuleRegistry::load(&root()).expect("registry loads");
        assert!(r.adr_for_rule("CH-NOT-A-REAL-RULE").is_none());
    }

    #[test]
    fn knows_adr_ids_present_in_tree() {
        let r = AdrRuleRegistry::load(&root()).expect("registry loads");
        assert!(r.knows_adr("ADR-0011"));
        assert!(r.knows_adr("ADR-0018"));
        assert!(!r.knows_adr("ADR-9999"));
    }

    #[test]
    fn internal_non_wire_ids_are_not_in_active_registry() {
        let r = AdrRuleRegistry::load(&root()).expect("registry loads");
        for rid in INTERNAL_NON_WIRE_RULE_IDS {
            assert!(
                r.adr_for_rule(rid).is_none(),
                "{rid} is documented internal but appears in active ADR enforces[]"
            );
        }
    }
}
