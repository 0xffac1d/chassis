use serde_json::Value;
use std::sync::LazyLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contract {
    pub name: String,
    pub kind: String,
    pub purpose: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub since: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assurance_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_objectives: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ring: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exports: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt: Option<Vec<crate::metadata::debt_item::DebtItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<bool>,
    pub invariants: Vec<serde_json::Value>,
    pub edge_cases: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_linkage: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caveats: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depended_by: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub states: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsive: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i18n: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "rateLimit")]
    pub rate_limit: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexes: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versioning: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produces: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resilience: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "healthCheck")]
    pub health_check: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selectors: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "defaultValue")]
    pub default_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeting: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todos: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chassis: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

static SCHEMA_STR: &str = include_str!("../../../schemas/contract.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("invalid schema JSON");
    jsonschema::validator_for(&schema).expect("invalid JSON Schema")
});

/// Validate a JSON value against `metadata/contract.schema.json`.
pub fn validate_metadata_contract(instance: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = COMPILED
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
