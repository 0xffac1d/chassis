#![forbid(unsafe_code)]

//! Build an in-toto Statement v1 wrapping the release-gate predicate.

use std::path::Path;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use git2::Repository;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::drift::report::DriftReport;
use crate::exempt::{ExemptionStatus, Registry};
use crate::fingerprint;
use crate::trace::types::TraceGraph;

use super::predicate::{
    validate_release_gate_predicate, CommandRun, DriftSummary, ExemptSummary, ReleaseGatePredicate,
    TraceSummary, Verdict,
};
use super::AttestError;

/// Resolved gate verdict + blocking reasons. The CLI computes these from the
/// same trace/drift/exemption inputs it uses to decide the process exit code,
/// then hands them to `assemble` so the signed predicate names the same
/// verdict, blocking reasons, and exit code the CLI prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateOutcome {
    pub verdict: Verdict,
    pub fail_on_drift: bool,
    pub trace_failed: bool,
    pub drift_failed: bool,
    pub exemption_failed: bool,
    pub attestation_failed: bool,
    pub spec_index_present: bool,
    pub spec_index_digest: Option<String>,
    pub spec_failed: bool,
    pub spec_error_count: usize,
    pub unsuppressed_blocking: usize,
    pub suppressed: usize,
    pub severity_overridden: usize,
    pub final_exit_code: i32,
}

pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
pub const PREDICATE_TYPE: &str = "https://chassis.dev/attestation/release-gate/v1";

static STATEMENT_SCHEMA_STR: &str =
    include_str!("../../../../schemas/in-toto-statement-v1.schema.json");

static STATEMENT_COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value =
        serde_json::from_str(STATEMENT_SCHEMA_STR).expect("in-toto statement schema");
    jsonschema::validator_for(&schema).expect("compile in-toto statement schema")
});

/// Full in-toto Statement (v1) with a typed predicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statement {
    #[serde(rename = "_type")]
    pub type_: String,
    pub subject: Vec<SubjectDescriptor>,
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    pub predicate: ReleaseGatePredicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectDescriptor {
    pub name: String,
    pub digest: DigestSet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigestSet {
    pub sha256: String,
}

fn git_head(repo: &Path) -> Result<String, AttestError> {
    let r = Repository::open(repo).map_err(|e| AttestError::Git(e.to_string()))?;
    let head = r.head().map_err(|e| AttestError::Git(e.to_string()))?;
    let oid = head
        .peel_to_commit()
        .map_err(|e| AttestError::Git(e.to_string()))?;
    Ok(oid.id().to_string())
}

fn summarize_exempt(reg: &Registry, today: chrono::NaiveDate) -> ExemptSummary {
    let mut active = 0usize;
    let mut expired_present = 0usize;
    for e in &reg.entries {
        if e.status == ExemptionStatus::Revoked {
            continue;
        }
        let expired = e.expires_at < today || e.status == ExemptionStatus::Expired;
        if expired {
            expired_present += 1;
        } else if e.status == ExemptionStatus::Active {
            active += 1;
        }
    }
    ExemptSummary {
        active,
        expired_present,
    }
}

/// Assemble a Statement with subject digest = `fingerprint::compute(repo)` and the v1 predicate.
///
/// `outcome` carries the verdict + per-axis blocking reasons + final exit
/// code the CLI is about to report. They are embedded verbatim so a verifier
/// reading only the signed artifact can determine why a release was blocked.
pub fn assemble(
    repo: &Path,
    trace: &TraceGraph,
    drift: &DriftReport,
    exempt: Option<&Registry>,
    commands_run: Vec<CommandRun>,
    outcome: GateOutcome,
    now: DateTime<Utc>,
) -> Result<Statement, AttestError> {
    let fp = fingerprint::compute(repo)?;
    let built_at = now.to_rfc3339();
    let git_commit = git_head(repo)?;
    let today = now.date_naive();

    let trace_summary = TraceSummary {
        claims: trace.claims.len(),
        orphan_sites: trace.orphan_sites.len(),
    };

    let drift_summary = DriftSummary {
        stale: drift.summary.stale,
        abandoned: drift.summary.abandoned,
        missing: drift.summary.missing,
    };

    let exempt_summary = match exempt {
        Some(r) => summarize_exempt(r, today),
        None => ExemptSummary {
            active: 0,
            expired_present: 0,
        },
    };

    let predicate = ReleaseGatePredicate {
        schema_fingerprint: fp.clone(),
        git_commit,
        built_at,
        verdict: outcome.verdict,
        fail_on_drift: outcome.fail_on_drift,
        trace_failed: outcome.trace_failed,
        drift_failed: outcome.drift_failed,
        exemption_failed: outcome.exemption_failed,
        attestation_failed: outcome.attestation_failed,
        spec_index_present: outcome.spec_index_present,
        spec_index_digest: outcome.spec_index_digest.clone(),
        spec_failed: outcome.spec_failed,
        spec_error_count: outcome.spec_error_count,
        unsuppressed_blocking: outcome.unsuppressed_blocking,
        suppressed: outcome.suppressed,
        severity_overridden: outcome.severity_overridden,
        final_exit_code: outcome.final_exit_code,
        trace_summary,
        drift_summary,
        exempt_summary,
        commands_run,
    };

    let pred_val =
        serde_json::to_value(&predicate).map_err(|e| AttestError::Json(e.to_string()))?;
    validate_release_gate_predicate(&pred_val).map_err(AttestError::PredicateSchema)?;

    let stmt = Statement {
        type_: STATEMENT_TYPE.to_string(),
        subject: vec![SubjectDescriptor {
            name: "chassis-schemas-manifest".to_string(),
            digest: DigestSet { sha256: fp },
        }],
        predicate_type: PREDICATE_TYPE.to_string(),
        predicate,
    };

    let stmt_val = serde_json::to_value(&stmt).map_err(|e| AttestError::Json(e.to_string()))?;
    let errors: Vec<String> = STATEMENT_COMPILED
        .iter_errors(&stmt_val)
        .map(|e| e.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(AttestError::StatementSchema(errors));
    }

    Ok(stmt)
}

/// Validate a JSON value against the vendored in-toto Statement schema (subset).
pub fn validate_statement_json(value: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = STATEMENT_COMPILED
        .iter_errors(value)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::drift::report::{DriftReport, DriftSummaryCounts};
    use crate::trace::types::TraceGraph;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn assembles_statement_that_validates() {
        let repo = repo_root();
        let trace = TraceGraph {
            claims: Default::default(),
            orphan_sites: vec![],
            diagnostics: vec![],
        };
        let drift = DriftReport {
            version: 1,
            summary: DriftSummaryCounts {
                stale: 0,
                abandoned: 0,
                missing: 0,
            },
            diagnostics: vec![],
        };
        let outcome = GateOutcome {
            verdict: Verdict::Pass,
            fail_on_drift: true,
            trace_failed: false,
            drift_failed: false,
            exemption_failed: false,
            attestation_failed: false,
            spec_index_present: false,
            spec_index_digest: None,
            spec_failed: false,
            spec_error_count: 0,
            unsuppressed_blocking: 0,
            suppressed: 0,
            severity_overridden: 0,
            final_exit_code: 0,
        };
        let stmt =
            assemble(&repo, &trace, &drift, None, vec![], outcome, Utc::now()).expect("assemble");
        let v = serde_json::to_value(&stmt).unwrap();
        validate_statement_json(&v).expect("statement validates");
    }
}
