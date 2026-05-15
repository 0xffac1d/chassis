//! ADR ⇄ kernel `ruleId` binding checks for `docs/adr/*.md`.
//!
//! - Forward (`every_kernel_ch_rule_is_bound…`): kernel-emitted IDs must appear in `enforces[]`.
//! - Reverse (`every_adr_enforced_kernel_rule…`): accepted ADRs cannot claim `CH-*` rules absent
//!   from that kernel emission set (see also `rejecting_fake_frontmatter_rule`).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::diff::{
    CH_DIFF_ASSURANCE_DEMOTED, CH_DIFF_ASSURANCE_PROMOTED, CH_DIFF_CLAIM_ADDED,
    CH_DIFF_CLAIM_ID_CHANGED, CH_DIFF_CLAIM_REMOVED, CH_DIFF_CLAIM_TEXT_CHANGED,
    CH_DIFF_EDGE_CASE_PROMOTED_TO_INVARIANT, CH_DIFF_INVARIANT_DEMOTED_TO_EDGE_CASE,
    CH_DIFF_KIND_CHANGED, CH_DIFF_NAME_CHANGED, CH_DIFF_OWNER_CHANGED,
    CH_DIFF_REQUIRED_KIND_FIELD_REMOVED, CH_DIFF_STATUS_CHANGED,
    CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR, CH_DIFF_VERSION_DOWNGRADED,
    CH_DIFF_VERSION_MAJOR_WITHOUT_BREAKING, CH_DIFF_VERSION_MISSING, CH_DIFF_VERSION_NOT_BUMPED,
};
use crate::exempt::rule_id as exempt_rid;

const CH_ATTEST_SIGN_FAILED: &str = "CH-ATTEST-SIGN-FAILED";
const CH_ATTEST_VERIFY_FAILED: &str = "CH-ATTEST-VERIFY-FAILED";
const CH_ATTEST_SUBJECT_MISMATCH: &str = "CH-ATTEST-SUBJECT-MISMATCH";
const CH_ATTEST_PREDICATE_INVALID: &str = "CH-ATTEST-PREDICATE-INVALID";
const CH_ATTEST_NOT_FOUND: &str = "CH-ATTEST-NOT-FOUND";

const CH_TRACE_MALFORMED_CLAIM: &str = "CH-TRACE-MALFORMED-CLAIM";
const CH_TRACE_CLAIM_NOT_IN_CONTRACT: &str = "CH-TRACE-CLAIM-NOT-IN-CONTRACT";
const CH_TRACE_DUPLICATE_CLAIM_AT_SITE: &str = "CH-TRACE-DUPLICATE-CLAIM-AT-SITE";

const CH_DRIFT_CHURN_INFO: &str = "CH-DRIFT-CHURN-INFO";
const CH_DRIFT_CLAIM_STALE: &str = "CH-DRIFT-CLAIM-STALE";
const CH_DRIFT_CLAIM_ABANDONED: &str = "CH-DRIFT-CLAIM-ABANDONED";
const CH_DRIFT_IMPL_MISSING: &str = "CH-DRIFT-IMPL-MISSING";

