//! ADR-0018 diagnostic helpers for the exemption registry surface.
//!
//! The crate already defines [`crate::diagnostic::Diagnostic`] as the schema-typed
//! ADR-0018 envelope. The helpers in this module produce diagnostics with the
//! `source` field always set to `"chassis exempt"` and `subject` populated with
//! the exemption id (or another stable identifier where no entry id is available).

use crate::diagnostic::Diagnostic;
use serde_json::{json, Value};

pub(crate) const SOURCE: &str = "chassis exempt";

pub(crate) fn diag(
    rule_id: &str,
    severity: &str,
    subject: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        source: Some(SOURCE.to_string()),
        subject: Some(subject.into()),
        rule_id: rule_id.to_string(),
        severity: severity.to_string(),
        message: message.into(),
        violated: Some(json!({ "convention": "ADR-0004" })),
        docs: None,
        fix: None,
        location: None,
        detail: None,
    }
}

pub(crate) fn diag_with_detail(
    rule_id: &str,
    severity: &str,
    subject: impl Into<String>,
    message: impl Into<String>,
    detail: Value,
) -> Diagnostic {
    let mut d = diag(rule_id, severity, subject, message);
    d.detail = Some(detail);
    d
}
