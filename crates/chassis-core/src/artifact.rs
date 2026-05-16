//! One-stop schema validation for every governance artifact Chassis emits.
//!
//! Each governance artifact ships with a canonical JSON Schema under
//! `schemas/`. This module re-exports a `validate_*_value` function for every
//! artifact so callers (CLI, JSON-RPC, self-attest script, tests) can enforce
//! schema conformance without reaching into per-module privates.
//!
//! Naming convention here is fixed: snake_case throughout the artifact
//! payloads themselves (see `schemas/trace-graph.schema.json`,
//! `schemas/drift-report.schema.json`, `schemas/release-gate.schema.json`).
//! Two artifacts are documented exceptions:
//!   - **diagnostic** (`ruleId`) — load-bearing across ADR-0001/0011/0018.
//!   - **in-toto Statement v1** (`_type`, `predicateType`) — vendored spec.
//!   - **DSSE envelope** (`payloadType`) — vendored spec.
//!   - **spec-index** (`version`, `chassis_preset_version`) — Spec Kit bridge artifact.
//!
//! Every emitter that produces one of these artifacts MUST call the matching
//! validator before printing or returning, so a serde/schema drift turns into
//! a noisy failure instead of a silent wire-format change.

#![forbid(unsafe_code)]

use serde_json::Value;

pub use crate::attest::sign::{validate_dsse_envelope, validate_dsse_envelope_json};
pub use crate::attest::{validate_release_gate_predicate, validate_statement_json};
pub use crate::diagnostic::validate_diagnostics_diagnostic;
pub use crate::drift::report::validate_drift_report_json;
pub use crate::exports::{
    validate_cedar_facts_value, validate_eventcatalog_metadata_value, validate_opa_input_value,
    validate_policy_input_value,
};
/// Validate a JSON value against `schemas/spec-index.schema.json`.
pub use crate::spec_index::validate_spec_index_value;

/// Validate a JSON value against `schemas/trace-graph.schema.json`.
///
/// Prefer [`crate::trace::validate_trace_graph`] for the typed entry point.
pub fn validate_trace_graph_value(value: &Value) -> Result<(), Vec<String>> {
    crate::trace::validate_trace_graph_json(value)
}

/// Validate a JSON value against `schemas/drift-report.schema.json`.
pub fn validate_drift_report_value(value: &Value) -> Result<(), Vec<String>> {
    validate_drift_report_json(value)
}

/// Validate a JSON value against `schemas/diagnostic.schema.json`.
pub fn validate_diagnostic_value(value: &Value) -> Result<(), Vec<String>> {
    validate_diagnostics_diagnostic(value)
}

/// Validate a JSON value against `schemas/release-gate.schema.json` (predicate only).
pub fn validate_release_gate_value(value: &Value) -> Result<(), Vec<String>> {
    validate_release_gate_predicate(value)
}

/// Validate a JSON value against `schemas/in-toto-statement-v1.schema.json`.
pub fn validate_in_toto_statement_value(value: &Value) -> Result<(), Vec<String>> {
    validate_statement_json(value)
}

/// Validate a JSON value against `schemas/dsse-envelope.schema.json`.
pub fn validate_dsse_envelope_value(value: &Value) -> Result<(), Vec<String>> {
    validate_dsse_envelope_json(value)
}

/// Validate a JSON value against `schemas/scanner-summary.schema.json`.
pub fn validate_scanner_summary_value(value: &Value) -> Result<(), Vec<String>> {
    use serde_json::Value as J;
    use std::sync::LazyLock;
    static SCHEMA_STR: &str = include_str!("../../../schemas/scanner-summary.schema.json");
    static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        let schema: J = serde_json::from_str(SCHEMA_STR).expect("scanner-summary schema");
        jsonschema::validator_for(&schema).expect("compile scanner-summary schema")
    });
    let errs: Vec<String> = COMPILED.iter_errors(value).map(|e| e.to_string()).collect();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}
