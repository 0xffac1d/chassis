//! Canonical diagnostic envelope per ADR-0018 / `schemas/diagnostic.schema.json`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

/// Diagnostic severity. Must match `schemas/diagnostic.schema.json` severity enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Reference to the convention or ADR the diagnostic violates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violated {
    /// ADR ID or convention name (e.g. `ADR-0019`, `ADR-0008`).
    pub convention: String,
}

/// One structured diagnostic conforming to `schemas/diagnostic.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violated: Option<Violated>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Value>,
}

static SCHEMA_STR: &str = include_str!("../../../schemas/diagnostic.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("invalid schema JSON");
    jsonschema::validator_for(&schema).expect("invalid JSON Schema")
});

/// Validate a JSON value against `diagnostics/diagnostic.schema.json`.
pub fn validate_diagnostics_diagnostic(instance: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = COMPILED
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
