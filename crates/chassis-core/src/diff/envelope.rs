//! ADR-0018 envelope constructors. Centralizes severity / classification
//! mapping so call sites stay declarative.

use serde_json::{json, Value};

use crate::diagnostic::{Diagnostic, Severity, Violated};

use super::{ADR_REF, SOURCE};

fn build(
    rule_id: &str,
    severity: Severity,
    classification: &str,
    subject: &str,
    message: String,
    extra_detail: Value,
) -> Diagnostic {
    let mut detail = json!({ "classification": classification });
    if let Value::Object(map) = extra_detail {
        if let Value::Object(d) = &mut detail {
            for (k, v) in map {
                d.insert(k, v);
            }
        }
    }
    Diagnostic {
        rule_id: rule_id.to_string(),
        severity,
        message,
        source: Some(SOURCE.to_string()),
        subject: Some(subject.to_string()),
        violated: Some(Violated {
            convention: ADR_REF.to_string(),
        }),
        docs: None,
        fix: None,
        location: None,
        detail: Some(detail),
    }
}

pub fn breaking(rule_id: &str, subject: &str, message: String, detail: Value) -> Diagnostic {
    build(
        rule_id,
        Severity::Error,
        "breaking",
        subject,
        message,
        detail,
    )
}

pub fn non_breaking(rule_id: &str, subject: &str, message: String, detail: Value) -> Diagnostic {
    build(
        rule_id,
        Severity::Warning,
        "non-breaking",
        subject,
        message,
        detail,
    )
}

pub fn additive(rule_id: &str, subject: &str, message: String, detail: Value) -> Diagnostic {
    build(
        rule_id,
        Severity::Info,
        "additive",
        subject,
        message,
        detail,
    )
}

/// `Severity::Warning` paired with `classification: non-breaking`. Used for
/// rules whose envelope severity is "warning" but which do not represent a
/// consumer-visible breaking change (e.g.
/// `CH-DIFF-VERSION-MAJOR-WITHOUT-BREAKING`).
pub fn warning_non_breaking(
    rule_id: &str,
    subject: &str,
    message: String,
    detail: Value,
) -> Diagnostic {
    non_breaking(rule_id, subject, message, detail)
}
