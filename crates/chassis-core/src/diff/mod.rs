//! Contract diff engine.
//!
//! Compares two CONTRACT JSON documents and emits a structured [`DiffReport`]
//! whose findings conform to the diagnostic envelope of ADR-0018
//! (`schemas/diagnostic.schema.json`).
//!
//! The diff-specific `breaking` / `non-breaking` / `additive` classification
//! lives inside each diagnostic's `detail.classification` key — the wire
//! envelope keeps the ADR-0018-mandated `severity` of `error` / `warning` /
//! `info`. `finding_is_breaking` and [`DiffReport::has_breaking`]
//! preserve the consumer-facing semantic.
//!
//! # Rule IDs (CH-DIFF-*)
//!
//! Per ADR-0011 every rule ID is stable and ADR-bound. The rules emitted by
//! this module are enumerated in `docs/adr/ADR-0019-contract-diff-rules.md`.
//! Mirror constants:
//!
//! - [`CH_DIFF_KIND_CHANGED`] (breaking)
//! - [`CH_DIFF_NAME_CHANGED`] (breaking)
//! - [`CH_DIFF_VERSION_MISSING`] (breaking)
//! - [`CH_DIFF_VERSION_NOT_BUMPED`] (breaking)
//! - [`CH_DIFF_VERSION_DOWNGRADED`] (breaking)
//! - [`CH_DIFF_VERSION_MAJOR_WITHOUT_BREAKING`] (warning, non-breaking)
//! - [`CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR`] (breaking)
//! - [`CH_DIFF_CLAIM_REMOVED`] (breaking)
//! - [`CH_DIFF_CLAIM_ID_CHANGED`] (breaking) — emitted as remove + add pair
//! - [`CH_DIFF_CLAIM_TEXT_CHANGED`] (non-breaking)
//! - [`CH_DIFF_CLAIM_ADDED`] (additive)
//! - [`CH_DIFF_INVARIANT_DEMOTED_TO_EDGE_CASE`] (breaking)
//! - [`CH_DIFF_EDGE_CASE_PROMOTED_TO_INVARIANT`] (non-breaking)
//! - [`CH_DIFF_ASSURANCE_DEMOTED`] (breaking)
//! - [`CH_DIFF_ASSURANCE_PROMOTED`] (non-breaking)
//! - [`CH_DIFF_STATUS_CHANGED`] (non-breaking)
//! - [`CH_DIFF_OWNER_CHANGED`] (non-breaking)
//! - [`CH_DIFF_REQUIRED_KIND_FIELD_REMOVED`] (breaking)
//! - [`CH_DIFF_PARSE_ERROR`] — surfaced via [`DiffError::Parse`], not as a
//!   [`Diagnostic`].

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod claims;
pub mod classify;
pub mod envelope;

use crate::diagnostic::{Diagnostic, Severity};
use crate::diff::classify::{kind_required_fields, ladder_rank};

