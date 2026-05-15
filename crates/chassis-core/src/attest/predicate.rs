#![forbid(unsafe_code)]

//! Release-gate predicate types and JSON Schema validation (`schemas/release-gate.schema.json`).

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CH_ATTEST_PREDICATE_INVALID: &str = "CH-ATTEST-PREDICATE-INVALID";

static PREDICATE_SCHEMA_STR: &str = include_str!("../../../../schemas/release-gate.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value =
        serde_json::from_str(PREDICATE_SCHEMA_STR).expect("release-gate schema JSON");
    jsonschema::validator_for(&schema).expect("compile release-gate schema")
});

/// Final verdict reported by the release-gate predicate. Mirrors the textual
/// summary the CLI prints so a verifier reading only the signed artifact can
/// reach the same conclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Fail,
}

/// Summary embedded in the attestation predicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceSummary {
    pub claims: usize,
    pub orphan_sites: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftSummary {
    pub stale: usize,
    pub abandoned: usize,
    pub missing: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExemptSummary {
    pub active: usize,
    pub expired_present: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRun {
    pub argv: Vec<String>,
    pub exit_code: i32,
}

/// JSON object matching `schemas/release-gate.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseGatePredicate {
    pub schema_fingerprint: String,
    pub git_commit: String,
    pub built_at: String,
    pub verdict: Verdict,
    pub fail_on_drift: bool,
    pub trace_failed: bool,
    pub drift_failed: bool,
    pub exemption_failed: bool,
    pub attestation_failed: bool,
    pub unsuppressed_blocking: usize,
    pub suppressed: usize,
    pub severity_overridden: usize,
    pub final_exit_code: i32,
    pub trace_summary: TraceSummary,
    pub drift_summary: DriftSummary,
    pub exempt_summary: ExemptSummary,
    pub commands_run: Vec<CommandRun>,
}

/// Validate `value` against the release-gate predicate schema (not full in-toto Statement).
pub fn validate_release_gate_predicate(value: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = COMPILED.iter_errors(value).map(|e| e.to_string()).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
