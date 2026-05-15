//! Unit and fixture-driven tests for the exempt module.

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, TimeZone, Utc};

use crate::diagnostic::Severity;

use super::{
    add, codeowners::Codeowners, id_matches_grammar, list, registry_parse_str_with_diagnostics,
    remove, rule_id, sweep, verify, Diagnostic, Exemption, ExemptionStatus, ListFilter, Registry,
    MAX_LIFETIME_DAYS,
};
use crate::exemption::validate_exemption_registry;

fn fixtures_root() -> PathBuf {
    // CARGO_MANIFEST_DIR -> crates/chassis-core
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_root
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/exempt")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e))
}

fn read_optional(path: &Path) -> Option<String> {
    if path.is_file() {
        Some(read(path))
    } else {
        None
    }
}

fn load_registry(path: &Path) -> Registry {
    let yaml: serde_yaml::Value = serde_yaml::from_str(&read(path))
        .unwrap_or_else(|e| panic!("parse yaml {}: {}", path.display(), e));
    let json = serde_json::to_value(&yaml)
        .unwrap_or_else(|e| panic!("yaml->json {}: {}", path.display(), e));
    serde_json::from_value(json)
        .unwrap_or_else(|e| panic!("registry from {}: {}", path.display(), e))
}

fn registry_as_json(path: &Path) -> serde_json::Value {
    let yaml: serde_yaml::Value = serde_yaml::from_str(&read(path)).unwrap();
    serde_json::to_value(&yaml).unwrap()
}

fn load_codeowners(dir: &Path) -> Codeowners {
    match read_optional(&dir.join("codeowners")) {
        Some(s) => Codeowners::parse(&s).expect("codeowners fixture should parse"),
        None => Codeowners::empty(),
    }
}

fn fixture_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(fixtures_root())
        .unwrap_or_else(|e| panic!("read_dir fixtures/exempt: {}", e))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

fn sample_entry(id: &str) -> Exemption {
    Exemption {
        id: id.to_string(),
        rule_id: Some("CH-DIFF-CLAIM-REMOVED".to_string()),
        finding_id: None,
        reason: "Vendored upstream parser uses unwrap() on tokenizer state; rewrite tracked in CHASSIS-142.".to_string(),
        owner: "platform-team@docs.invalid".to_string(),
        created_at: date(2026, 4, 1),
        expires_at: date(2026, 6, 30),
        paths: vec!["crates/legacy-sql-driver/src/parser.rs".to_string()],
        codeowner_acknowledgments: vec![],
        linked_issue: None,
        adr: None,
        status: ExemptionStatus::Active,
        severity_override: None,
        allow_global: None,
    }
}

// ---------- ID grammar ----------

#[test]
fn exemption_id_must_match_grammar() {
    assert!(id_matches_grammar("EX-2026-0001"));
    assert!(id_matches_grammar("EX-1999-9999"));
    assert!(!id_matches_grammar("EX-2026-1"));
    assert!(!id_matches_grammar("ex-2026-0001"));
    assert!(!id_matches_grammar("EX-2026-AAAA"));
    assert!(!id_matches_grammar("EX-20260-001"));
    assert!(!id_matches_grammar("EX2026-0001"));
    assert!(!id_matches_grammar(""));
}

// ---------- Add ----------

#[test]
fn add_accepts_well_formed_entry() {
    let registry = Registry::empty();
    let entry = sample_entry("EX-2026-0001");
    let result = add(
        registry,
        entry.clone(),
        at(2026, 4, 15),
        &Codeowners::empty(),
    );
    let reg = result.expect("well-formed entry should accept");
    assert_eq!(reg.entries.len(), 1);
    assert_eq!(reg.entries[0].id, "EX-2026-0001");
}

#[test]
fn add_rejects_lifetime_over_90_days() {
    let registry = Registry::empty();
    let mut entry = sample_entry("EX-2026-0002");
    entry.created_at = date(2026, 1, 1);
    entry.expires_at = date(2026, 5, 1); // 120 days
    let errs = add(registry, entry, at(2026, 1, 2), &Codeowners::empty()).unwrap_err();
    assert!(errs.iter().any(|d| d.rule_id == rule_id::LIFETIME_EXCEEDED));
}

