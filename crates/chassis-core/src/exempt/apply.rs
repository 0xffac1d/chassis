//! Apply the registry to a set of diagnostics — the **suppression** half of
//! ADR-0004 + ADR-0020.
//!
//! `verify` answers *is the registry well-formed?*. This module answers *given
//! a clean registry, which findings should be suppressed, which downgraded,
//! and what audit trail proves what we did?*.
//!
//! # Fail-closed semantics
//!
//! Only entries with `status: active` and `created_at <= today <= expires_at`
//! are eligible. `status: revoked`, `status: expired`, calendar-expired
//! `active` entries, and future-dated `active` entries are skipped. Wildcard
//! and repo-root path scopes require **both** registry-level and per-entry
//! `allow_global: true`; otherwise the entry is skipped (defense in depth on
//! top of CH-EXEMPT-GLOBAL-WITHOUT-OPT-IN which verify already emits).
//!
//! When both `rule_id` and `finding_id` are set on an entry, **both** must
//! match — intersection, not union. This matches the principle of least
//! surprise: an entry that names a specific finding inside a rule should not
//! suddenly suppress every finding for that rule.

use chrono::{DateTime, Utc};
use globset::{Glob, GlobMatcher};
use serde_json::json;

use crate::diagnostic::{Diagnostic, Severity};

use super::envelope::diag_with_detail;
use super::{
    entry_is_suppression_eligible, path_requires_global_allow, rule_id, Exemption, Registry,
};

/// One audited action the apply pipeline took on a finding.
#[derive(Debug, Clone)]
pub struct SuppressionRecord {
    /// The finding as it arrived (severity, message, location — verbatim).
    /// Severity is preserved here even when an override downgraded the
    /// surfaced copy, so audit evidence is never lost.
    pub original: Diagnostic,
    /// The exemption id that matched.
    pub exemption_id: String,
    /// What the apply pipeline did with this finding.
    pub action: AppliedAction,
    /// Which field carried the match. Useful for downstream filtering.
    pub matched_on: MatchedOn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedAction {
    /// Finding fully suppressed — removed from the unsuppressed bucket.
    Suppressed,
    /// Finding's surfaced severity was downgraded to `to`; it remains visible
    /// in the unsuppressed bucket and in `overridden`.
    SeverityOverride { from: Severity, to: Severity },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchedOn {
    RuleId,
    FindingId,
    Both,
}

/// Result of applying the registry to a batch of findings.
#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    /// Findings that pass through the gate, possibly with a downgraded
    /// severity from `severity_override`. Order preserved from input.
    pub unsuppressed: Vec<Diagnostic>,
    /// Findings dropped from the gate output. Each carries the matching
    /// exemption id and original severity for audit.
    pub suppressed: Vec<SuppressionRecord>,
    /// Findings whose severity was downgraded by `severity_override`. These
    /// also appear (downgraded) in `unsuppressed`; this vec is the audit-side
    /// view that keeps the *original* severity intact.
    pub overridden: Vec<SuppressionRecord>,
    /// `CH-EXEMPT-APPLIED` info diagnostics — one per action.
    pub audit: Vec<Diagnostic>,
}

/// Apply the registry to a batch of findings.
pub fn apply_exemptions(
    findings: Vec<Diagnostic>,
    registry: &Registry,
    now: DateTime<Utc>,
) -> ApplyOutcome {
    let eligible: Vec<CompiledEntry<'_>> = registry
        .entries
        .iter()
        .filter_map(|e| CompiledEntry::compile(e, registry, now))
        .collect();

    let mut out = ApplyOutcome::default();

    for finding in findings.into_iter() {
        let matched = eligible.iter().find_map(|c| c.matches(&finding));
        match matched {
            None => out.unsuppressed.push(finding),
            Some((compiled, matched_on)) => {
                let exemption_id = compiled.entry.id.clone();
                match compiled.entry.severity_override {
                    Some(to) if to != finding.severity => {
                        let from = finding.severity;
                        let mut downgraded = finding.clone();
                        downgraded.severity = to;
                        out.audit.push(applied_audit(
                            &exemption_id,
                            compiled.entry,
                            &finding,
                            Some((from, to)),
                            matched_on,
                        ));
                        let record = SuppressionRecord {
                            original: finding,
                            exemption_id,
                            action: AppliedAction::SeverityOverride { from, to },
                            matched_on,
                        };
                        out.overridden.push(record);
                        out.unsuppressed.push(downgraded);
                    }
                    Some(_) | None => {
                        out.audit.push(applied_audit(
                            &exemption_id,
                            compiled.entry,
                            &finding,
                            None,
                            matched_on,
                        ));
                        out.suppressed.push(SuppressionRecord {
                            original: finding,
                            exemption_id,
                            action: AppliedAction::Suppressed,
                            matched_on,
                        });
                    }
                }
            }
        }
    }

    out
}

struct CompiledEntry<'a> {
    entry: &'a Exemption,
    path_matchers: Vec<GlobMatcher>,
}

