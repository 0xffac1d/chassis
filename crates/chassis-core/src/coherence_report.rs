use serde_json::Value;
use std::sync::LazyLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryCoherenceReport {
    pub schema_version: String,
    pub generated_at: String,
    pub runtime_evidence_sources: Vec<String>,
    pub summary: serde_json::Value,
    pub trust_ladder: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_ledger: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autonomy_hints: Option<Vec<serde_json::Value>>,
    pub findings: Vec<serde_json::Value>,
}

static SCHEMA_STR: &str = include_str!("../../../schemas/coherence-report.schema.json");

static COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_STR).expect("invalid schema JSON");
    jsonschema::validator_for(&schema).expect("invalid JSON Schema")
});

/// Validate a JSON value against `coherence/repository-coherence-report.schema.json`.
pub fn validate_coherence_repository_coherence_report(instance: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = COMPILED
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
