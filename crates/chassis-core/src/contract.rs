use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use jsonschema::Resource;

/// Parsed CONTRACT.yaml document (`kind` discriminates the payload).
// @claim chassis.contract-schema-kind-discriminated
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

// -- library --------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LibraryExportKind {
    Function,
    Type,
    Module,
    Macro,
    Trait,
    Constant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryExport {
    pub path: String,
    pub kind: LibraryExportKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub exports: Vec<LibraryExport>,
}

// -- cli ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliSubcommand {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub entrypoint: String,
    #[serde(rename = "argsSummary")]
    pub args_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcommands: Option<Vec<CliSubcommand>>,
}

// -- component ------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentProp {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEvent {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_schema_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSlot {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transitions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub props: Vec<ComponentProp>,
    pub events: Vec<ComponentEvent>,
    pub slots: Vec<ComponentSlot>,
    pub states: Vec<ComponentState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,
    #[serde(rename = "ui_taxonomy", skip_serializing_if = "Option::is_none")]
    pub ui_taxonomy: Option<String>,
}

// -- endpoint -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRequest {
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointResponse {
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointExample {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub method: String,
    pub path: String,
    /// Either a bare scheme string (e.g. `"bearer"`) or a structured `{type, ...}` object.
    pub auth: Value,
    pub request: EndpointRequest,
    pub response: EndpointResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_examples: Option<Vec<EndpointExample>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_examples: Option<Vec<EndpointExample>>,
}

// -- entity ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub name: String,
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityIndex {
    pub fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTimestamps {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<bool>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<bool>,
    #[serde(rename = "deletedAt", skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub fields: Vec<EntityField>,
    pub relationships: Vec<EntityRelationship>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexes: Option<Vec<EntityIndex>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<EntityTimestamps>,
}

// -- service --------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub protocol: String,
    pub endpoints: Vec<String>,
    pub consumes: Vec<String>,
    pub produces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resilience: Option<Value>,
}

// -- event-stream ---------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStreamPayload {
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStreamContract {
    #[serde(flatten)]
    pub base: ContractBase,
    pub source: String,
    pub payload: EventStreamPayload,
    pub delivery: String,
    pub consumers: Vec<String>,
}

// -- feature-flag ---------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlagCondition {
    pub attribute: String,
    pub operator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlagRule {
    pub description: String,
    pub variation: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<FeatureFlagCondition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlagTargeting {
    pub rules: Vec<FeatureFlagRule>,
    pub default_variation: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlagContract {
    #[serde(flatten)]
    pub base: ContractBase,
    #[serde(rename = "type")]
    pub flag_type: String,
    #[serde(rename = "defaultValue")]
    pub default_value: Value,
    pub targeting: FeatureFlagTargeting,
    pub metrics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
}

// -- validator ------------------------------------------------------------

static SCHEMA_STR: &str = include_str!("../../../schemas/contract.schema.json");

const SUBSCHEMAS: &[(&str, &str)] = &[
    (
        "https://chassis.dev/schemas/contract-kinds/library.schema.json",
        include_str!("../../../schemas/contract-kinds/library.schema.json"),
    ),
    (
        "https://chassis.dev/schemas/contract-kinds/cli.schema.json",
        include_str!("../../../schemas/contract-kinds/cli.schema.json"),
    ),
    (
        "https://chassis.dev/schemas/contract-kinds/component.schema.json",
        include_str!("../../../schemas/contract-kinds/component.schema.json"),
    ),
    (
        "https://chassis.dev/schemas/contract-kinds/endpoint.schema.json",
        include_str!("../../../schemas/contract-kinds/endpoint.schema.json"),
    ),
    (
        "https://chassis.dev/schemas/contract-kinds/entity.schema.json",
        include_str!("../../../schemas/contract-kinds/entity.schema.json"),
    ),
    (
        "https://chassis.dev/schemas/contract-kinds/service.schema.json",
        include_str!("../../../schemas/contract-kinds/service.schema.json"),
    ),
    (
        "https://chassis.dev/schemas/contract-kinds/event-stream.schema.json",
        include_str!("../../../schemas/contract-kinds/event-stream.schema.json"),
    ),
    (
        "https://chassis.dev/schemas/contract-kinds/feature-flag.schema.json",
        include_str!("../../../schemas/contract-kinds/feature-flag.schema.json"),
    ),
];

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("invalid schema JSON");
    let mut options = jsonschema::options();
    for (uri, raw) in SUBSCHEMAS {
        let value: Value = serde_json::from_str(raw).expect("invalid embedded subschema JSON");
        let resource = Resource::from_contents(value).expect("invalid embedded subschema");
        options = options.with_resource(*uri, resource);
    }
    options.build(&schema).expect("invalid JSON Schema")
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
