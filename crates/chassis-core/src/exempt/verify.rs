//! Whole-registry integrity checks per ADR-0004.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::diagnostic::Severity;

use super::envelope::{diag, diag_with_detail};
use super::{
    entry_is_quota_active, entry_violates_global_policy, id_matches_grammar,
    is_entry_expired_audit, lifetime_days, rule_id, Codeowners, Diagnostic, ExemptionStatus,
    Registry, MAX_ACTIVE_ENTRIES, MAX_LIFETIME_DAYS,
};

pub(crate) fn verify(
    registry: &Registry,
    now: DateTime<Utc>,
    codeowners: &Codeowners,
    adr_rule_ids: Option<&[String]>,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();

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

        if !entry.has_rule_or_finding() {
            out.push(diag(
                rule_id::MISSING_RULE_OR_FINDING,
                Severity::Error,
                entry.id.clone(),
                "entry requires non-empty `rule_id`, `finding_id`, or legacy `rule`",
            ));
        }

        if matches!(entry.status, ExemptionStatus::Revoked) {
            out.push(diag(
                rule_id::REVOKED,
                Severity::Info,
                entry.id.clone(),
                format!(
                    "exemption `{}` is revoked and retained only as audit evidence",
                    entry.id
                ),
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

        if entry_violates_global_policy(entry, registry) {
            out.push(diag(
                rule_id::GLOBAL_WITHOUT_OPT_IN,
                Severity::Error,
                entry.id.clone(),
                "wildcard or repo-root path requires registry.allow_global=true and entry.allow_global=true",
            ));
        }

        let lifetime = lifetime_days(entry.created_at, entry.expires_at);
        if !(0..=MAX_LIFETIME_DAYS).contains(&lifetime) {
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

        if is_entry_expired_audit(entry, now) {
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
            if let Some(ref rid) = entry.rule_id {
                if !known.iter().any(|r| r == rid) {
                    out.push(diag_with_detail(
                        rule_id::RULE_NOT_IN_ADR,
                        Severity::Warning,
                        entry.id.clone(),
                        format!("rule_id `{rid}` does not resolve to any ADR enforces[]"),
                        json!({ "ruleId": rid }),
                    ));
                }
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
        .filter(|e| entry_is_quota_active(e, today))
        .count()
}