impl<'a> CompiledEntry<'a> {
    fn compile(entry: &'a Exemption, registry: &Registry, now: DateTime<Utc>) -> Option<Self> {
        if !entry_is_suppression_eligible(entry, now) {
            return None;
        }
        // Defense in depth: even if verify would flag CH-EXEMPT-GLOBAL-WITHOUT-OPT-IN,
        // apply refuses to consume the entry unless both opt-ins are set.
        let registry_allow_global = registry.allow_global == Some(true);
        let entry_allow_global = entry.allow_global == Some(true);
        let any_global = entry.paths.iter().any(|p| path_requires_global_allow(p));
        if any_global && !(registry_allow_global && entry_allow_global) {
            return None;
        }
        // An entry with no path-set can't reliably scope anything; verify
        // catches this as CH-EXEMPT-PATHS-EMPTY, apply skips it.
        if entry.paths.is_empty() {
            return None;
        }
        let mut path_matchers: Vec<GlobMatcher> = Vec::with_capacity(entry.paths.len());
        for raw in &entry.paths {
            let pattern = if raw == "/" {
                "**".to_string()
            } else {
                raw.clone()
            };
            match Glob::new(&pattern) {
                Ok(g) => path_matchers.push(g.compile_matcher()),
                // Unparseable globs are fail-closed: skip the entry entirely.
                Err(_) => return None,
            }
        }
        Some(Self {
            entry,
            path_matchers,
        })
    }

    fn matches(&self, finding: &Diagnostic) -> Option<(&Self, MatchedOn)> {
        let rule_field = self
            .entry
            .rule_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let finding_field = self
            .entry
            .finding_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let finding_path = finding
            .location
            .as_ref()
            .and_then(|loc| loc.get("path"))
            .and_then(|p| p.as_str());

        let path_ok = match finding_path {
            Some(p) => self.path_matchers.iter().any(|m| m.is_match(p)),
            // Fail-closed: a finding with no path cannot be matched by an
            // entry whose scope is anything other than fully-global (and
            // global already required the dual opt-in above).
            None => self
                .entry
                .paths
                .iter()
                .any(|raw| raw == "/" || raw == "**" || raw == "**/*"),
        };
        if !path_ok {
            return None;
        }

        let rule_match = rule_field.map(|r| r == finding.rule_id.as_str());
        let finding_match = finding_field.map(|f| {
            // finding_id can match either the diagnostic's `subject` or a
            // value placed in `detail.findingId` (some emitters carry it
            // there). The diagnostic envelope does not have a top-level
            // findingId, so subject is the primary handle.
            let subject_match = finding.subject.as_deref().map(|s| s == f).unwrap_or(false);
            let detail_match = finding
                .detail
                .as_ref()
                .and_then(|d| d.get("findingId"))
                .and_then(|v| v.as_str())
                .map(|s| s == f)
                .unwrap_or(false);
            subject_match || detail_match
        });

        match (rule_match, finding_match) {
            // Both fields set on the entry: intersection — both must match.
            (Some(true), Some(true)) => Some((self, MatchedOn::Both)),
            (Some(true), None) => Some((self, MatchedOn::RuleId)),
            (None, Some(true)) => Some((self, MatchedOn::FindingId)),
            _ => None,
        }
    }
}

