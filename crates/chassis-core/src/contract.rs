use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// Parsed CONTRACT.yaml document (`kind` discriminates the payload).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Contract {
    #[serde(rename = "library")]
    Library(LibraryContract),
    #[serde(rename = "cli")]
    Cli(CliContract),
    #[serde(rename = "component")]
    Component(ComponentContract),
    #[serde(rename = "endpoint")]
    Endpoint(EndpointContract),
    #[serde(rename = "entity")]
    Entity(EntityContract),
    #[serde(rename = "service")]
    Service(ServiceContract),
    #[serde(rename = "event-stream")]
    EventStream(EventStreamContract),
    #[serde(rename = "feature-flag")]
    FeatureFlag(FeatureFlagContract),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractBase {
    pub name: String,
    pub purpose: String,
    pub status: String,
    pub since: String,
    pub version: String,
    pub assurance_level: String,
    pub owner: String,
    pub invariants: Vec<Claim>,
    pub edge_cases: Vec<Claim>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_objectives: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ring: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<IoDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<IoDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_linkage: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caveats: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depended_by: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_linkage: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoDescriptor {
    pub name: String,
    pub description: String,
    #[serde(rename = "schemaRef", skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub entrypoint: String,
    #[serde(rename = "argsSummary")]
    pub args_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentContract {
    #[serde(flatten)]
    pub base: ContractBase,
    #[serde(rename = "ui_taxonomy")]
    pub ui_taxonomy: String,
    pub props: Vec<Value>,
    pub events: Vec<Value>,
    pub slots: Vec<Value>,
    pub states: Value,
    pub accessibility: Value,
    pub dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsive: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub method: String,
    pub path: String,
    pub auth: Value,
    pub request: Value,
    pub response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub fields: Vec<Value>,
    pub relationships: Vec<Value>,
    pub indexes: Vec<Value>,
    pub timestamps: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versioning: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub protocol: String,
    pub endpoints: Vec<String>,
    pub consumes: Vec<String>,
    pub produces: Vec<String>,
    pub resilience: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStreamContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub source: String,
    pub payload: Value,
    pub delivery: Value,
    pub consumers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlagContract {
    #[serde(flatten)]
    pub base: ContractBase,
    #[serde(rename = "type")]
    pub flag_type: String,
    #[serde(rename = "defaultValue")]
    pub default_value: Value,
    pub targeting: Vec<Value>,
    pub metrics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
}

static SCHEMA_STR: &str = include_str!("../../../schemas/contract.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("invalid schema JSON");
    jsonschema::validator_for(&schema).expect("invalid JSON Schema")
});

/// Validate then deserialize into [`Contract`] (kind-discriminated).
pub fn validate_metadata_contract(instance: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = COMPILED
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }

    serde_json::from_value::<Contract>(instance.clone())
        .map_err(|e| vec![format!("contract deserialize after schema validation: {e}")])
        .map(drop)
}