pub const CH_DIFF_KIND_CHANGED: &str = "CH-DIFF-KIND-CHANGED";
pub const CH_DIFF_NAME_CHANGED: &str = "CH-DIFF-NAME-CHANGED";
pub const CH_DIFF_VERSION_MISSING: &str = "CH-DIFF-VERSION-MISSING";
pub const CH_DIFF_VERSION_NOT_BUMPED: &str = "CH-DIFF-VERSION-NOT-BUMPED";
pub const CH_DIFF_VERSION_DOWNGRADED: &str = "CH-DIFF-VERSION-DOWNGRADED";
pub const CH_DIFF_VERSION_MAJOR_WITHOUT_BREAKING: &str = "CH-DIFF-VERSION-MAJOR-WITHOUT-BREAKING";
pub const CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR: &str = "CH-DIFF-VERSION-BREAKING-WITHOUT-MAJOR";
pub const CH_DIFF_CLAIM_REMOVED: &str = "CH-DIFF-CLAIM-REMOVED";
pub const CH_DIFF_CLAIM_ID_CHANGED: &str = "CH-DIFF-CLAIM-ID-CHANGED";
pub const CH_DIFF_CLAIM_TEXT_CHANGED: &str = "CH-DIFF-CLAIM-TEXT-CHANGED";
pub const CH_DIFF_CLAIM_ADDED: &str = "CH-DIFF-CLAIM-ADDED";
pub const CH_DIFF_INVARIANT_DEMOTED_TO_EDGE_CASE: &str = "CH-DIFF-INVARIANT-DEMOTED-TO-EDGE-CASE";
pub const CH_DIFF_EDGE_CASE_PROMOTED_TO_INVARIANT: &str = "CH-DIFF-EDGE-CASE-PROMOTED-TO-INVARIANT";
pub const CH_DIFF_ASSURANCE_DEMOTED: &str = "CH-DIFF-ASSURANCE-DEMOTED";
pub const CH_DIFF_ASSURANCE_PROMOTED: &str = "CH-DIFF-ASSURANCE-PROMOTED";
pub const CH_DIFF_STATUS_CHANGED: &str = "CH-DIFF-STATUS-CHANGED";
pub const CH_DIFF_OWNER_CHANGED: &str = "CH-DIFF-OWNER-CHANGED";
pub const CH_DIFF_REQUIRED_KIND_FIELD_REMOVED: &str = "CH-DIFF-REQUIRED-KIND-FIELD-REMOVED";
pub const CH_DIFF_PARSE_ERROR: &str = "CH-DIFF-PARSE-ERROR";

pub const SOURCE: &str = "chassis diff";
pub const ADR_REF: &str = "ADR-0019";

/// Diagnostic schema version this module emits against (ADR-0018 envelope).
pub const DIAGNOSTIC_SCHEMA_VERSION: &str = "1.2.0";

/// Diff-specific classification carried inside `detail.classification`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    Breaking,
    NonBreaking,
    Additive,
}

/// Parse `detail.classification` from a canonical [`Diagnostic`].
pub(crate) fn finding_classification(d: &Diagnostic) -> Option<Classification> {
    d.detail.as_ref()?.get("classification").and_then(|v| {
        serde_json::from_value::<Classification>(v.clone()).ok()
    })
}

/// True when the finding carries diff-domain classification `breaking`.
pub(crate) fn finding_is_breaking(d: &Diagnostic) -> bool {
    finding_classification(d) == Some(Classification::Breaking)
}

/// Full diff result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub schema_version: String,
    pub findings: Vec<Diagnostic>,
}

impl DiffReport {
    pub fn has_breaking(&self) -> bool {
        self.findings.iter().any(|d| finding_is_breaking(d))
    }

    pub fn count_by_classification(&self, c: Classification) -> usize {
        self.findings
            .iter()
            .filter(|d| finding_classification(d) == Some(c))
            .count()
    }

    pub fn count_by_severity(&self, s: Severity) -> usize {
        self.findings.iter().filter(|d| d.severity == s).count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffError {
    Parse(String),
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffError::Parse(msg) => write!(f, "{CH_DIFF_PARSE_ERROR}: {msg}"),
        }
    }
}

impl std::error::Error for DiffError {}

