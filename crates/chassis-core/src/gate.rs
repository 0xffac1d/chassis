#![forbid(unsafe_code)]

//! Shared release-gate computation.
//!
//! The CLI's `chassis release-gate` and the JSON-RPC `release_gate` method
//! must produce the **same** predicate/verdict shape — otherwise an agent
//! using the JSON-RPC kernel surface and a human using the CLI would
//! disagree about whether a given repo is ready to ship. This module is the
//! one place that walks the repo, builds the trace + drift + exemption
//! state, and condenses it into a [`GateOutcome`] + [`ReleaseGatePredicate`]
//! pair that both surfaces serialize.
//!
//! Signing is out of scope here. Callers that want a signed DSSE envelope
//! pass the predicate to [`crate::attest::assemble`] + sign it themselves.

use std::path::Path;

use chrono::{DateTime, Utc};
use git2::Repository;

use crate::attest::assemble::GateOutcome;
use crate::attest::predicate::{
    CommandRun, DriftSummary, ExemptSummary, ReleaseGatePredicate, TraceSummary, Verdict,
};
use crate::diagnostic::{Diagnostic, Severity};
use crate::drift::report::{build_drift_report, validate_drift_report_json, DriftReport};
use crate::exempt::{
    apply::apply_exemptions, list, verify as exempt_verify, Codeowners, ListFilter, Registry,
};
use crate::fingerprint;
use crate::spec_index::{digest_sha256_hex, link_spec_index, validate_spec_index_value, SpecIndex};
use crate::trace::{build_trace_graph, types::TraceGraph, validate_trace_graph};

/// Stable rule ids surfaced when the gate computation itself fails. Bound to
/// the in-tree CONTRACT.yaml claim `chassis.jsonrpc-release-gate-honest`.
pub mod rule_id {
    /// The repo root is missing or unreadable — no trace graph could be built.
    pub const REPO_UNREADABLE: &str = "CH-GATE-REPO-UNREADABLE";
    /// Trace, drift, or git inspection failed below the gate.
    pub const SUBSYSTEM_FAILURE: &str = "CH-GATE-SUBSYSTEM-FAILURE";
    /// The exemption registry file existed but failed to parse.
    pub const REGISTRY_MALFORMED: &str = "CH-GATE-REGISTRY-MALFORMED";
    /// One of the artifact serializations failed its canonical schema check —
    /// trace-graph, drift-report, or release-gate predicate.
    pub const SCHEMA_INVALID: &str = "CH-GATE-SCHEMA-INVALID";
}

/// Error from [`compute`]. Each variant carries a stable rule id so the CLI
/// and JSON-RPC surfaces map gate failures onto the same diagnostic vocabulary.
#[derive(Debug)]
pub struct GateError {
    pub rule_id: &'static str,
    pub message: String,
}

impl GateError {
    fn new(rule_id: &'static str, message: impl Into<String>) -> Self {
        Self {
            rule_id,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for GateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule_id, self.message)
    }
}

impl std::error::Error for GateError {}

/// Optional Spec Kit index (`artifacts/spec-index.json`) linkage state for the gate.
#[derive(Debug, Clone)]
pub struct SpecGateState {
    pub present: bool,
    pub digest: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl SpecGateState {
    pub fn failed(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }
}

/// Resolved gate state — everything needed to derive the predicate, the
/// blocking reasons, and the human-readable verdict envelope. Owned values
/// so callers can re-use parts (e.g. a CLI prints contract validation, a
/// JSON-RPC surface returns only the predicate).
#[derive(Debug)]
pub struct GateRun {
    pub trace: TraceGraph,
    pub drift: DriftReport,
    pub spec: SpecGateState,
    pub exempt_registry: Option<Registry>,
    pub exempt_diagnostics: Vec<Diagnostic>,
    pub unsuppressed_drift: Vec<Diagnostic>,
    pub suppressed: usize,
    pub overridden: usize,
    pub audit: usize,
    pub schema_fingerprint: String,
    pub git_commit: String,
    pub fail_on_drift: bool,
    pub now: DateTime<Utc>,
}

impl GateRun {
    /// True if the trace has orphan sites, or if any contract claim lacks
    /// implementation or test sites (matches `chassis release-gate` semantics).
    pub fn trace_failed(&self) -> bool {
        !self.trace.orphan_sites.is_empty()
            || self
                .trace
                .claims
                .values()
                .any(|n| n.impl_sites.is_empty() || n.test_sites.is_empty())
    }

