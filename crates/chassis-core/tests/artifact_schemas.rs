//! Schema-validation gate for every governance artifact Chassis emits.
//!
//! For each artifact kind: load both a `valid` and an `invalid` fixture and
//! assert the validator accepts and rejects them respectively. Then build a
//! live trace graph and drift report against `fixtures/drift-repo/...` and
//! assert that what the emitter actually produces validates — closing the
//! "schema and Rust struct silently disagree" gap that motivated this suite.
//!
//! Bound rule: any emitter regression that introduces a camelCase /
//! snake_case mismatch (or any other shape drift) trips this file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::TimeZone;
use serde_json::Value;

use chassis_core::artifact::{
    validate_diagnostic_value, validate_drift_report_value, validate_dsse_envelope_value,
    validate_in_toto_statement_value, validate_release_gate_value, validate_spec_index_value,
    validate_trace_graph_value,
};
use chassis_core::contract::Claim;
use chassis_core::drift::report::build_drift_report;
use chassis_core::trace::{
    validate_trace_graph, ClaimContractKind, ClaimNode, ClaimSite, SiteKind, TraceGraph,
};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/artifacts")
}

fn load_json(rel: &str) -> Value {
    let path = fixtures_root().join(rel);
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn assert_valid(rel: &str, validator: impl Fn(&Value) -> Result<(), Vec<String>>) {
    let v = load_json(rel);
    if let Err(errs) = validator(&v) {
        panic!("expected {rel} to validate, got: {errs:?}");
    }
}

fn assert_invalid(rel: &str, validator: impl Fn(&Value) -> Result<(), Vec<String>>) {
    let v = load_json(rel);
    let res = validator(&v);
    assert!(
        res.is_err(),
        "expected {rel} to be rejected by its schema, but it passed validation"
    );
}

// ---- trace-graph ---------------------------------------------------------

#[test]
fn fixture_valid_trace_graph_validates() {
    assert_valid("valid/trace-graph.json", validate_trace_graph_value);
}

#[test]
fn fixture_invalid_trace_graph_camelcase_rejected() {
    // Regression guard for the historical bug: trace structs serialized as
    // camelCase, but the schema requires snake_case. A camelCase payload must
    // be rejected so future serde rename_all drift cannot smuggle past CI.
    assert_invalid(
        "invalid/trace-graph-camelcase.json",
        validate_trace_graph_value,
    );
}

#[test]
fn live_built_trace_graph_serializes_to_snake_case() {
    // Build a tiny in-memory trace graph and check the *actual* JSON keys are
    // snake_case, end to end through serde, against the canonical schema.
    let mut claims = BTreeMap::new();
    claims.insert(
        "demo.alpha".to_string(),
        ClaimNode {
            claim_id: "demo.alpha".into(),
            contract_path: PathBuf::from("CONTRACT.yaml"),
            contract_kind: ClaimContractKind::Invariant,
            claim_record: Claim {
                id: "demo.alpha".into(),
                text: "x".into(),
                test_linkage: None,
            },
            impl_sites: vec![ClaimSite {
                file: PathBuf::from("src/lib.rs"),
                line: 1,
                claim_id: "demo.alpha".into(),
                kind: SiteKind::Impl,
            }],
            test_sites: vec![],
            adr_refs: vec![],
            active_exemptions: vec![],
        },
    );
    let g = TraceGraph {
        claims,
        orphan_sites: vec![],
        diagnostics: vec![],
    };
    validate_trace_graph(&g).expect("emitted trace graph must satisfy its schema");

    let v = serde_json::to_value(&g).unwrap();
    // Spot-check the bug-prone keys directly: schema requires snake_case.
    let node = &v["claims"]["demo.alpha"];
    assert!(
        node.get("claim_id").is_some(),
        "must emit claim_id, not claimId"
    );
    assert!(
        node.get("contract_kind").is_some(),
        "must emit contract_kind"
    );
    assert!(node.get("impl_sites").is_some(), "must emit impl_sites");
    assert!(
        node.get("active_exemptions").is_some(),
        "must emit active_exemptions"
    );
    assert!(
        node.get("claimId").is_none(),
        "must not emit camelCase claimId"
    );
    assert!(v.get("orphan_sites").is_some(), "must emit orphan_sites");
    assert!(
        v.get("orphanSites").is_none(),
        "must not emit camelCase orphanSites"
    );
}

// ---- drift-report --------------------------------------------------------

#[test]
fn fixture_valid_drift_report_validates() {
    assert_valid("valid/drift-report.json", validate_drift_report_value);
}

#[test]
fn fixture_invalid_drift_report_wrong_version_rejected() {
    assert_invalid(
        "invalid/drift-report-wrong-version.json",
        validate_drift_report_value,
    );
}

#[test]
fn live_built_drift_report_validates() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/drift-repo/drift_fixture.git")
        .canonicalize()
        .expect("fixture bare repo");

    let mut claims = BTreeMap::new();
    claims.insert(
        "drift.fixture.alpha".to_string(),
        ClaimNode {
            claim_id: "drift.fixture.alpha".into(),
            contract_path: PathBuf::from("CONTRACT.yaml"),
            contract_kind: ClaimContractKind::Invariant,
            claim_record: Claim {
                id: "drift.fixture.alpha".into(),
                text: "x".into(),
                test_linkage: None,
            },
            impl_sites: vec![ClaimSite {
                file: PathBuf::from("src_impl.rs"),
                line: 1,
                claim_id: "drift.fixture.alpha".into(),
                kind: SiteKind::Impl,
            }],
            test_sites: vec![],
            adr_refs: vec![],
            active_exemptions: vec![],
        },
    );
    let trace = TraceGraph {
        claims,
        orphan_sites: vec![],
        diagnostics: vec![],
    };
    let now = chrono::Utc.with_ymd_and_hms(2024, 7, 15, 0, 0, 0).unwrap();
    let report = build_drift_report(&repo, &trace, now).expect("drift report");
    let v = serde_json::to_value(&report).expect("serde");
    validate_drift_report_value(&v).expect("live drift report must satisfy its schema");
}

