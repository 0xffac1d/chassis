//! Whole-registry integrity checks per ADR-0004.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::diagnostic::Severity;

use super::envelope::{diag, diag_with_detail};
use super::{
    id_matches_grammar, is_expired, lifetime_days, rule_id, Codeowners, Diagnostic, Registry,
    MAX_ACTIVE_ENTRIES, MAX_LIFETIME_DAYS,
};

pub(crate) fn verify(
    registry: &Registry,
    now: DateTime<Utc>,
    codeowners: &Codeowners,
    adr_rule_ids: Option<&[String]>,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();

    // Duplicate-id sweep first; reporting once per duplicate id avoids spam.
    let mut seen: HashSet<&str> = HashSet::new();
    let mut dup_reported: HashSet<&str> = HashSet::new();
    for entry in &registry.entries {
        if !seen.insert(entry.id.as_str()) && dup_reported.insert(entry.id.as_str()) {
            out.push(diag(
                rule_id::DUPLICATE_ID,
                Severity::Error,
                entry.id.clone(),
                format!("duplicate exemption id `{}`", entry.id),
            ));
        }
    }

    for entry in &registry.entries {
        if !id_matches_grammar(&entry.id) {
            out.push(diag(
                rule_id::MALFORMED_ID,
                Severity::Error,
                entry.id.clone(),
                format!("exemption id `{}` does not match EX-YYYY-NNNN", entry.id),
            ));
        }

        if entry.paths.is_empty() {
            out.push(diag(
                rule_id::PATHS_EMPTY,
                Severity::Error,
                entry.id.clone(),
                "exemption has no paths; at least one is required (ADR-0004)",
            ));
        }

        let lifetime = lifetime_days(entry.created_at, entry.expires_at);
        if lifetime < 0 || lifetime > MAX_LIFETIME_DAYS {
            out.push(diag_with_detail(
                rule_id::LIFETIME_EXCEEDED,
                Severity::Error,
                entry.id.clone(),
                format!(
                    "lifetime {} days exceeds {}-day maximum (ADR-0004)",
                    lifetime, MAX_LIFETIME_DAYS
                ),
                json!({
                    "lifetimeDays": lifetime,
                    "maxLifetimeDays": MAX_LIFETIME_DAYS,
                    "createdAt": entry.created_at.to_string(),
                    "expiresAt": entry.expires_at.to_string(),
                }),
            ));
        }

        if is_expired(entry.expires_at, now) {
            out.push(diag_with_detail(
                rule_id::EXPIRED,
                Severity::Error,
                entry.id.clone(),
                format!(
                    "exemption `{}` expired on {} but is still present (ADR-0004)",
                    entry.id, entry.expires_at
                ),
                json!({ "expiresAt": entry.expires_at.to_string() }),
            ));
        }

        let required = codeowners.required_owners(&entry.paths);
        let missing: Vec<String> = required
            .iter()
            .filter(|owner| !entry.codeowner_acknowledgments.contains(owner))
            .cloned()
            .collect();
        if !missing.is_empty() {
            out.push(diag_with_detail(
                rule_id::MISSING_CODEOWNERS,
                Severity::Error,
                entry.id.clone(),
                format!(
                    "missing CODEOWNERS acknowledgment(s): {}",
                    missing.join(", ")
                ),
                json!({ "missing": missing, "required": required }),
            ));
        }

        if let Some(known) = adr_rule_ids {
            if !known.iter().any(|r| r == &entry.rule_id) {
                out.push(diag_with_detail(
                    rule_id::RULE_NOT_IN_ADR,
                    Severity::Warning,
                    entry.id.clone(),
                    format!(
                        "rule_id `{}` does not resolve to any ADR enforces[]",
                        entry.rule_id
                    ),
                    json!({ "ruleId": entry.rule_id }),
                ));
            }
        }
    }

    let active = active_count(registry, now);
    if active > MAX_ACTIVE_ENTRIES {
        out.push(diag_with_detail(
            rule_id::QUOTA_EXCEEDED,
            Severity::Error,
            "registry".to_string(),
            format!(
                "active exemption count {} exceeds cap {} (ADR-0004)",
                active, MAX_ACTIVE_ENTRIES
            ),
            json!({
                "activeCount": active,
                "maxActive": MAX_ACTIVE_ENTRIES,
            }),
        ));
    }

    out
}

fn active_count(registry: &Registry, now: DateTime<Utc>) -> usize {
    let today = now.date_naive();
    registry
        .entries
        .iter()
        .filter(|e| e.created_at <= today && today <= e.expires_at)
        .count()
}