#[test]
fn add_rejects_quota_breach() {
    let mut registry = Registry::empty();
    for n in 1..=25 {
        let entry = sample_entry(&format!("EX-2026-{:04}", n));
        registry = add(registry, entry, at(2026, 4, 15), &Codeowners::empty())
            .expect("first 25 fit under cap");
    }
    let entry = sample_entry("EX-2026-0026");
    let errs = add(registry, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap_err();
    assert!(errs.iter().any(|d| d.rule_id == rule_id::QUOTA_EXCEEDED));
}

#[test]
fn add_rejects_missing_codeowner_signoff() {
    let codeowners =
        Codeowners::parse("crates/legacy-sql-driver/** @platform-team @security-team\n").unwrap();
    let registry = Registry::empty();
    let mut entry = sample_entry("EX-2026-0003");
    entry.codeowner_acknowledgments = vec!["@platform-team".to_string()];
    let errs = add(registry, entry, at(2026, 4, 15), &codeowners).unwrap_err();
    let missing = errs
        .iter()
        .find(|d| d.rule_id == rule_id::MISSING_CODEOWNERS)
        .expect("missing-codeowner diagnostic");
    let detail = missing.detail.as_ref().expect("detail populated");
    assert!(detail["missing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "@security-team"));
}

#[test]
fn add_rejects_duplicate_id() {
    let registry = Registry::empty();
    let entry = sample_entry("EX-2026-0001");
    let reg = add(
        registry,
        entry.clone(),
        at(2026, 4, 15),
        &Codeowners::empty(),
    )
    .unwrap();
    let errs = add(reg, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap_err();
    assert!(errs.iter().any(|d| d.rule_id == rule_id::DUPLICATE_ID));
}

#[test]
fn add_rejects_malformed_id() {
    let registry = Registry::empty();
    let mut entry = sample_entry("badly-formatted-id");
    entry.id = "badly-formatted-id".to_string();
    let errs = add(registry, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap_err();
    assert!(errs.iter().any(|d| d.rule_id == rule_id::MALFORMED_ID));
}

#[test]
fn add_rejects_empty_paths() {
    let registry = Registry::empty();
    let mut entry = sample_entry("EX-2026-0001");
    entry.paths.clear();
    let errs = add(registry, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap_err();
    assert!(errs.iter().any(|d| d.rule_id == rule_id::PATHS_EMPTY));
}

// ---------- Remove ----------

#[test]
fn remove_unknown_id_returns_diagnostic() {
    let registry = Registry::empty();
    let err = remove(registry, "EX-2026-9999").unwrap_err();
    assert_eq!(err.rule_id, rule_id::NOT_FOUND);
    assert_eq!(err.severity, Severity::Error);
}

#[test]
fn remove_known_id_drops_entry() {
    let registry = Registry::empty();
    let entry = sample_entry("EX-2026-0001");
    let reg = add(registry, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap();
    let reg = remove(reg, "EX-2026-0001").unwrap();
    assert!(reg.entries.is_empty());
}

// ---------- List ----------

#[test]
fn list_filters_by_rule_and_path() {
    let mut registry = Registry::empty();
    for (n, rid) in [(1, "CH-DIFF-CLAIM-REMOVED"), (2, "CH-DIFF-OWNER-CHANGED")] {
        let mut entry = sample_entry(&format!("EX-2026-{:04}", n));
        entry.rule_id = Some(rid.to_string());
        entry.paths = vec![format!("crates/foo/src/{}.rs", n)];
        registry = add(registry, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap();
    }
    let filtered = list(
        &registry,
        ListFilter {
            rule_id: Some("CH-DIFF-OWNER-CHANGED".to_string()),
            ..Default::default()
        },
    );
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "EX-2026-0002");
    let by_path = list(
        &registry,
        ListFilter {
            path: Some("crates/foo/src/1.rs".to_string()),
            ..Default::default()
        },
    );
    assert_eq!(by_path.len(), 1);
    assert_eq!(by_path[0].id, "EX-2026-0001");
}

// ---------- Sweep ----------

#[test]
fn sweep_idempotent_on_no_expired() {
    let mut registry = Registry::empty();
    let entry = sample_entry("EX-2026-0001");
    registry = add(registry, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap();
    let (after, diags) = sweep(registry.clone(), at(2026, 4, 16));
    assert_eq!(after, registry);
    assert!(diags.is_empty());
}

#[test]
fn sweep_removes_expired_entries() {
    let mut registry = Registry::empty();
    let mut e1 = sample_entry("EX-2026-0001");
    e1.created_at = date(2026, 1, 1);
    e1.expires_at = date(2026, 1, 31);
    registry = add(registry, e1, at(2026, 1, 2), &Codeowners::empty()).unwrap();
    let e2 = sample_entry("EX-2026-0002");
    registry = add(registry, e2, at(2026, 4, 15), &Codeowners::empty()).unwrap();
    let (after, diags) = sweep(registry, at(2026, 4, 15));
    assert_eq!(after.entries.len(), 1);
    assert_eq!(after.entries[0].id, "EX-2026-0002");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule_id, rule_id::REMOVED_BY_SWEEPER);
    assert_eq!(diags[0].severity, Severity::Info);
}

// ---------- Verify ----------

#[test]
fn verify_clean_registry_returns_empty() {
    let mut registry = Registry::empty();
    let entry = sample_entry("EX-2026-0001");
    registry = add(registry, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap();
    let diags = verify(&registry, at(2026, 4, 16), &Codeowners::empty());
    assert!(diags.is_empty(), "expected clean verify, got {:?}", diags);
}

#[test]
fn verify_flags_expired_still_present() {
    let mut registry = Registry::empty();
    let mut entry = sample_entry("EX-2026-0001");
    entry.created_at = date(2026, 1, 1);
    entry.expires_at = date(2026, 1, 31);
    // bypass add()'s quota for this constructed scenario: push directly
    registry.entries.push(entry);
    let diags = verify(&registry, at(2026, 4, 15), &Codeowners::empty());
    assert!(diags.iter().any(|d| d.rule_id == rule_id::EXPIRED));
}

// ---------- CODEOWNERS semantics ----------

#[test]
fn codeowners_last_match_wins() {
    let owners =
        Codeowners::parse("* @global-team\ndocs/** @docs-team\ndocs/api/** @api-team\n").unwrap();
    assert_eq!(owners.owners_for("README.md"), vec!["@global-team"]);
    assert_eq!(owners.owners_for("docs/guide.md"), vec!["@docs-team"]);
    assert_eq!(
        owners.owners_for("docs/api/openapi.yaml"),
        vec!["@api-team"]
    );
}

#[test]
fn codeowners_union_across_paths() {
    let owners = Codeowners::parse("crates/foo/** @foo-team\ncrates/bar/** @bar-team\n").unwrap();
    let paths = [
        "crates/foo/src/lib.rs".to_string(),
        "crates/bar/src/lib.rs".to_string(),
    ];
    let required = owners.required_owners(&paths);
    assert_eq!(required, vec!["@foo-team", "@bar-team"]);
}

#[test]
fn codeowners_parses_comments_and_blank_lines() {
    let owners = Codeowners::parse(
        "# header comment\n\n*.md   @docs-team  # inline ok\ncrates/** @platform\n",
    )
    .unwrap();
    assert_eq!(owners.owners_for("README.md"), vec!["@docs-team"]);
    assert_eq!(owners.owners_for("crates/x/src/lib.rs"), vec!["@platform"]);
}

#[test]
fn codeowners_pattern_without_owners_is_an_error() {
    let err = Codeowners::parse("docs/**\n").unwrap_err();
    assert!(err.message.contains("no owners"));
}

// ---------- Schema round-trip ----------

#[test]
fn registry_round_trips_through_schema() {
    let mut registry = Registry::empty();
    let mut entry = sample_entry("EX-2026-0001");
    entry.codeowner_acknowledgments = vec!["@platform".to_string()];
    registry.entries.push(entry);
    let as_json = serde_json::to_value(&registry).expect("serialize");
    validate_exemption_registry(&as_json)
        .unwrap_or_else(|errs| panic!("registry failed schema: {:?}", errs));
}

// ---------- Revoked / wildcard / lifecycle extensions ----------

#[test]
fn revoked_entry_does_not_consume_quota_slot() {
    let mut registry = Registry::empty();
    for n in 1..=24_u32 {
        let entry = sample_entry(&format!("EX-2026-{:04}", n));
        registry = add(registry, entry, at(2026, 4, 15), &Codeowners::empty()).unwrap();
    }
    let mut revoked = sample_entry("EX-2026-0998");
    revoked.status = ExemptionStatus::Revoked;
    registry = add(registry, revoked, at(2026, 4, 15), &Codeowners::empty())
        .expect("revoked should not occupy an active-cap slot");

    registry = add(
        registry,
        sample_entry("EX-2026-0999"),
        at(2026, 4, 15),
        &Codeowners::empty(),
    )
    .expect("25th active should fit under cap alongside revoked retention");

    let errs = add(
        registry,
        sample_entry("EX-2026-0997"),
        at(2026, 4, 15),
        &Codeowners::empty(),
    )
    .expect_err("26th simultaneous active slot must breach cap");
    assert!(
        errs.iter().any(|d| d.rule_id == rule_id::QUOTA_EXCEEDED),
        "{errs:?}"
    );
}

#[test]
fn add_accepts_finding_id_without_rule_id() {
    let registry = Registry::empty();
    let mut e = sample_entry("EX-2026-5001");
    e.rule_id = None;
    e.finding_id = Some("FIND-CH-REMOTE-042".into());
    add(registry, e, at(2026, 4, 15), &Codeowners::empty()).expect("finding_id-only exemption");
}

#[test]
fn add_rejects_wildcard_path_without_allow_global_opt_in() {
    let registry = Registry::empty();
    let mut e = sample_entry("EX-2026-5002");
    e.paths = vec!["**/generated/**".into()];
    let errs = add(registry, e, at(2026, 4, 15), &Codeowners::empty()).expect_err("global opt-in");
    assert!(
        errs.iter()
            .any(|d| d.rule_id == rule_id::GLOBAL_WITHOUT_OPT_IN),
        "{errs:?}"
    );
}

#[test]
fn legacy_aliases_emit_info_via_parse_helper() {
    let raw = r#"{
      "version": 2,
      "entries": [
        {
          "id": "EX-2026-5003",
          "reason": "Vendored upstream parser uses unwrap() on tokenizer state; rewrite tracked.",
          "owner": "team@invalid",
          "rule": "CH-DIFF-CLAIM-REMOVED",
          "scope": "crates/old/**",
          "created": "2026-04-01",
          "expires": "2026-06-01"
        }
      ]
    }"#;
    let (_reg, diags) = registry_parse_str_with_diagnostics(raw).unwrap();
    assert!(
        diags.iter().any(|d| d.rule_id == rule_id::LEGACY_ALIAS),
        "{diags:?}"
    );
}

#[test]
fn verify_flags_status_expired_as_info_not_error() {
    // status: expired is curated audit-only state — fires EXPIRED-RETAINED (info),
    // never CH-EXEMPT-EXPIRED (error). This is the per-ADR-0020 expired policy:
    // owners move entries to `expired` to resolve the error without losing audit history.
    let mut registry = Registry::empty();
    let mut entry = sample_entry("EX-2026-8888");
    entry.expires_at = date(2027, 12, 31);
    entry.status = ExemptionStatus::Expired;
    registry.entries.push(entry);
    let diags = verify(&registry, at(2026, 5, 1), &Codeowners::empty());
    assert!(
        !diags.iter().any(|d| d.rule_id == rule_id::EXPIRED),
        "status:expired must NOT fire CH-EXEMPT-EXPIRED error: {diags:?}"
    );
    let retained = diags
        .iter()
        .find(|d| d.rule_id == rule_id::EXPIRED_RETAINED)
        .expect("status:expired should fire CH-EXEMPT-EXPIRED-RETAINED info");
    assert_eq!(retained.severity, Severity::Info);
}

#[test]
fn verify_active_with_past_expires_at_is_error() {
    // The error case: status: active + calendar-expired. Never matches the
    // info rule — strictly one or the other.
    let mut registry = Registry::empty();
    let mut entry = sample_entry("EX-2026-8889");
    entry.created_at = date(2026, 1, 1);
    entry.expires_at = date(2026, 1, 31);
    entry.status = ExemptionStatus::Active;
    registry.entries.push(entry);
    let diags = verify(&registry, at(2026, 5, 1), &Codeowners::empty());
    let err = diags
        .iter()
        .find(|d| d.rule_id == rule_id::EXPIRED)
        .expect("active+past must fire EXPIRED error");
    assert_eq!(err.severity, Severity::Error);
    assert!(
        !diags.iter().any(|d| d.rule_id == rule_id::EXPIRED_RETAINED),
        "the two rules are mutually exclusive"
    );
}

#[test]
fn verify_info_for_revoked_entries() {
    let mut registry = Registry::empty();
    let mut entry = sample_entry("EX-2026-8877");
    entry.status = ExemptionStatus::Revoked;
    registry.entries.push(entry);
    let diags = verify(&registry, at(2026, 5, 1), &Codeowners::empty());
    assert!(diags.iter().any(|d| d.rule_id == rule_id::REVOKED));
}

// ---------- Constants sanity ----------

#[test]
fn max_lifetime_is_ninety_days() {
    assert_eq!(MAX_LIFETIME_DAYS, 90);
}

// ---------- Fixture-driven cases ----------

#[derive(serde::Deserialize)]
struct ExpectedDiags(Vec<ExpectedDiag>);

#[derive(serde::Deserialize)]
struct ExpectedDiag {
    #[serde(rename = "ruleId")]
    rule_id: String,
    #[serde(default)]
    severity: Option<String>,
}

#[test]
fn fixture_driven_verify_cases() {
    let mut ran = 0usize;
    for dir in fixture_dirs() {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("sweep-") || name.starts_with("add-") {
            continue;
        }
        let registry_path = dir.join("registry.yaml");
        if !registry_path.is_file() {
            continue;
        }
        let registry_json = registry_as_json(&registry_path);
        // Each non-malformed fixture must validate against schema.
        if !name.contains("malformed") {
            validate_exemption_registry(&registry_json).unwrap_or_else(|errs| {
                panic!("{} fails schema: {:?}", registry_path.display(), errs)
            });
        }
        let registry = load_registry(&registry_path);
        let codeowners = load_codeowners(&dir);
        let now = read_optional(&dir.join("now.txt"))
            .map(|s| {
                DateTime::parse_from_rfc3339(s.trim())
                    .expect("now.txt must be RFC3339")
                    .with_timezone(&Utc)
            })
            .unwrap_or_else(|| at(2026, 5, 14));
        let actual = verify(&registry, now, &codeowners);
        let expected_path = dir.join("expected-verify.json");
        let raw = read(&expected_path);
        let expected: ExpectedDiags =
            serde_json::from_str(&raw).expect("expected-verify.json shape");
        let actual_rule_ids: Vec<String> = actual.iter().map(|d| d.rule_id.clone()).collect();
        for want in expected.0 {
            assert!(
                actual_rule_ids.contains(&want.rule_id),
                "fixture {}: expected rule_id {} not found in {:?}",
                name,
                want.rule_id,
                actual_rule_ids
            );
            if let Some(sev) = want.severity {
                let d = actual
                    .iter()
                    .find(|d| d.rule_id == want.rule_id)
                    .expect("rule_id present");
                assert_eq!(
                    d.severity,
                    match sev.as_str() {
                        "error" => Severity::Error,
                        "warning" => Severity::Warning,
                        "info" => Severity::Info,
                        other => panic!("unknown severity {other}"),
                    },
                    "fixture {}: rule {} severity",
                    name,
                    want.rule_id
                );
            }
        }
        ran += 1;
    }
    assert!(
        ran > 0,
        "no verify fixtures discovered under {:?}",
        fixtures_root()
    );
}

#[test]
fn fixture_driven_sweep_cases() {
    let mut ran = 0usize;
    for dir in fixture_dirs() {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("sweep-") {
            continue;
        }
        let registry = load_registry(&dir.join("registry.yaml"));
        let now_text = read(&dir.join("now.txt"));
        let now = DateTime::parse_from_rfc3339(now_text.trim())
            .unwrap()
            .with_timezone(&Utc);
        let (after, _diags) = sweep(registry, now);
        let after_json = serde_json::to_value(&after).unwrap();
        let expected_yaml: serde_yaml::Value =
            serde_yaml::from_str(&read(&dir.join("expected-after.yaml"))).unwrap();
        let expected_json: serde_json::Value = serde_json::to_value(&expected_yaml).unwrap();
        let expected: Registry = serde_json::from_value(expected_json.clone()).unwrap();
        // Compare structurally via the typed Registry to ignore serialization noise.
        let after_typed: Registry = serde_json::from_value(after_json).unwrap();
        assert_eq!(after_typed, expected, "fixture {} sweep mismatch", name);
        ran += 1;
    }
    assert!(
        ran > 0,
        "no sweep fixtures discovered under {:?}",
        fixtures_root()
    );
}

#[test]
fn fixture_driven_add_cases() {
    let mut ran = 0usize;
    for dir in fixture_dirs() {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("add-") {
            continue;
        }
        let registry = load_registry(&dir.join("registry.yaml"));
        let codeowners = load_codeowners(&dir);
        let candidate: Exemption = {
            let yaml: serde_yaml::Value =
                serde_yaml::from_str(&read(&dir.join("candidate.yaml"))).unwrap();
            serde_json::from_value(serde_json::to_value(&yaml).unwrap()).unwrap()
        };
        let now_text = read_optional(&dir.join("now.txt"))
            .unwrap_or_else(|| "2026-05-14T00:00:00Z".to_string());
        let now = DateTime::parse_from_rfc3339(now_text.trim())
            .unwrap()
            .with_timezone(&Utc);
        let result = add(registry, candidate, now, &codeowners);
        let expected_path = dir.join("expected.json");
        let expected: serde_json::Value = serde_json::from_str(&read(&expected_path)).unwrap();
        let outcome = expected["outcome"].as_str().expect("outcome string");
        match outcome {
            "ok" => {
                let reg = result.expect("add should accept");
                if let Some(want_len) = expected.get("entries").and_then(|v| v.as_u64()) {
                    assert_eq!(
                        reg.entries.len() as u64,
                        want_len,
                        "fixture {} entries length",
                        name
                    );
                }
            }
            "error" => {
                let errs = result.unwrap_err();
                let want_rules: Vec<String> = expected["ruleIds"]
                    .as_array()
                    .expect("ruleIds array")
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                let got_rules: Vec<&str> = errs
                    .iter()
                    .map(|d: &Diagnostic| d.rule_id.as_str())
                    .collect();
                for want in want_rules {
                    assert!(
                        got_rules.iter().any(|r| *r == want),
                        "fixture {}: missing rule {} in {:?}",
                        name,
                        want,
                        got_rules
                    );
                }
            }
            other => panic!("fixture {}: unknown outcome `{}`", name, other),
        }
        ran += 1;
    }
    assert!(
        ran > 0,
        "no add fixtures discovered under {:?}",
        fixtures_root()
    );
}
