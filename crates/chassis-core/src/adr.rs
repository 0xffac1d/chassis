use serde_json::Value;
use std::sync::LazyLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Adr {
    pub id: String,
    pub title: String,
    pub status: String,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforces: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applies_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

static SCHEMA_STR: &str = include_str!("../../../schemas/adr.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("invalid schema JSON");
    jsonschema::validator_for(&schema).expect("invalid JSON Schema")
});

/// Validate a JSON value against `decision/adr.schema.json`.
pub fn validate_decision_adr(instance: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = COMPILED
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