// ---- diagnostic ----------------------------------------------------------

#[test]
fn fixture_valid_diagnostic_validates() {
    assert_valid("valid/diagnostic.json", validate_diagnostic_value);
}

#[test]
fn fixture_invalid_diagnostic_snake_rule_id_rejected() {
    // Diagnostic envelope deliberately uses camelCase `ruleId` (ADR-0001 /
    // ADR-0011 / ADR-0018). Snake-case `rule_id` must NOT smuggle past.
    assert_invalid(
        "invalid/diagnostic-snake-rule-id.json",
        validate_diagnostic_value,
    );
}

// ---- release-gate (predicate only) --------------------------------------

#[test]
fn fixture_valid_release_gate_validates() {
    assert_valid("valid/release-gate.json", validate_release_gate_value);
}

#[test]
fn fixture_invalid_release_gate_bad_fingerprint_rejected() {
    assert_invalid(
        "invalid/release-gate-bad-fingerprint.json",
        validate_release_gate_value,
    );
}

// ---- in-toto Statement v1 -----------------------------------------------

#[test]
fn fixture_valid_in_toto_statement_validates() {
    assert_valid(
        "valid/in-toto-statement.json",
        validate_in_toto_statement_value,
    );
}

#[test]
fn fixture_invalid_in_toto_statement_wrong_predicate_type_rejected() {
    assert_invalid(
        "invalid/in-toto-statement-wrong-predicate-type.json",
        validate_in_toto_statement_value,
    );
}

// ---- DSSE envelope -------------------------------------------------------

#[test]
fn fixture_valid_dsse_envelope_validates() {
    assert_valid("valid/dsse-envelope.json", validate_dsse_envelope_value);
}

#[test]
fn fixture_invalid_dsse_envelope_snake_payload_type_rejected() {
    // DSSE specification uses camelCase `payloadType`. Snake-case must fail.
    assert_invalid(
        "invalid/dsse-envelope-snake-payload-type.json",
        validate_dsse_envelope_value,
    );
}

// ---- spec-index ----------------------------------------------------------

#[test]
fn fixture_valid_spec_index_validates() {
    assert_valid("valid/spec-index.json", validate_spec_index_value);
}

#[test]
fn fixture_invalid_spec_index_empty_acceptance_rejected() {
    assert_invalid(
        "invalid/spec-index-empty-acceptance.json",
        validate_spec_index_value,
    );
}
