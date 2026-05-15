//! Every wire `ruleId` emitted by the Rust kernel (`CH-*` diagnostics) resolves to
//! an `enforces[].rule:` entry under `docs/adr/*.md`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::diff::{
    CH_DIFF_ASSURANCE_DEMOTED, CH_DIFF_ASSURANCE_PROMOTED, CH_DIFF_CLAIM_ADDED,
    CH_DIFF_CLAIM_ID_CHANGED, CH_DIFF_CLAIM_REMOVED, CH_DIFF_CLAIM_TEXT_CHANGED,
    CH_DIFF_EDGE_CASE_PROMOTED_TO_INVARIANT, CH_DIFF_INVARIANT_DEMOTED_TO_EDGE_CASE,
    CH_DIFF_KIND_CHANGED, CH_DIFF_NAME_CHANGED, CH_DIFF_OWNER_CHANGED,
    CH_DIFF_REQUIRED_KIND_FIELD_REMOVED, CH_DIFF_STATUS_CHANGED,
    CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR, CH_DIFF_VERSION_DOWNGRADED,
    CH_DIFF_VERSION_MAJOR_WITHOUT_BREAKING, CH_DIFF_VERSION_MISSING, CH_DIFF_VERSION_NOT_BUMPED,
};
use crate::exempt::rule_id as exempt_rid;

#[derive(Debug, Deserialize)]
struct AdrFrontmatter {
    enforces: Option<Vec<Enforce>>,
}

#[derive(Debug, Deserialize)]
struct Enforce {
    rule: String,
}

fn repo_root_from_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root via CARGO_MANIFEST_DIR/../.. ")
}

fn extract_frontmatter(raw: &str) -> Option<String> {
    let body = raw.strip_prefix("---\n")?;
    let end = body.find("\n---\n")?;
    Some(body[..end].to_string())
}

fn all_adr_rule_ids(root: &Path) -> BTreeSet<String> {
    let adr_dir = root.join("docs/adr");
    let mut ids = BTreeSet::new();
    for dir in fs::read_dir(&adr_dir).unwrap_or_else(|e| panic!("read_dir {:?}: {}", adr_dir, e)) {
        let p = dir.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {}", p.display(), e));
        let Some(fm_text) = extract_frontmatter(&text) else {
            continue;
        };
        let meta: Result<AdrFrontmatter, _> = serde_yaml::from_str(&fm_text);
        let meta = meta.unwrap_or_else(|e| panic!("yaml frontmatter {:?}: {}", p.display(), e));
        if let Some(enforces) = meta.enforces {
            for row in enforces {
                ids.insert(row.rule.trim().to_string());
            }
        }
    }
    ids
}

/// Kernel-produced wire rule IDs (`CH-*`) excluding test-only scaffolding and PARSE-only errors.
#[test]
fn every_kernel_ch_rule_is_bound_in_an_accepted_adr() {
    let kern: BTreeSet<String> = [
        // Contract validation (`validators.rs`).
        "CH-RUST-METADATA-CONTRACT",
        // contract-diff (`diff/mod.rs`; PARSE intentionally not ADR-wire per ADR-0019).
        CH_DIFF_KIND_CHANGED,
        CH_DIFF_NAME_CHANGED,
        CH_DIFF_VERSION_MISSING,
        CH_DIFF_VERSION_NOT_BUMPED,
        CH_DIFF_VERSION_DOWNGRADED,
        CH_DIFF_VERSION_MAJOR_WITHOUT_BREAKING,
        CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR,
        CH_DIFF_CLAIM_REMOVED,
        CH_DIFF_CLAIM_ID_CHANGED,
        CH_DIFF_CLAIM_TEXT_CHANGED,
        CH_DIFF_CLAIM_ADDED,
        CH_DIFF_INVARIANT_DEMOTED_TO_EDGE_CASE,
        CH_DIFF_EDGE_CASE_PROMOTED_TO_INVARIANT,
        CH_DIFF_ASSURANCE_DEMOTED,
        CH_DIFF_ASSURANCE_PROMOTED,
        CH_DIFF_STATUS_CHANGED,
        CH_DIFF_OWNER_CHANGED,
        CH_DIFF_REQUIRED_KIND_FIELD_REMOVED,
        exempt_rid::QUOTA_EXCEEDED,
        exempt_rid::LIFETIME_EXCEEDED,
        exempt_rid::EXPIRED,
        exempt_rid::MISSING_CODEOWNERS,
        exempt_rid::DUPLICATE_ID,
        exempt_rid::MALFORMED_ID,
        exempt_rid::RULE_NOT_IN_ADR,
        exempt_rid::REMOVED_BY_SWEEPER,
        exempt_rid::PATHS_EMPTY,
        exempt_rid::CODEOWNERS_PARSE_ERROR,
        exempt_rid::NOT_FOUND,
        exempt_rid::REVOKED,
        exempt_rid::LEGACY_ALIAS,
        exempt_rid::GLOBAL_WITHOUT_OPT_IN,
        exempt_rid::MISSING_RULE_OR_FINDING,
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();

    let root = repo_root_from_manifest();
    let adr_ids = all_adr_rule_ids(&root);
    let missing: Vec<_> = kern.difference(&adr_ids).cloned().collect();
    assert!(
        missing.is_empty(),
        "these kernel rule IDs are missing from docs/adr enforces[] lists: {:?}\n adr had {} entries",
        missing,
        adr_ids.len()
    );
}