fn kernel_wire_ch_rules() -> BTreeSet<String> {
    [
        "CH-RUST-METADATA-CONTRACT",
        CH_DIFF_KIND_CHANGED,
        CH_DIFF_NAME_CHANGED,
        CH_DIFF_VERSION_MISSING,
        CH_DIFF_VERSION_NOT_BUMPED,
        CH_DIFF_VERSION_DOWNGRADED,
        CH_DIFF_VERSION_MAJOR_WITHOUT_BREAKING,
        CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR,
        CH_DIFF_CLAIM_REMOVED,
        CH_DIFF_CLAIM_ID_CHANGED,
        CH_DIFF_CLAIM_TEXT_CHANGED,
        CH_DIFF_CLAIM_ADDED,
        CH_DIFF_INVARIANT_DEMOTED_TO_EDGE_CASE,
        CH_DIFF_EDGE_CASE_PROMOTED_TO_INVARIANT,
        CH_DIFF_ASSURANCE_DEMOTED,
        CH_DIFF_ASSURANCE_PROMOTED,
        CH_DIFF_STATUS_CHANGED,
        CH_DIFF_OWNER_CHANGED,
        CH_DIFF_REQUIRED_KIND_FIELD_REMOVED,
        exempt_rid::QUOTA_EXCEEDED,
        exempt_rid::LIFETIME_EXCEEDED,
        exempt_rid::EXPIRED,
        exempt_rid::MISSING_CODEOWNERS,
        exempt_rid::DUPLICATE_ID,
        exempt_rid::MALFORMED_ID,
        exempt_rid::RULE_NOT_IN_ADR,
        exempt_rid::REMOVED_BY_SWEEPER,
        exempt_rid::PATHS_EMPTY,
        exempt_rid::CODEOWNERS_PARSE_ERROR,
        exempt_rid::NOT_FOUND,
        exempt_rid::REVOKED,
        exempt_rid::LEGACY_ALIAS,
        exempt_rid::GLOBAL_WITHOUT_OPT_IN,
        exempt_rid::MISSING_RULE_OR_FINDING,
        CH_ATTEST_SIGN_FAILED,
        CH_ATTEST_VERIFY_FAILED,
        CH_ATTEST_SUBJECT_MISMATCH,
        CH_ATTEST_PREDICATE_INVALID,
        CH_ATTEST_NOT_FOUND,
        CH_TRACE_MALFORMED_CLAIM,
        CH_TRACE_CLAIM_NOT_IN_CONTRACT,
        CH_TRACE_DUPLICATE_CLAIM_AT_SITE,
        CH_DRIFT_CHURN_INFO,
        CH_DRIFT_CLAIM_STALE,
        CH_DRIFT_CLAIM_ABANDONED,
        CH_DRIFT_IMPL_MISSING,
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

#[derive(Debug, Deserialize)]
struct AdrFrontmatter {
    status: Option<String>,
    enforces: Option<Vec<Enforce>>,
}

#[derive(Debug, Deserialize)]
struct Enforce {
    rule: String,
}

fn repo_root_from_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root via CARGO_MANIFEST_DIR/../.. ")
}

fn extract_frontmatter(raw: &str) -> Option<String> {
    let body = raw.strip_prefix("---\n")?;
    let end = body.find("\n---\n")?;
    Some(body[..end].to_string())
}

fn adr_status_skips_binding(meta: &AdrFrontmatter) -> bool {
    let Some(raw) = &meta.status else {
        return false;
    };
    matches!(raw.to_ascii_lowercase().as_str(), "proposed" | "superseded")
}

fn all_adr_enforced_wire_rule_ids(root: &Path, require_ch_prefix_only: bool) -> BTreeSet<String> {
    let adr_dir = root.join("docs/adr");
    let mut ids = BTreeSet::new();
    for dir in fs::read_dir(&adr_dir).unwrap_or_else(|e| panic!("read_dir {:?}: {}", adr_dir, e)) {
        let p = dir.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {}", p.display(), e));
        let Some(fm_text) = extract_frontmatter(&text) else {
            continue;
        };
        let meta: AdrFrontmatter = serde_yaml::from_str(&fm_text)
            .unwrap_or_else(|e| panic!("yaml frontmatter {:?}: {}", p.display(), e));
        if adr_status_skips_binding(&meta) {
            continue;
        }
        if let Some(enforces) = meta.enforces {
            for row in enforces {
                let r = row.rule.trim();
                if !require_ch_prefix_only || r.starts_with("CH-") {
                    ids.insert(r.to_string());
                }
            }
        }
    }
    ids
}

/// Kernel-produced wire rule IDs (`CH-*`) excluding test-only scaffolding and PARSE-only errors.
#[test]
fn every_kernel_ch_rule_is_bound_in_an_accepted_adr() {
    let kern = kernel_wire_ch_rules();
    let root = repo_root_from_manifest();
    let adr_ids = all_adr_enforced_wire_rule_ids(&root, false);
    let missing: Vec<_> = kern.difference(&adr_ids).cloned().collect();
    assert!(
        missing.is_empty(),
        "these kernel rule IDs are missing from docs/adr enforces[] lists: {:?}\n adr had {} entries",
        missing,
        adr_ids.len()
    );
}

#[test]
fn every_adr_enforces_ch_wire_rule_exists_in_kernel_set() {
    let kern = kernel_wire_ch_rules();
    let root = repo_root_from_manifest();
    let from_adrs = all_adr_enforced_wire_rule_ids(&root, true);
    let extra: Vec<_> = from_adrs.difference(&kern).cloned().collect();
    assert!(
        extra.is_empty(),
        "these docs/adr enforces[] CH-* rules are absent from chassis-core emission set {:?}; \
         add kernel surface or loosen ADR wording",
        extra
    );
}

#[test]
fn rejecting_fake_frontmatter_rule_that_kernel_does_not_emit() {
    let kern = kernel_wire_ch_rules();
    let fm = "---\nstatus: accepted\nenforces:\n  - rule: CH-FAKE-RULE\n    description: \"should never exist\"\n";
    let meta: AdrFrontmatter = serde_yaml::from_str(fm).expect("parsable yaml");
    assert!(!adr_status_skips_binding(&meta));
    let mut one = BTreeSet::new();
    for row in meta.enforces.expect("has enforces") {
        let r = row.rule.trim().to_string();
        if r.starts_with("CH-") {
            one.insert(r);
        }
    }
    let extra: Vec<_> = one.difference(&kern).cloned().collect();
    assert_eq!(
        extra,
        vec!["CH-FAKE-RULE".to_string()],
        "fixture should surface exactly the fake binding"
    );
}