    /// True iff `fail_on_drift` is set AND at least one unsuppressed drift
    /// diagnostic carries `error` or `warning` severity.
    pub fn drift_failed(&self) -> bool {
        self.fail_on_drift
            && self
                .unsuppressed_drift
                .iter()
                .any(|d| matches!(d.severity, Severity::Error | Severity::Warning))
    }

    /// True iff the exemption-registry verifier emitted at least one error.
    pub fn exemption_failed(&self) -> bool {
        self.exempt_diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Count of unsuppressed drift diagnostics at error/warning severity.
    pub fn unsuppressed_blocking(&self) -> usize {
        self.unsuppressed_drift
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
            .count()
    }

    /// Active (non-revoked, non-expired) entries in the registry, as of `now`.
    pub fn exempt_active(&self) -> usize {
        match &self.exempt_registry {
            None => 0,
            Some(reg) => list(
                reg,
                ListFilter {
                    rule_id: None,
                    path: None,
                    active_at: Some(self.now.date_naive()),
                },
            )
            .len(),
        }
    }

    pub fn spec_failed(&self) -> bool {
        self.spec.failed()
    }

    /// Build the per-axis [`GateOutcome`] embedded into the signed predicate.
    /// `attestation_failed` reflects whether signing succeeded (false when the
    /// caller did not attempt to sign).
    pub fn outcome(&self, attestation_failed: bool) -> GateOutcome {
        let trace_failed = self.trace_failed();
        let drift_failed = self.drift_failed();
        let exemption_failed = self.exemption_failed();
        let spec_failed = self.spec_failed();
        let passed = !spec_failed
            && !trace_failed
            && !drift_failed
            && !exemption_failed
            && !attestation_failed;
        // Exit code precedence matches `chassis release-gate`.
        let final_exit_code = if passed {
            0
        } else if attestation_failed {
            6
        } else if spec_failed {
            2
        } else if trace_failed || drift_failed {
            5
        } else if exemption_failed {
            3
        } else {
            5
        };
        GateOutcome {
            verdict: if passed { Verdict::Pass } else { Verdict::Fail },
            fail_on_drift: self.fail_on_drift,
            trace_failed,
            drift_failed,
            exemption_failed,
            attestation_failed,
            spec_index_present: self.spec.present,
            spec_index_digest: self.spec.digest.clone(),
            spec_failed,
            spec_error_count: self.spec.error_count(),
            unsuppressed_blocking: self.unsuppressed_blocking(),
            suppressed: self.suppressed,
            severity_overridden: self.overridden,
            final_exit_code,
        }
    }

    /// Assemble the canonical release-gate predicate. Both the JSON-RPC
    /// surface and the CLI render this through `serde_json::to_value` and
    /// validate against `schemas/release-gate.schema.json`.
    pub fn predicate(
        &self,
        commands_run: Vec<CommandRun>,
        attestation_failed: bool,
    ) -> Result<ReleaseGatePredicate, GateError> {
        let outcome = self.outcome(attestation_failed);
        let today = self.now.date_naive();
        let trace_summary = TraceSummary {
            claims: self.trace.claims.len(),
            orphan_sites: self.trace.orphan_sites.len(),
        };
        let drift_summary = DriftSummary {
            stale: self.drift.summary.stale,
            abandoned: self.drift.summary.abandoned,
            missing: self.drift.summary.missing,
        };
        let exempt_summary = match &self.exempt_registry {
            None => ExemptSummary {
                active: 0,
                expired_present: 0,
            },
            Some(reg) => {
                let mut active = 0usize;
                let mut expired_present = 0usize;
                for e in &reg.entries {
                    if e.status == crate::exempt::ExemptionStatus::Revoked {
                        continue;
                    }
                    let expired =
                        e.expires_at < today || e.status == crate::exempt::ExemptionStatus::Expired;
                    if expired {
                        expired_present += 1;
                    } else if e.status == crate::exempt::ExemptionStatus::Active {
                        active += 1;
                    }
                }
                ExemptSummary {
                    active,
                    expired_present,
                }
            }
        };

        let predicate = ReleaseGatePredicate {
            schema_fingerprint: self.schema_fingerprint.clone(),
            git_commit: self.git_commit.clone(),
            built_at: self.now.to_rfc3339(),
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

        let value = serde_json::to_value(&predicate)
            .map_err(|e| GateError::new(rule_id::SCHEMA_INVALID, format!("serialize: {e}")))?;
        crate::attest::predicate::validate_release_gate_predicate(&value).map_err(|errs| {
            GateError::new(
                rule_id::SCHEMA_INVALID,
                format!("release-gate predicate failed schema: {errs:?}"),
            )
        })?;
        Ok(predicate)
    }
}

/// Walk the repo and build every input the release-gate verdict depends on.
///
/// Caller controls `fail_on_drift` so the CLI's `--fail-on-drift` flag and a
/// future JSON-RPC `params.fail_on_drift` option produce matching verdicts.
pub fn compute(repo: &Path, now: DateTime<Utc>, fail_on_drift: bool) -> Result<GateRun, GateError> {
    if !repo.is_dir() {
        return Err(GateError::new(
            rule_id::REPO_UNREADABLE,
            format!("repo root not a directory: {}", repo.display()),
        ));
    }

    let trace = build_trace_graph(repo).map_err(|e| {
        GateError::new(
            rule_id::SUBSYSTEM_FAILURE,
            format!("build trace graph: {e}"),
        )
    })?;
    validate_trace_graph(&trace).map_err(|errs| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("trace graph failed schema: {errs:?}"),
        )
    })?;