fn applied_audit(
    exemption_id: &str,
    entry: &Exemption,
    finding: &Diagnostic,
    severity_change: Option<(Severity, Severity)>,
    matched_on: MatchedOn,
) -> Diagnostic {
    let action = if severity_change.is_some() {
        "severity-override"
    } else {
        "suppressed"
    };
    let mut detail = json!({
        "exemptionId": exemption_id,
        "action": action,
        "ruleId": finding.rule_id,
        "matchedOn": match matched_on {
            MatchedOn::RuleId => "rule_id",
            MatchedOn::FindingId => "finding_id",
            MatchedOn::Both => "both",
        },
    });
    if let Some(s) = finding.subject.as_deref() {
        detail["findingId"] = json!(s);
    }
    if let Some(loc) = finding.location.as_ref() {
        if let Some(p) = loc.get("path") {
            detail["path"] = p.clone();
        }
    }
    if let Some((from, to)) = severity_change {
        detail["severityFrom"] = json!(severity_name(from));
        detail["severityTo"] = json!(severity_name(to));
    }
    // entry context for downstream tooling
    if let Some(rid) = entry.rule_id.as_deref() {
        detail["entryRuleId"] = json!(rid);
    }
    if let Some(fid) = entry.finding_id.as_deref() {
        detail["entryFindingId"] = json!(fid);
    }
    diag_with_detail(
        rule_id::APPLIED,
        Severity::Info,
        exemption_id.to_string(),
        format!(
            "exemption `{}` {} finding `{}`",
            exemption_id, action, finding.rule_id
        ),
        detail,
    )
}

