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
pub use crate::trace::validate_trace_graph;

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
