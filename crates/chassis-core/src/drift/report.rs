#![forbid(unsafe_code)]

//! Drift report assembly: git inputs + pure score (`drift::score`).

use std::path::Path;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::diagnostic::{Diagnostic, Severity, Violated};
use crate::drift::git::{commits_touching_file_since, last_claim_edit, last_file_edit, GitError};
use crate::drift::score::{score, DriftKind};
use crate::trace::types::{SiteKind, TraceGraph};

pub const RULE_IMPL_MISSING: &str = "CH-DRIFT-IMPL-MISSING";

static DRIFT_SCHEMA_STR: &str = include_str!("../../../../schemas/drift-report.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(DRIFT_SCHEMA_STR).expect("drift-report schema JSON");
    jsonschema::validator_for(&schema).expect("compile drift-report schema")
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftSummaryCounts {
    pub stale: usize,
    pub abandoned: usize,
    pub missing: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub version: i32,
    pub summary: DriftSummaryCounts,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct DriftError(pub String);

impl std::fmt::Display for DriftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DriftError {}

impl From<GitError> for DriftError {
    fn from(value: GitError) -> Self {
        DriftError(value.0)
    }
}

/// Per ADR-0024: score each claim node; emit diagnostics per drift band.
pub fn build_drift_report(
    repo: &Path,
    trace: &TraceGraph,
    now: DateTime<Utc>,
) -> Result<DriftReport, DriftError> {
    let mut diagnostics = Vec::new();
    let mut stale = 0usize;
    let mut abandoned = 0usize;
    let mut missing = 0usize;

    for (cid, node) in &trace.claims {
        let contract_path = &node.contract_path;

        if node.impl_sites.is_empty() {
            missing += 1;
            diagnostics.push(diagnostic(
                RULE_IMPL_MISSING,
                Severity::Error,
                format!("claim `{cid}` has no backing implementation sites in the trace graph"),
                Some(format!("{}::{}", node.contract_path.display(), cid)),
            ));
            continue;
        }

        let rel_impl = node
            .impl_sites
            .iter()
            .find(|s| matches!(s.kind, SiteKind::Impl))
            .or_else(|| node.impl_sites.first())
            .map(|s| s.file.clone())
            .expect("impl sites non-empty");

        let claim_edit = match last_claim_edit(repo, contract_path, cid) {
            Ok(o) => o,
            Err(e) => return Err(e.into()),
        };
        let Some(claim_last_edit) = claim_edit else {
            continue;
        };

        let impl_last_edit = last_file_edit(repo, &rel_impl).map_err(DriftError::from)?;
        let Some(impl_last_edit) = impl_last_edit else {
            missing += 1;
            diagnostics.push(diagnostic(
                RULE_IMPL_MISSING,
                Severity::Error,
                format!(
                    "implementation file `{}` has no git history (claim `{cid}`)",
                    rel_impl.display()
                ),
                Some(rel_impl.display().to_string()),
            ));
            continue;
        };

        let churn = commits_touching_file_since(repo, &rel_impl, claim_last_edit)
            .map_err(DriftError::from)?;

        let (_raw, band) = score(claim_last_edit, impl_last_edit, churn, now);
        let Some(band) = band else {
            continue;
        };

        match band {
            DriftKind::Info => {
                diagnostics.push(diagnostic(
                    band.rule_id(),
                    Severity::Info,
                    format!("drift score band=info claim=`{cid}`"),
                    Some(cid.to_string()),
                ));
            }
            DriftKind::StaleWarning => {
                stale += 1;
                diagnostics.push(diagnostic(
                    band.rule_id(),
                    Severity::Warning,
                    format!("drift score band=stale claim=`{cid}`"),
                    Some(cid.to_string()),
                ));
            }
            DriftKind::AbandonedError => {
                abandoned += 1;
                diagnostics.push(diagnostic(
                    band.rule_id(),
                    Severity::Error,
                    format!("drift score band=abandoned claim=`{cid}`"),
                    Some(cid.to_string()),
                ));
            }
        }
    }

    Ok(DriftReport {
        version: 1,
        summary: DriftSummaryCounts {
            stale,
            abandoned,
            missing,
        },
        diagnostics,
    })
}

fn diagnostic(
    rule: &str,
    severity: Severity,
    message: String,
    subject: Option<String>,
) -> Diagnostic {
    Diagnostic {
        rule_id: rule.to_string(),
        severity,
        message,
        source: Some("drift::report".into()),
        subject,
        violated: Some(Violated {
            convention: "ADR-0024".to_string(),
        }),
        docs: None,
        fix: None,
        location: None,
        detail: None,
    }
}

/// Validate drift report JSON against `schemas/drift-report.schema.json`.
pub fn validate_drift_report_json(value: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = COMPILED.iter_errors(value).map(|e| e.to_string()).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::contract::Claim;
    use crate::trace::types::{ClaimContractKind, ClaimNode, ClaimSite};

    #[test]
    fn fixture_repo_drift_report_validates_schema() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/drift-repo/drift_fixture.git");
        let repo = repo.canonicalize().expect("fixture bare repo");

        let mut claims = BTreeMap::new();
        claims.insert(
            "drift.fixture.alpha".into(),
            ClaimNode {
                claim_id: "drift.fixture.alpha".into(),
                contract_path: PathBuf::from("CONTRACT.yaml"),
                contract_kind: ClaimContractKind::Invariant,
                claim_record: Claim {
                    id: "drift.fixture.alpha".into(),
                    text: "x".into(),
                    test_linkage: None,
                },
                impl_sites: vec![ClaimSite {
                    file: PathBuf::from("src_impl.rs"),
                    line: 1,
                    claim_id: "drift.fixture.alpha".into(),
                    kind: SiteKind::Impl,
                }],
                test_sites: vec![],
                adr_refs: vec![],
                active_exemptions: vec![],
            },
        );

        let trace = TraceGraph {
            claims,
            orphan_sites: vec![],
            diagnostics: vec![],
        };

        let now = Utc.with_ymd_and_hms(2024, 7, 15, 0, 0, 0).unwrap();
        let report = build_drift_report(&repo, &trace, now).expect("report");
        let v = serde_json::to_value(&report).unwrap();
        validate_drift_report_json(&v).expect("schema ok");
    }
}