fn severity_name(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, Severity, Violated};
    use crate::exempt::{Exemption, ExemptionStatus, Registry};
    use chrono::{NaiveDate, TimeZone, Utc};
    use serde_json::json;

    fn at(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }
    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn finding(rule: &str, path: &str, severity: Severity) -> Diagnostic {
        Diagnostic {
            rule_id: rule.into(),
            severity,
            message: format!("{rule} fired"),
            source: Some("test".into()),
            subject: Some(path.into()),
            violated: Some(Violated {
                convention: "ADR-0000".into(),
            }),
            docs: None,
            fix: None,
            location: Some(json!({ "path": path })),
            detail: None,
        }
    }

    fn finding_with_subject(rule: &str, path: &str, subject: &str) -> Diagnostic {
        let mut d = finding(rule, path, Severity::Error);
        d.subject = Some(subject.into());
        d
    }

    fn base_entry(id: &str) -> Exemption {
        Exemption {
            id: id.into(),
            rule_id: Some("CH-DIFF-CLAIM-REMOVED".into()),
            finding_id: None,
            reason: "Vendored upstream parser uses unwrap() on tokenizer state; rewrite tracked."
                .into(),
            owner: "platform-team@docs.invalid".into(),
            created_at: date(2026, 4, 1),
            expires_at: date(2026, 6, 30),
            // Concrete (non-wildcard) path so `path_requires_global_allow`
            // does not gate this default fixture; wildcard cases set their own.
            paths: vec!["crates/legacy/src/a.rs".into()],
            codeowner_acknowledgments: vec![],
            linked_issue: None,
            adr: None,
            status: ExemptionStatus::Active,
            severity_override: None,
            allow_global: None,
        }
    }

    fn registry_with(entry: Exemption) -> Registry {
        let mut r = Registry::empty();
        r.entries.push(entry);
        r
    }

    // ---------- positive paths ----------

    #[test]
    fn suppresses_by_rule_id_and_path() {
        let r = registry_with(base_entry("EX-2026-0001"));
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            Severity::Error,
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(out.unsuppressed.len(), 0);
        assert_eq!(out.suppressed.len(), 1);
        assert_eq!(out.audit.len(), 1);
        assert_eq!(out.audit[0].rule_id, rule_id::APPLIED);
        assert_eq!(out.audit[0].severity, Severity::Info);
    }

    #[test]
    fn suppresses_by_finding_id_match() {
        let mut e = base_entry("EX-2026-0010");
        e.rule_id = None;
        e.finding_id = Some("FIND-CH-REMOTE-042".into());
        let r = registry_with(e);
        let f = finding_with_subject(
            "CH-DRIFT-STALE",
            "crates/legacy/src/a.rs",
            "FIND-CH-REMOTE-042",
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(out.suppressed.len(), 1);
        assert_eq!(out.suppressed[0].matched_on, MatchedOn::FindingId);
    }

    #[test]
    fn both_target_fields_require_intersection() {
        let mut e = base_entry("EX-2026-0011");
        e.rule_id = Some("CH-DIFF-CLAIM-REMOVED".into());
        e.finding_id = Some("FIND-XYZ".into());
        let r = registry_with(e);
        // Right rule_id, wrong subject — must NOT match.
        let f1 = finding_with_subject(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            "FIND-OTHER",
        );
        let out = apply_exemptions(vec![f1], &r, at(2026, 5, 1));
        assert_eq!(out.suppressed.len(), 0);
        assert_eq!(out.unsuppressed.len(), 1);
        // Both match — suppression with MatchedOn::Both.
        let f2 = finding_with_subject(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            "FIND-XYZ",
        );
        let out = apply_exemptions(vec![f2], &r, at(2026, 5, 1));
        assert_eq!(out.suppressed.len(), 1);
        assert_eq!(out.suppressed[0].matched_on, MatchedOn::Both);
    }

    // ---------- fail-closed: lifecycle states ----------

    #[test]
    fn revoked_entry_never_suppresses() {
        let mut e = base_entry("EX-2026-0002");
        e.status = ExemptionStatus::Revoked;
        let r = registry_with(e);
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            Severity::Error,
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(out.suppressed.len(), 0, "revoked must not suppress");
        assert_eq!(out.unsuppressed.len(), 1);
        assert!(out.audit.is_empty());
    }

    #[test]
    fn status_expired_entry_never_suppresses() {
        let mut e = base_entry("EX-2026-0003");
        e.status = ExemptionStatus::Expired;
        let r = registry_with(e);
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            Severity::Error,
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(out.suppressed.len(), 0, "status: expired must not suppress");
        assert_eq!(out.unsuppressed.len(), 1);
    }

    #[test]
    fn active_but_past_expires_at_never_suppresses() {
        let mut e = base_entry("EX-2026-0004");
        e.created_at = date(2026, 1, 1);
        e.expires_at = date(2026, 1, 31);
        e.status = ExemptionStatus::Active;
        let r = registry_with(e);
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            Severity::Error,
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(
            out.suppressed.len(),
            0,
            "active+past expires_at must fail closed at apply time even though verify errors"
        );
    }

    #[test]
    fn future_dated_active_entry_does_not_suppress_yet() {
        let mut e = base_entry("EX-2026-0005");
        e.created_at = date(2026, 6, 1);
        e.expires_at = date(2026, 8, 1);
        let r = registry_with(e);
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            Severity::Error,
        );
        // now is before created_at
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(out.suppressed.len(), 0);
    }

    // ---------- fail-closed: scope ----------

    #[test]
    fn wildcard_without_allow_global_fails_closed() {
        let mut e = base_entry("EX-2026-0006");
        e.paths = vec!["**".into()];
        // neither registry nor entry opt in
        let r = registry_with(e);
        let f = finding("CH-DIFF-CLAIM-REMOVED", "anywhere/x.rs", Severity::Error);
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(
            out.suppressed.len(),
            0,
            "global without opt-in must not suppress"
        );
    }

    #[test]
    fn wildcard_with_only_entry_opt_in_fails_closed() {
        let mut e = base_entry("EX-2026-0007");
        e.paths = vec!["**".into()];
        e.allow_global = Some(true); // entry yes, registry no
        let r = registry_with(e);
        let f = finding("CH-DIFF-CLAIM-REMOVED", "anywhere/x.rs", Severity::Error);
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(
            out.suppressed.len(),
            0,
            "registry allow_global also required"
        );
    }

    #[test]
    fn wildcard_with_both_opt_ins_succeeds() {
        let mut e = base_entry("EX-2026-0008");
        e.paths = vec!["**".into()];
        e.allow_global = Some(true);
        let mut r = registry_with(e);
        r.allow_global = Some(true);
        let f = finding("CH-DIFF-CLAIM-REMOVED", "anywhere/x.rs", Severity::Error);
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(out.suppressed.len(), 1);
    }

    #[test]
    fn path_outside_scope_is_not_suppressed() {
        let r = registry_with(base_entry("EX-2026-0009"));
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/other/src/a.rs",
            Severity::Error,
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        assert_eq!(out.suppressed.len(), 0);
    }

    // ---------- severity override preserves evidence ----------

    #[test]
    fn severity_override_downgrades_visibility_keeps_evidence() {
        let mut e = base_entry("EX-2026-0020");
        e.severity_override = Some(Severity::Warning);
        let r = registry_with(e);
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            Severity::Error,
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        // surfaced copy is downgraded but still visible
        assert_eq!(out.unsuppressed.len(), 1);
        assert_eq!(out.unsuppressed[0].severity, Severity::Warning);
        // override record carries the *original* severity for audit
        assert_eq!(out.overridden.len(), 1);
        match out.overridden[0].action {
            AppliedAction::SeverityOverride { from, to } => {
                assert_eq!(from, Severity::Error);
                assert_eq!(to, Severity::Warning);
            }
            _ => panic!("expected SeverityOverride"),
        }
        assert_eq!(out.overridden[0].original.severity, Severity::Error);
        // suppressed bucket is empty: override does not delete evidence
        assert_eq!(out.suppressed.len(), 0);
        // audit diagnostic recorded the change
        assert_eq!(out.audit.len(), 1);
        let detail = out.audit[0].detail.as_ref().unwrap();
        assert_eq!(detail["action"], json!("severity-override"));
        assert_eq!(detail["severityFrom"], json!("error"));
        assert_eq!(detail["severityTo"], json!("warning"));
    }

    #[test]
    fn severity_override_no_op_when_already_equal() {
        let mut e = base_entry("EX-2026-0021");
        e.severity_override = Some(Severity::Error);
        let r = registry_with(e);
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            Severity::Error,
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        // override equal to original = suppression path (no observable change)
        assert_eq!(out.suppressed.len(), 1);
        assert_eq!(out.overridden.len(), 0);
    }

    // ---------- audit shape ----------

    #[test]
    fn audit_diagnostic_carries_routable_detail() {
        let r = registry_with(base_entry("EX-2026-0030"));
        let f = finding(
            "CH-DIFF-CLAIM-REMOVED",
            "crates/legacy/src/a.rs",
            Severity::Error,
        );
        let out = apply_exemptions(vec![f], &r, at(2026, 5, 1));
        let audit = &out.audit[0];
        assert_eq!(audit.rule_id, rule_id::APPLIED);
        assert_eq!(audit.subject.as_deref(), Some("EX-2026-0030"));
        let d = audit.detail.as_ref().unwrap();
        assert_eq!(d["exemptionId"], json!("EX-2026-0030"));
        assert_eq!(d["action"], json!("suppressed"));
        assert_eq!(d["ruleId"], json!("CH-DIFF-CLAIM-REMOVED"));
        assert_eq!(d["matchedOn"], json!("rule_id"));
        assert_eq!(d["path"], json!("crates/legacy/src/a.rs"));
    }

    #[test]
    fn unrelated_finding_passes_through_untouched() {
        let r = registry_with(base_entry("EX-2026-0040"));
        let f = finding("CH-UNRELATED", "crates/legacy/src/a.rs", Severity::Error);
        let out = apply_exemptions(vec![f.clone()], &r, at(2026, 5, 1));
        assert_eq!(out.unsuppressed.len(), 1);
        assert_eq!(out.unsuppressed[0].rule_id, f.rule_id);
        assert_eq!(out.suppressed.len(), 0);
        assert!(out.audit.is_empty());
    }
}