/// Compute the diff between two CONTRACT JSON documents.
///
/// Inputs must be objects shaped like the canonical contract schema. This
/// function does not re-validate against `schemas/contract.schema.json` —
/// the caller is responsible for that.
pub fn diff(old: &Value, new: &Value) -> Result<DiffReport, DiffError> {
    let old_obj = require_object(old, "old")?;
    let new_obj = require_object(new, "new")?;

    let mut findings: Vec<Diagnostic> = Vec::new();

    let old_name = old_obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let new_name = new_obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let subject_prefix = if !new_name.is_empty() {
        format!("contract<{new_name}>")
    } else if !old_name.is_empty() {
        format!("contract<{old_name}>")
    } else {
        "contract".to_string()
    };

    // 1. Identity (name) check.
    if old_name != new_name {
        findings.push(envelope::breaking(
            CH_DIFF_NAME_CHANGED,
            &subject_prefix,
            format!("contract name changed: {old_name:?} -> {new_name:?}"),
            json!({ "before": old_name, "after": new_name }),
        ));
    }

    // 2. Kind check.
    let old_kind = old_obj.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    let new_kind = new_obj.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    let kinds_match = old_kind == new_kind;
    if !kinds_match {
        findings.push(envelope::breaking(
            CH_DIFF_KIND_CHANGED,
            &format!("{subject_prefix}.kind"),
            format!("contract kind changed: {old_kind:?} -> {new_kind:?}"),
            json!({ "before": old_kind, "after": new_kind }),
        ));
    }

    // 4. Claims diff (run before version so version logic can see breakage).
    claims::diff_claim_sets(old_obj, new_obj, &subject_prefix, &mut findings);

    // 5. Assurance ladder.
    let old_al = old_obj
        .get("assurance_level")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let new_al = new_obj
        .get("assurance_level")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if old_al != new_al && !old_al.is_empty() && !new_al.is_empty() {
        let (old_r, new_r) = (ladder_rank(old_al), ladder_rank(new_al));
        match (old_r, new_r) {
            (Some(o), Some(n)) if n < o => findings.push(envelope::breaking(
                CH_DIFF_ASSURANCE_DEMOTED,
                &format!("{subject_prefix}.assurance_level"),
                format!("assurance_level demoted: {old_al} -> {new_al}"),
                json!({ "before": old_al, "after": new_al }),
            )),
            (Some(o), Some(n)) if n > o => findings.push(envelope::non_breaking(
                CH_DIFF_ASSURANCE_PROMOTED,
                &format!("{subject_prefix}.assurance_level"),
                format!("assurance_level promoted: {old_al} -> {new_al}"),
                json!({ "before": old_al, "after": new_al }),
            )),
            _ => {}
        }
    }

    // 6. Status diff.
    let old_status = old_obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let new_status = new_obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if old_status != new_status && !old_status.is_empty() && !new_status.is_empty() {
        findings.push(envelope::non_breaking(
            CH_DIFF_STATUS_CHANGED,
            &format!("{subject_prefix}.status"),
            format!("status changed: {old_status} -> {new_status}"),
            json!({ "before": old_status, "after": new_status }),
        ));
    }

    // 7. Owner diff.
    let old_owner = old_obj.get("owner").and_then(|v| v.as_str()).unwrap_or("");
    let new_owner = new_obj.get("owner").and_then(|v| v.as_str()).unwrap_or("");
    if old_owner != new_owner && !old_owner.is_empty() && !new_owner.is_empty() {
        findings.push(envelope::non_breaking(
            CH_DIFF_OWNER_CHANGED,
            &format!("{subject_prefix}.owner"),
            format!("owner changed: {old_owner} -> {new_owner}"),
            json!({ "before": old_owner, "after": new_owner }),
        ));
    }

    // 8. Kind-specific required-field removal (only when kind matches).
    if kinds_match && !new_kind.is_empty() {
        if let Some(required) = kind_required_fields(new_kind) {
            for field in required {
                let in_old = old_obj.contains_key(*field);
                let in_new = new_obj.contains_key(*field);
                if in_old && !in_new {
                    findings.push(envelope::breaking(
                        CH_DIFF_REQUIRED_KIND_FIELD_REMOVED,
                        &format!("{subject_prefix}.{field}"),
                        format!(
                            "kind-required field '{field}' removed for kind '{new_kind}'"
                        ),
                        json!({ "field": field, "kind": new_kind }),
                    ));
                }
            }
        }
    }

    // 3. Version check — last, so it can observe whether any breaking diff was seen.
    let old_version = old_obj.get("version").and_then(|v| v.as_str());
    let new_version = new_obj.get("version").and_then(|v| v.as_str());
    diff_version(
        old_version,
        new_version,
        &subject_prefix,
        breaking_so_far(&findings),
        &mut findings,
    );

    Ok(DiffReport {
        schema_version: DIAGNOSTIC_SCHEMA_VERSION.to_string(),
        findings,
    })
}

