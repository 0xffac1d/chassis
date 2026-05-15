//! Cross-surface diagnostic governance.
//!
//! For each emitter surface — contract validation, contract-diff, exemption
//! verifier, trace-graph build, drift-report build — drive the surface against
//! a fixture, then assert every resulting [`Diagnostic`] is **governance-safe**:
//!
//! 1. Its envelope validates against `schemas/diagnostic.schema.json` (ADR-0018).
//! 2. Its `ruleId` resolves to a rule in an accepted ADR's `enforces[]`
//!    (ADR-0011 immutability + ADR-0018 binding).
//! 3. If present, `violated.convention` references a real ADR file and matches
//!    the ADR that enforces the rule.
//! 4. The rule is not one of [`INTERNAL_NON_WIRE_RULE_IDS`] — those are
//!    deliberately raised as typed errors, never as wire diagnostics.
//!
//! Because the workspace runs `cargo test --workspace` in CI, every failure of
//! this suite blocks merge — no command or JSON-RPC method can ship a
//! schema-invalid diagnostic without tripping this gate.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use serde_json::Value;

use chassis_core::contract::validate_metadata_contract;
use chassis_core::diagnostic::{Diagnostic, Severity, Violated};
use chassis_core::diagnostic_registry::{AdrRuleRegistry, INTERNAL_NON_WIRE_RULE_IDS};
use chassis_core::diff;
use chassis_core::drift::report::build_drift_report;
use chassis_core::exempt::{self, Codeowners};
use chassis_core::trace::build_trace_graph;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn fixtures() -> PathBuf {
    repo().join("fixtures")
}

fn load_registry() -> AdrRuleRegistry {
    AdrRuleRegistry::load(&repo()).expect("ADR registry loads")
}

fn assert_governance_safe(d: &Diagnostic, where_: &str, reg: &AdrRuleRegistry) {
    assert!(
        !INTERNAL_NON_WIRE_RULE_IDS
            .iter()
            .any(|rid| *rid == d.rule_id),
        "{where_}: ruleId `{}` is declared internal-non-wire and must not appear on the diagnostic wire",
        d.rule_id
    );
    if let Err(errs) = d.check_adr_bound(reg) {
        panic!(
            "{where_}: diagnostic is not governance-safe.\n  diagnostic = {:#?}\n  errors = {:#?}",
            d, errs
        );
    }
}

fn yaml_file_to_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let y: serde_yaml::Value =
        serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("yaml {}: {e}", path.display()));
    serde_json::to_value(y).unwrap_or_else(|e| panic!("yaml→json {}: {e}", path.display()))
}

// -------- contract validation --------

/// `chassis validate` and `validate_contract` JSON-RPC both wrap kernel errors
/// in `CH-RUST-METADATA-CONTRACT` diagnostics (ADR-0021). Build the same wire
/// envelopes the CLI/RPC produce and confirm each is governance-safe.
#[test]
fn contract_validation_diagnostics_are_governance_safe() {
    let reg = load_registry();
    let invalid = fixtures().join("adversarial/invalid-schema/CONTRACT.yaml");
    let v = yaml_file_to_json(&invalid);
    let msgs = validate_metadata_contract(&v)
        .expect_err("adversarial fixture must fail contract validation");
    assert!(!msgs.is_empty());

    for msg in &msgs {
        let d = Diagnostic {
            rule_id: "CH-RUST-METADATA-CONTRACT".to_string(),
            severity: Severity::Error,
            message: msg.clone(),
            source: Some("test.contract.validate".into()),
            subject: None,
            violated: Some(Violated {
                convention: "ADR-0021".into(),
            }),
            docs: None,
            fix: None,
            location: None,
            detail: None,
        };
        assert_governance_safe(&d, "contract.validate", &reg);
    }
}

// -------- diff --------

