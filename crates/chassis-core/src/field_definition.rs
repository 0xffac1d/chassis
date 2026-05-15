use serde_json::Value;
use std::sync::LazyLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "enumValues")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
}

static SCHEMA_STR: &str = include_str!("../../../schemas/field-definition.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("invalid schema JSON");
    jsonschema::validator_for(&schema).expect("invalid JSON Schema")
});

/// Validate a JSON value against `common/field-definition.schema.json`.
pub fn validate_common_field_definition(instance: &Value) -> Result<(), Vec<String>> {
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