fn breaking_so_far(findings: &[Diagnostic]) -> bool {
    findings.iter().any(|d| finding_is_breaking(d))
}

fn diff_version(
    old: Option<&str>,
    new: Option<&str>,
    subject_prefix: &str,
    breaking_seen: bool,
    findings: &mut Vec<Diagnostic>,
) {
    let subject = format!("{subject_prefix}.version");

    let Some(new_v) = new else {
        findings.push(envelope::breaking(
            CH_DIFF_VERSION_MISSING,
            &subject,
            "new contract is missing required field 'version' (ADR-0008)".into(),
            json!({ "field": "version" }),
        ));
        return;
    };

    let Some(old_v) = old else {
        // No old version is unusual but not categorized: caller's responsibility.
        // Don't emit anything besides the missing-field-on-new check above.
        return;
    };

    let old_sv = semver::Version::parse(old_v).ok();
    let new_sv = semver::Version::parse(new_v).ok();

    let (Some(old_sv), Some(new_sv)) = (old_sv, new_sv) else {
        // Schema enforces `^\d+\.\d+\.\d+$`, so unparseable means caller bypassed
        // validation. Treat as missing.
        findings.push(envelope::breaking(
            CH_DIFF_VERSION_MISSING,
            &subject,
            format!(
                "version field not parseable as semver (old={old_v:?}, new={new_v:?})"
            ),
            json!({ "before": old_v, "after": new_v }),
        ));
        return;
    };

    if new_sv == old_sv {
        if breaking_seen {
            findings.push(envelope::breaking(
                CH_DIFF_VERSION_NOT_BUMPED,
                &subject,
                format!(
                    "breaking change detected but version not bumped (still {old_v})"
                ),
                json!({ "before": old_v, "after": new_v }),
            ));
        }
        return;
    }

    if new_sv < old_sv {
        findings.push(envelope::breaking(
            CH_DIFF_VERSION_DOWNGRADED,
            &subject,
            format!("version downgraded: {old_v} -> {new_v}"),
            json!({ "before": old_v, "after": new_v }),
        ));
        return;
    }

    let major_bump = new_sv.major > old_sv.major;
    let minor_or_patch_bump = !major_bump && new_sv > old_sv;

    if major_bump && !breaking_seen {
        findings.push(envelope::warning_non_breaking(
            CH_DIFF_VERSION_MAJOR_WITHOUT_BREAKING,
            &subject,
            format!(
                "major version bumped ({old_v} -> {new_v}) but no breaking changes detected"
            ),
            json!({ "before": old_v, "after": new_v }),
        ));
    }

    if minor_or_patch_bump && breaking_seen {
        findings.push(envelope::breaking(
            CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR,
            &subject,
            format!(
                "breaking changes detected but version bump is not major ({old_v} -> {new_v})"
            ),
            json!({ "before": old_v, "after": new_v }),
        ));
    }
}

fn require_object<'a>(
    v: &'a Value,
    label: &str,
) -> Result<&'a serde_json::Map<String, Value>, DiffError> {
    match v.as_object() {
        Some(o) if !o.is_empty() => Ok(o),
        Some(_) => Err(DiffError::Parse(format!(
            "{label} contract is an empty object; not a recognizable Contract shape"
        ))),
        None => Err(DiffError::Parse(format!(
            "{label} contract is not a JSON object"
        ))),
    }
}

#[cfg(test)]
mod tests;
