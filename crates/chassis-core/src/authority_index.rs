use serde_json::Value;
use std::sync::LazyLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityIndex {
    pub chassis_authority_index_version: String,
    pub entries: Vec<serde_json::Value>,
}

static SCHEMA_STR: &str = include_str!("../../../schemas/authority-index.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("invalid schema JSON");
    jsonschema::validator_for(&schema).expect("invalid JSON Schema")
});

/// Validate a JSON value against `coherence/authority-index.schema.json`.
pub fn validate_coherence_authority_index(instance: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = COMPILED
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
