//! JSON-Schema-backed validators over the canonical chassis schemas.

#![forbid(unsafe_code)]

pub type RuleId = &'static str;

/// Structural validator over a JSON-shaped value.
pub trait Validator {
    /// Error returned on validation failure.
    type Error;

    /// Validate `value`, returning `Ok(())` on success.
    fn validate(&self, value: &serde_json::Value) -> Result<(), Self::Error>;
}

/// Validation error. Carries the offending rule id, a static summary
/// message, and a rendered
/// `jsonschema` error trail for diagnostics.
#[derive(Debug)]
pub struct ValidationError {
    /// Rule id that owns the schema.
    pub rule_id: RuleId,
    /// Static human-readable summary.
    pub message: &'static str,
    /// First concrete validation failure rendered by `jsonschema`. Empty
    /// when construction failed before validation could run.
    pub detail: String,
}

impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.detail.is_empty() {
            write!(f, "[{}] {}", self.rule_id, self.message)
        } else {
            write!(f, "[{}] {}: {}", self.rule_id, self.message, self.detail)
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validator bound to a statically embedded schema.
///
/// The schema is parsed and compiled once at construction; subsequent
/// calls to [`StaticValidator::validate`] are allocation-free on the
/// happy path.
pub struct StaticValidator {
    rule_id: RuleId,
    compiled: jsonschema::Validator,
}

/// Validates arbitrary JSON against the canonical `contract.schema.json`
/// embedded in `chassis-core` at build time via `include_str!`.
pub struct CanonicalMetadataContractValidator;

impl Validator for CanonicalMetadataContractValidator {
    type Error = ValidationError;

    fn validate(&self, value: &serde_json::Value) -> Result<(), Self::Error> {
        crate::contract::validate_metadata_contract(value).map_err(|errs| ValidationError {
            rule_id: "CH-RUST-METADATA-CONTRACT",
            message: "metadata contract schema validation failed",
            detail: errs.join("; "),
        })
    }
}

impl StaticValidator {
    /// Construct a validator from an embedded schema string.
    ///
    /// # Panics
    ///
    /// Panics if `schema` is not valid JSON or does not compile as a
    /// JSON Schema. The input is always developer-controlled and
    /// embedded at compile time via `include_str!`, so this is the
    /// correct fail-fast point; a malformed schema is a build-time bug.
    #[must_use]
    pub fn from_embedded(schema: &'static str, rule_id: RuleId) -> Self {
        let parsed: serde_json::Value = serde_json::from_str(schema)
            .unwrap_or_else(|e| panic!("[{rule_id}] embedded schema is not valid JSON: {e}"));
        let compiled = jsonschema::validator_for(&parsed)
            .unwrap_or_else(|e| panic!("[{rule_id}] embedded schema failed to compile: {e}"));
        Self { rule_id, compiled }
    }

    /// Validate `value` against the embedded schema.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError`] on the first schema violation.
    pub fn validate(&self, value: &serde_json::Value) -> Result<(), ValidationError> {
        match self.compiled.validate(value) {
            Ok(()) => Ok(()),
            Err(err) => Err(ValidationError {
                rule_id: self.rule_id,
                message: "schema validation failed",
                detail: err.to_string(),
            }),
        }
    }

    /// Rule id this validator was constructed for.
    #[must_use]
    pub fn rule_id(&self) -> RuleId {
        self.rule_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const OBJ_SCHEMA: &str = r#"{
        "type": "object",
        "properties": {"a": {"type": "string"}},
        "required": ["a"]
    }"#;

    #[test]
    fn valid_input_passes() {
        let v = StaticValidator::from_embedded(OBJ_SCHEMA, "CH-RULE-VAL");
        let input = serde_json::json!({"a": "hello"});
        assert!(v.validate(&input).is_ok());
        assert_eq!(v.rule_id(), "CH-RULE-VAL");
    }

    #[test]
    fn invalid_input_produces_error_with_detail() {
        let v = StaticValidator::from_embedded(OBJ_SCHEMA, "CH-RULE-VAL");
        let input = serde_json::json!({"a": 42});
        let err = v.validate(&input).expect_err("non-string `a` must fail");
        assert_eq!(err.rule_id, "CH-RULE-VAL");
        assert!(!err.detail.is_empty(), "detail must be populated");
        assert!(err.to_string().contains("CH-RULE-VAL"));
    }

    #[test]
    #[should_panic(expected = "not valid JSON")]
    fn malformed_embedded_schema_panics() {
        let _ = StaticValidator::from_embedded("{not json", "CH-RULE-VAL");
    }

    fn validate_kind_fixture(dir: &str) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/happy-path")
            .join(dir)
            .join("CONTRACT.yaml");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        let value: serde_json::Value =
            serde_yaml::from_str(&raw).expect("parse fixture CONTRACT.yaml");
        CanonicalMetadataContractValidator
            .validate(&value)
            .unwrap_or_else(|e| panic!("{dir} fixture failed validation: {e}"));
    }

    #[test]
    fn canonical_validator_accepts_repo_contract_yaml() {
        let contract_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/happy-path/rust-minimal/CONTRACT.yaml");
        let raw = std::fs::read_to_string(contract_path).expect("CONTRACT.yaml");
        let value: serde_json::Value = serde_yaml::from_str(&raw).expect("parse CONTRACT.yaml");
        CanonicalMetadataContractValidator
            .validate(&value)
            .expect("repo root CONTRACT.yaml validates");
    }

    #[test]
    fn validates_library_fixture() {
        validate_kind_fixture("rust-minimal");
        validate_kind_fixture("typescript-vite");
    }

    #[test]
    fn validates_cli_fixture() {
        validate_kind_fixture("cli-minimal");
    }

    #[test]
    fn validates_component_fixture() {
        validate_kind_fixture("component-minimal");
    }

    #[test]
    fn validates_endpoint_fixture() {
        validate_kind_fixture("endpoint-minimal");
    }

    #[test]
    fn validates_entity_fixture() {
        validate_kind_fixture("entity-minimal");
    }

    #[test]
    fn validates_service_fixture() {
        validate_kind_fixture("service-minimal");
    }

    #[test]
    fn validates_event_stream_fixture() {
        validate_kind_fixture("event-stream-minimal");
    }

    #[test]
    fn validates_feature_flag_fixture() {
        validate_kind_fixture("feature-flag-minimal");
    }

    // @claim chassis.adversarial-fixture-rejected
    #[test]
    fn adversarial_invalid_schema_fixture_fails_validation() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/adversarial/invalid-schema/CONTRACT.yaml");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        let value: serde_json::Value =
            serde_yaml::from_str(&raw).expect("parse invalid-schema CONTRACT.yaml");
        let err = CanonicalMetadataContractValidator
            .validate(&value)
            .expect_err("adversarial fixture must fail canonical validation");
        let detail = err.detail.as_str();
        assert!(
            detail.contains("required") || detail.contains("enum"),
            "expected 'required' or 'enum' in error detail, got: {detail}"
        );
        assert_eq!(err.rule_id, "CH-RUST-METADATA-CONTRACT");
    }

    #[test]
    fn adversarial_cli_missing_kind_required_field_fails() {
        // CLI contract missing the kind-specific `entrypoint` field.
        let yaml = r#"
name: "cli-broken"
kind: cli
version: "0.1.0"
purpose: "Adversarial CLI fixture missing the kind-specific entrypoint required field."
status: stable
since: "0.1.0"
assurance_level: declared
owner: chassis-fixtures
argsSummary: "broken [--help]"
invariants:
  - id: cli-broken.some
    text: "Has at least one invariant."
edge_cases: []
"#;
        let value: serde_json::Value = serde_yaml::from_str(yaml).expect("parse inline yaml");
        let err = CanonicalMetadataContractValidator
            .validate(&value)
            .expect_err("cli missing entrypoint must fail");
        assert!(
            err.detail.contains("entrypoint"),
            "expected 'entrypoint' in error detail, got: {}",
            err.detail
        );
    }

    #[test]
    fn adversarial_entity_invalid_relationship_kind_fails() {
        // Entity contract with a relationship kind outside the allowed enum.
        let yaml = r#"
name: "entity-broken"
kind: entity
version: "0.1.0"
purpose: "Adversarial entity fixture using an invalid relationship kind value."
status: stable
since: "0.1.0"
assurance_level: declared
owner: chassis-fixtures
fields:
  - name: "id"
    type: "uuid"
relationships:
  - name: "primary"
    kind: "has_many_through"
    target: "Other"
invariants:
  - id: entity-broken.some
    text: "Has at least one invariant."
edge_cases: []
"#;
        let value: serde_json::Value = serde_yaml::from_str(yaml).expect("parse inline yaml");
        let err = CanonicalMetadataContractValidator
            .validate(&value)
            .expect_err("invalid relationship kind must fail");
        assert!(
            err.detail.contains("enum") || err.detail.contains("has_many_through"),
            "expected enum violation in error detail, got: {}",
            err.detail
        );
    }
}