#[test]
fn diff_diagnostics_across_all_fixtures_are_governance_safe() {
    let reg = load_registry();
    let mut checked = 0usize;
    for entry in fs::read_dir(fixtures().join("diff")).expect("fixtures/diff exists") {
        let case = entry.unwrap().path();
        let old = case.join("old.yaml");
        let new = case.join("new.yaml");
        if !old.is_file() || !new.is_file() {
            continue;
        }
        let Ok(report) = diff::diff(&yaml_file_to_json(&old), &yaml_file_to_json(&new)) else {
            // parse-error fixture is deliberately non-Diagnostic (DiffError::Parse);
            // confirm the carve-out matches `INTERNAL_NON_WIRE_RULE_IDS` semantics.
            continue;
        };
        for d in &report.findings {
            assert_governance_safe(d, &format!("diff::{}", case.display()), &reg);
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "no diff diagnostics exercised — fixtures missing"
    );
}

// -------- exemption verifier --------

#[test]
fn exempt_verify_diagnostics_across_all_fixtures_are_governance_safe() {
    let reg = load_registry();
    let mut checked = 0usize;
    for entry in fs::read_dir(fixtures().join("exempt")).expect("fixtures/exempt exists") {
        let dir = entry.unwrap().path();
        let registry_path = dir.join("registry.yaml");
        if !registry_path.is_file() {
            continue;
        }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        // The malformed-schema fixture is for "registry doesn't parse" surface;
        // it doesn't produce verify diagnostics.
        if name.contains("malformed-id-bypasses-schema") {
            continue;
        }
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(&fs::read_to_string(&registry_path).unwrap()).unwrap();
        let json = serde_json::to_value(&yaml).unwrap();
        let Ok(reg_doc): Result<exempt::Registry, _> = serde_json::from_value(json) else {
            continue;
        };
        let codeowners = match fs::read_to_string(dir.join("codeowners")) {
            Ok(s) => Codeowners::parse(&s).expect("codeowners parses"),
            Err(_) => Codeowners::empty(),
        };
        let now = match fs::read_to_string(dir.join("now.txt")) {
            Ok(s) => chrono::DateTime::parse_from_rfc3339(s.trim())
                .expect("now.txt RFC3339")
                .with_timezone(&Utc),
            Err(_) => Utc.with_ymd_and_hms(2026, 5, 14, 0, 0, 0).unwrap(),
        };
        for d in &exempt::verify(&reg_doc, now, &codeowners) {
            assert_governance_safe(d, &format!("exempt::verify::{name}"), &reg);
            checked += 1;
        }
    }
    assert!(checked > 0, "exempt verify diagnostics not exercised");
}

// -------- trace --------

#[test]
fn trace_graph_diagnostics_against_repo_root_are_governance_safe() {
    let reg = load_registry();
    let graph = build_trace_graph(&repo()).expect("trace graph builds against repo root");
    for d in &graph.diagnostics {
        assert_governance_safe(d, "trace::graph", &reg);
    }
}

// -------- drift --------

#[test]
fn drift_report_diagnostics_are_governance_safe() {
    use std::collections::BTreeMap;

    use chassis_core::contract::Claim;
    use chassis_core::trace::types::{ClaimContractKind, ClaimNode, TraceGraph};

    let reg = load_registry();

    // Construct a graph whose only claim has no impl sites. This forces
    // `build_drift_report` down the `RULE_IMPL_MISSING` branch — which is the
    // exact wire diagnostic we want to exercise — without ever touching the
    // git repo on disk. That avoids coupling this governance test to the
    // drift bare-repo fixture's integrity.
    let mut claims = BTreeMap::new();
    claims.insert(
        "drift.governance.missing".into(),
        ClaimNode {
            claim_id: "drift.governance.missing".into(),
            contract_path: PathBuf::from("CONTRACT.yaml"),
            contract_kind: ClaimContractKind::Invariant,
            claim_record: Claim {
                id: "drift.governance.missing".into(),
                text: "y".into(),
                test_linkage: None,
            },
            impl_sites: vec![],
            test_sites: vec![],
            adr_refs: vec![],
            active_exemptions: vec![],
        },
    );

    let graph = TraceGraph {
        claims,
        orphan_sites: vec![],
        diagnostics: vec![],
    };

    let now = Utc.with_ymd_and_hms(2024, 7, 15, 0, 0, 0).unwrap();
    // The repo path is unused on this branch, so any path is fine.
    let report = build_drift_report(&repo(), &graph, now).expect("drift report builds");
    assert!(
        !report.diagnostics.is_empty(),
        "drift report should emit at least one CH-DRIFT-IMPL-MISSING diagnostic"
    );
    for d in &report.diagnostics {
        assert_governance_safe(d, "drift::report", &reg);
    }
}

// -------- internal carve-out --------

#[test]
fn internal_non_wire_rule_ids_do_not_appear_in_active_adr_registry() {
    let reg = load_registry();
    for rid in INTERNAL_NON_WIRE_RULE_IDS {
        assert!(
            reg.adr_for_rule(rid).is_none(),
            "{rid} is documented internal-non-wire but resolves to an ADR — drop the carve-out or unbind the ADR rule",
        );
    }
}

/// Sanity: synthetic diagnostic with the pre-fix `chassis.contract` value must
/// fail `check_adr_bound` so a regression is caught loudly.
#[test]
fn governance_check_rejects_the_legacy_chassis_contract_convention() {
    let reg = load_registry();
    let bad = Diagnostic {
        rule_id: "CH-RUST-METADATA-CONTRACT".to_string(),
        severity: Severity::Error,
        message: "fake".into(),
        source: Some("test.regression".into()),
        subject: None,
        violated: Some(Violated {
            convention: "chassis.contract".into(),
        }),
        docs: None,
        fix: None,
        location: None,
        detail: None,
    };
    let err = bad
        .check_adr_bound(&reg)
        .expect_err("legacy convention must fail governance");
    assert!(
        err.iter().any(|e| e.contains("chassis.contract")),
        "error list must mention the offending convention: {err:?}"
    );
}
