use serde_json::Value;
use std::sync::LazyLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violated: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
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