    let spec = load_spec_index_gate(repo, &trace)?;

    let drift = build_drift_report(repo, &trace, now).map_err(|e| {
        GateError::new(
            rule_id::SUBSYSTEM_FAILURE,
            format!("build drift report: {e}"),
        )
    })?;
    let drift_value = serde_json::to_value(&drift).map_err(|e| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("serialize drift report: {e}"),
        )
    })?;
    validate_drift_report_json(&drift_value).map_err(|errs| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("drift report failed schema: {errs:?}"),
        )
    })?;

    let exempt = load_and_apply_exemptions(repo, drift.diagnostics.clone(), now)?;

    let schema_fingerprint = fingerprint::compute(repo)
        .map_err(|e| GateError::new(rule_id::SUBSYSTEM_FAILURE, format!("fingerprint: {e}")))?;
    let git_commit = git_head(repo)?;

    Ok(GateRun {
        trace,
        drift,
        spec,
        exempt_registry: exempt.registry,
        exempt_diagnostics: exempt.diagnostics,
        unsuppressed_drift: exempt.unsuppressed_drift,
        suppressed: exempt.suppressed,
        overridden: exempt.overridden,
        audit: exempt.audit,
        schema_fingerprint,
        git_commit,
        fail_on_drift,
        now,
    })
}

fn load_spec_index_gate(repo: &Path, trace: &TraceGraph) -> Result<SpecGateState, GateError> {
    let p = repo.join("artifacts/spec-index.json");
    if !p.is_file() {
        return Ok(SpecGateState {
            present: false,
            digest: None,
            diagnostics: Vec::new(),
        });
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("read {}: {e}", p.display()),
        )
    })?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("{}: JSON: {e}", p.display()),
        )
    })?;
    validate_spec_index_value(&v).map_err(|errs| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("{}: {}", p.display(), errs.join("; ")),
        )
    })?;
    let spec: SpecIndex = serde_json::from_value(v).map_err(|e| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("{}: spec index shape: {e}", p.display()),
        )
    })?;
    let digest =
        digest_sha256_hex(&spec).map_err(|e| GateError::new(rule_id::SCHEMA_INVALID, e))?;
    let diagnostics = link_spec_index(&spec, repo, trace);
    Ok(SpecGateState {
        present: true,
        digest: Some(digest),
        diagnostics,
    })
}

