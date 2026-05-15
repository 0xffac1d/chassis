//! Canonical diagnostic envelope per ADR-0018 / `schemas/diagnostic.schema.json`.
//!
//! Every emitter routes its findings through [`Diagnostic`]; tests in
//! `tests/diagnostic_governance.rs` assert that every emitted instance
//! round-trips through the schema and that its `ruleId`/`violated.convention`
//! resolve through [`crate::diagnostic_registry::AdrRuleRegistry`].

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

use crate::diagnostic_registry::AdrRuleRegistry;

/// Diagnostic severity. Must match `schemas/diagnostic.schema.json` severity enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Reference to the convention or ADR the diagnostic violates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violated {
    /// ADR ID or convention name (e.g. `ADR-0019`, `ADR-0008`).
    pub convention: String,
}

/// One structured diagnostic conforming to `schemas/diagnostic.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violated: Option<Violated>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Value>,
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
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

impl Diagnostic {
    /// Validate this instance against `schemas/diagnostic.schema.json`.
    ///
    /// Returns the list of schema errors. An empty `Ok(())` means the
    /// envelope satisfies ADR-0018's required fields, severity enum, and
    /// `violated.convention` grammar. Use this in tests around every emitter
    /// surface to guarantee CI fails on schema-invalid wire output.
    pub fn validate_envelope(&self) -> Result<(), Vec<String>> {
        let v =
            serde_json::to_value(self).map_err(|e| vec![format!("serialize Diagnostic: {e}")])?;
        validate_diagnostics_diagnostic(&v)
    }

    /// Verify the diagnostic is governance-bound:
    ///
    /// 1. envelope passes JSON-Schema validation, and
    /// 2. `ruleId` is enforced by some accepted ADR (per `registry`), and
    /// 3. when `violated.convention` is present, the ADR exists in the tree.
    ///
    /// Caller picks whether to additionally require `violated.convention` to
    /// match the ADR that enforces the rule — see [`Self::assert_adr_bound`].
    pub fn check_adr_bound(&self, registry: &AdrRuleRegistry) -> Result<(), Vec<String>> {
        let mut errs = self.validate_envelope().err().unwrap_or_default();
        match registry.adr_for_rule(&self.rule_id) {
            Some(_) => {}
            None => errs.push(format!(
                "ruleId `{}` does not resolve to any ADR enforces[] (ADR-0011 / ADR-0018)",
                self.rule_id
            )),
        }
        if let Some(v) = &self.violated {
            if !registry.knows_adr(&v.convention) {
                errs.push(format!(
                    "violated.convention `{}` does not match any docs/adr/*.md id",
                    v.convention
                ));
            } else if let Some(expected) = registry.adr_for_rule(&self.rule_id) {
                if expected != v.convention {
                    errs.push(format!(
                        "violated.convention `{}` does not match rule `{}`'s ADR `{}`",
                        v.convention, self.rule_id, expected
                    ));
                }
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }

    /// Panic with a rich diagnostic if [`Self::check_adr_bound`] fails. Tests
    /// use this so a regression points at the exact emitter site.
    pub fn assert_adr_bound(&self, registry: &AdrRuleRegistry) {
        if let Err(errs) = self.check_adr_bound(registry) {
            panic!(
                "diagnostic is not ADR-bound:\n  diagnostic = {:?}\n  errors = {:#?}",
                self, errs
            );
        }
    }
}
