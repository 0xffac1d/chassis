//! Lifecycle operations: add, remove, list, sweep.

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::diagnostic::Severity;

use super::envelope::{diag, diag_with_detail};
use super::{
    entry_is_quota_active, entry_violates_global_policy, id_matches_grammar, is_expired,
    lifetime_days, rule_id, Codeowners, Diagnostic, Exemption, ExemptionStatus, ListFilter,
    Registry, MAX_ACTIVE_ENTRIES, MAX_LIFETIME_DAYS,
};

pub(crate) fn add(
    mut registry: Registry,
    entry: Exemption,
    now: DateTime<Utc>,
    codeowners: &Codeowners,
    adr_rule_ids: Option<&[String]>,
) -> Result<Registry, Vec<Diagnostic>> {
    let mut errors: Vec<Diagnostic> = Vec::new();

    if !id_matches_grammar(&entry.id) {
        errors.push(diag(
            rule_id::MALFORMED_ID,
            Severity::Error,
            entry.id.clone(),
            format!("exemption id `{}` does not match EX-YYYY-NNNN", entry.id),
        ));
    }

    if registry.entries.iter().any(|e| e.id == entry.id) {
        errors.push(diag(
            rule_id::DUPLICATE_ID,
            Severity::Error,
            entry.id.clone(),
            format!("an entry with id `{}` already exists", entry.id),
        ));
    }

    if !entry.has_rule_or_finding() {
        errors.push(diag(
            rule_id::MISSING_RULE_OR_FINDING,
            Severity::Error,
            entry.id.clone(),
            "each entry requires `rule_id`, `finding_id`, or legacy `rule`",
        ));
    }

    if entry.paths.is_empty() {
        errors.push(diag(
            rule_id::PATHS_EMPTY,
            Severity::Error,
            entry.id.clone(),
            "exemption has no paths; at least one is required (ADR-0004)",
        ));
    }

    if entry_violates_global_policy(&entry, &registry) {
        errors.push(diag(
            rule_id::GLOBAL_WITHOUT_OPT_IN,
            Severity::Error,
            entry.id.clone(),
            "wildcard or repo-root path requires registry.allow_global=true and entry.allow_global=true",
        ));
    }

    let lifetime = lifetime_days(entry.created_at, entry.expires_at);
    if !(0..=MAX_LIFETIME_DAYS).contains(&lifetime) {
        errors.push(diag_with_detail(
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

    let active_after: usize = active_count(&registry, now) + quota_weight_if_adding(&entry, now);
    if active_after > MAX_ACTIVE_ENTRIES {
        errors.push(diag_with_detail(
            rule_id::QUOTA_EXCEEDED,
            Severity::Error,
            entry.id.clone(),
            format!(
                "active exemption count would become {} (cap is {}; ADR-0004)",
                active_after, MAX_ACTIVE_ENTRIES
            ),
            json!({
                "activeCount": active_after,
                "maxActive": MAX_ACTIVE_ENTRIES,
            }),
        ));
    }

    let required = codeowners.required_owners(&entry.paths);
    let missing: Vec<String> = required
        .iter()
        .filter(|owner| !entry.codeowner_acknowledgments.contains(owner))
        .cloned()
        .collect();
    if !missing.is_empty() {
        errors.push(diag_with_detail(
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
                errors.push(diag_with_detail(
                    rule_id::RULE_NOT_IN_ADR,
                    Severity::Warning,
                    entry.id.clone(),
                    format!("rule_id `{rid}` does not resolve to any ADR enforces[]"),
                    json!({ "ruleId": rid }),
                ));
            }
        }
    }

    let has_error = errors.iter().any(|d| d.severity == Severity::Error);
    if has_error {
        return Err(errors);
    }

    registry.entries.push(entry);
    Ok(registry)
}

pub(crate) fn remove(mut registry: Registry, id: &str) -> Result<Registry, Diagnostic> {
    let before = registry.entries.len();
    registry.entries.retain(|e| e.id != id);
    if registry.entries.len() == before {
        return Err(diag(
            rule_id::NOT_FOUND,
            Severity::Error,
            id.to_string(),
            format!("no exemption with id `{}` found", id),
        ));
    }
    Ok(registry)
}

pub(crate) fn list(registry: &Registry, filter: ListFilter) -> Vec<&Exemption> {
    registry
        .entries
        .iter()
        .filter(|e| match &filter.rule_id {
            Some(r) => {
                e.rule_id.as_deref() == Some(r.as_str())
                    || e.finding_id.as_deref() == Some(r.as_str())
            }
            None => true,
        })
        .filter(|e| match &filter.path {
            Some(p) => e.paths.iter().any(|q| q == p),
            None => true,
        })
        .filter(|e| match filter.active_at {
            Some(d) => {
                e.status == ExemptionStatus::Active && e.created_at <= d && d <= e.expires_at
            }
            None => true,
        })
        .collect()
}

pub(crate) fn sweep(mut registry: Registry, now: DateTime<Utc>) -> (Registry, Vec<Diagnostic>) {
    let mut removed: Vec<Diagnostic> = Vec::new();
    let mut kept: Vec<Exemption> = Vec::with_capacity(registry.entries.len());
    for entry in registry.entries.into_iter() {
        let should_remove = match entry.status {
            ExemptionStatus::Revoked => false,
            ExemptionStatus::Expired => true,
            ExemptionStatus::Active => is_expired(entry.expires_at, now),
        };
        if should_remove {
            removed.push(diag_with_detail(
                rule_id::REMOVED_BY_SWEEPER,
                Severity::Info,
                entry.id.clone(),
                format!(
                    "removed expired exemption `{}` (expires {})",
                    entry.id, entry.expires_at
                ),
                json!({
                    "id": entry.id,
                    "ruleId": entry.primary_rule_label(),
                    "expiresAt": entry.expires_at.to_string(),
                }),
            ));
        } else {
            kept.push(entry);
        }
    }
    registry.entries = kept;
    (registry, removed)
}

fn active_count(registry: &Registry, now: DateTime<Utc>) -> usize {
    let today = now.date_naive();
    registry
        .entries
        .iter()
        .filter(|e| entry_is_quota_active(e, today))
        .count()
}

/// New entry counts toward the quota cap only when it would be quota-active immediately.
fn quota_weight_if_adding(entry: &Exemption, now: DateTime<Utc>) -> usize {
    let today = now.date_naive();
    if entry_is_quota_active(entry, today) {
        1
    } else {
        0
    }
}