fn git_head(repo: &Path) -> Result<String, GateError> {
    let r = Repository::open(repo)
        .map_err(|e| GateError::new(rule_id::SUBSYSTEM_FAILURE, format!("open repo: {e}")))?;
    let head = r
        .head()
        .map_err(|e| GateError::new(rule_id::SUBSYSTEM_FAILURE, format!("HEAD: {e}")))?;
    let oid = head
        .peel_to_commit()
        .map_err(|e| GateError::new(rule_id::SUBSYSTEM_FAILURE, format!("peel commit: {e}")))?;
    Ok(oid.id().to_string())
}

/// Output of [`load_and_apply_exemptions`]: the parsed registry (if any), the
/// verifier diagnostics, the counts of suppressed/overridden/audit records
/// from the apply pass, and the drift diagnostics that survived suppression.
struct ExemptionState {
    registry: Option<Registry>,
    diagnostics: Vec<Diagnostic>,
    suppressed: usize,
    overridden: usize,
    audit: usize,
    unsuppressed_drift: Vec<Diagnostic>,
}

fn load_and_apply_exemptions(
    repo: &Path,
    drift_diagnostics: Vec<Diagnostic>,
    now: DateTime<Utc>,
) -> Result<ExemptionState, GateError> {
    let p = repo.join(".chassis/exemptions.yaml");
    if !p.exists() {
        return Ok(ExemptionState {
            registry: None,
            diagnostics: Vec::new(),
            suppressed: 0,
            overridden: 0,
            audit: 0,
            unsuppressed_drift: drift_diagnostics,
        });
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| {
        GateError::new(
            rule_id::REGISTRY_MALFORMED,
            format!("read {}: {e}", p.display()),
        )
    })?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&raw)
        .map_err(|e| GateError::new(rule_id::REGISTRY_MALFORMED, format!("parse YAML: {e}")))?;
    let value = serde_json::to_value(yaml)
        .map_err(|e| GateError::new(rule_id::REGISTRY_MALFORMED, format!("to JSON: {e}")))?;
    let registry: Registry = serde_json::from_value(value)
        .map_err(|e| GateError::new(rule_id::REGISTRY_MALFORMED, format!("registry shape: {e}")))?;
    let co_path = repo.join("CODEOWNERS");
    let co_raw = std::fs::read_to_string(&co_path).unwrap_or_default();
    let codeowners = Codeowners::parse(&co_raw)
        .map_err(|e| GateError::new(rule_id::REGISTRY_MALFORMED, format!("CODEOWNERS: {e}")))?;
    let exempt_diagnostics = exempt_verify(&registry, now, &codeowners);
    let applied = apply_exemptions(drift_diagnostics, &registry, now);
    Ok(ExemptionState {
        registry: Some(registry),
        diagnostics: exempt_diagnostics,
        suppressed: applied.suppressed.len(),
        overridden: applied.overridden.len(),
        audit: applied.audit.len(),
        unsuppressed_drift: applied.unsuppressed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn compute_produces_schema_valid_predicate_for_self_repo() {
        let repo = repo_root();
        let run = compute(&repo, Utc::now(), true).expect("gate compute");
        let predicate = run.predicate(vec![], false).expect("predicate");
        let v = serde_json::to_value(&predicate).unwrap();
        crate::attest::predicate::validate_release_gate_predicate(&v)
            .expect("self-repo predicate matches schema");
    }

    #[test]
    fn compute_rejects_missing_repo() {
        let bogus = std::path::PathBuf::from("/this/path/does/not/exist/anywhere");
        let err = compute(&bogus, Utc::now(), true).expect_err("must fail for missing repo");
        assert_eq!(err.rule_id, rule_id::REPO_UNREADABLE);
    }
}
