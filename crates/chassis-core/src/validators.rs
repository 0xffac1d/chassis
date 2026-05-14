//! JSON-Schema-backed validator. Heavy; opt-in via `feature = "validation"`.

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
/// message, and (when the real compiler is wired in) a rendered
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

/// Validates arbitrary JSON against the canonical [`metadata/contract.schema.json`]
/// embedded in `chassis-schemas` (same source tree as the Chassis CLI).
///
/// Enable **`feature = "validation"`** on `chassis-runtime` to use this type.
pub struct CanonicalMetadataContractValidator;

impl Validator for CanonicalMetadataContractValidator {
    type Error = ValidationError;

    fn validate(&self, value: &serde_json::Value) -> Result<(), Self::Error> {
        crate::contract::validate_metadata_contract(value).map_err(|errs| {
            ValidationError {
                rule_id: "CH-RUST-METADATA-CONTRACT",
                message: "metadata contract schema validation failed",
                detail: errs.join("; "),
            }
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

    #[test]
    fn canonical_validator_accepts_repo_contract_yaml() {
        let contract_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/happy-path/rust-minimal/CONTRACT.yaml");
        let raw = std::fs::read_to_string(contract_path).expect("CONTRACT.yaml");
        let value: serde_json::Value = serde_yaml::from_str(&raw).expect("parse CONTRACT.yaml");
        CanonicalMetadataContractValidator
            .validate(&value)
            .expect("repo root CONTRACT.yaml validates");
    }
}
