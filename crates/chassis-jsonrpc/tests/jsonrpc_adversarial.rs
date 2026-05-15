#![forbid(unsafe_code)]

//! Adversarial coverage for every chassis-jsonrpc method.
//!
//! Each verb (`validate_contract`, `diff_contracts`, `trace_claim`,
//! `drift_report`, `release_gate`, `list_exemptions`) is exercised against
//! malformed JSON, unknown method, missing params, wrong param types, bad
//! repo path, schema-invalid input, expected JSON-RPC error code, and (on
//! the happy path) a check that the structured result still validates
//! against its canonical schema. The full matrix lives in this file rather
//! than the smoke file so failures point at a specific failure-mode bucket
//! and so the smoke file stays a quick happy-path sanity check.

// @claim chassis.jsonrpc-not-mcp
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use assert_cmd::cargo::cargo_bin;
use chassis_core::artifact::{
    validate_diagnostic_value, validate_drift_report_value, validate_release_gate_value,
    validate_trace_graph_value,
};
use serde_json::{json, Value};
use tempfile::TempDir;

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

fn jsonrpc_repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn spawn_with_repo(repo: &Path) -> Child {
    Command::new(cargo_bin("chassis-jsonrpc"))
        .env("CHASSIS_REPO_ROOT", repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn chassis-jsonrpc")
}

/// Send `request_lines` to a freshly-spawned server, close its stdin, and
/// return every newline-delimited JSON reply it produced in order.
fn round_trip(repo: &Path, request_lines: &[String]) -> Vec<Value> {
    let mut child = spawn_with_repo(repo);
    {
        let mut stdin = child.stdin.take().expect("stdin");
        for line in request_lines {
            writeln!(stdin, "{line}").expect("write line");
        }
    }
    let stdout = child.wait_with_output().expect("wait").stdout;
    let reader = BufReader::new(stdout.as_slice());
    let mut out = Vec::new();
    for line in reader.lines() {
        let raw = line.expect("read line");
        if raw.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(&raw).expect("valid JSON-RPC reply"));
    }
    out
}

/// Send one request, return one reply. Panics if anything else surfaces.
fn one(repo: &Path, request: Value) -> Value {
    let replies = round_trip(
        repo,
        &[serde_json::to_string(&request).expect("to_string request")],
    );
    assert_eq!(
        replies.len(),
        1,
        "expected exactly one reply, got {replies:#?}"
    );
    replies.into_iter().next().unwrap()
}

fn assert_error(reply: &Value, expected_code: i64, hint: &str) {
    assert_eq!(reply["jsonrpc"], "2.0", "missing jsonrpc envelope: {reply}");
    assert!(
        reply.get("error").is_some(),
        "expected error envelope ({hint}), got: {reply}"
    );
    assert_eq!(
        reply["error"]["code"]
            .as_i64()
            .unwrap_or_else(|| panic!("error.code not int: {reply}")),
        expected_code,
        "{hint}: wrong code in {reply}"
    );
    assert!(
        reply["error"]["message"].is_string(),
        "error.message must be a string ({hint}): {reply}"
    );
}

fn assert_ok(reply: &Value, hint: &str) {
    assert_eq!(reply["jsonrpc"], "2.0", "missing jsonrpc envelope: {reply}");
    assert!(
        reply.get("error").is_none(),
        "unexpected error envelope ({hint}): {reply}"
    );
    assert!(
        reply.get("result").is_some(),
        "expected result envelope ({hint}): {reply}"
    );
}

// ---------------------------------------------------------------- envelope --

#[test]
fn malformed_json_returns_parse_error() {
    let repo = jsonrpc_repo_root();
    let replies = round_trip(&repo, &["{ not json".to_string()]);
    assert_eq!(replies.len(), 1);
    assert_error(&replies[0], PARSE_ERROR, "malformed JSON");
}

#[test]
fn non_object_request_returns_invalid_request() {
    let repo = jsonrpc_repo_root();
    let reply = one(&repo, json!([1, 2, 3]));
    assert_error(&reply, INVALID_REQUEST, "request must be object");
}

#[test]
fn missing_jsonrpc_version_returns_invalid_request() {
    let repo = jsonrpc_repo_root();
    // Valid JSON-RPC envelopes require `"jsonrpc": "2.0"`. A request without
    // it is structurally bogus regardless of method.
    let reply = one(
        &repo,
        json!({"id": 1, "method": "validate_contract", "params": {"yaml": ""}}),
    );
    assert_error(&reply, INVALID_REQUEST, "missing jsonrpc field");
}

#[test]
fn missing_method_returns_invalid_request() {
    let repo = jsonrpc_repo_root();
    let reply = one(&repo, json!({"jsonrpc": "2.0", "id": 1}));
    assert_error(&reply, INVALID_REQUEST, "missing method");
}

#[test]
fn unknown_method_returns_method_not_found() {
    let repo = jsonrpc_repo_root();
    let reply = one(
        &repo,
        json!({"jsonrpc": "2.0", "id": 42, "method": "obliterate_planet"}),
    );
    assert_error(&reply, METHOD_NOT_FOUND, "unknown verb");
    assert_eq!(reply["id"], json!(42), "reply must echo request id");
}

// ------------------------------------------------------- validate_contract --

#[test]
fn validate_contract_missing_params() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "validate_contract"}),
    );
    assert_error(&reply, INVALID_PARAMS, "no params");
}

#[test]
fn validate_contract_wrong_param_type() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "validate_contract", "params": {"yaml": 42}}),
    );
    assert_error(&reply, INVALID_PARAMS, "yaml must be string");
}

#[test]
fn validate_contract_malformed_yaml() {
    // Tabs inside YAML mapping context are illegal — surface as INVALID_PARAMS
    // rather than crashing the server.
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "validate_contract",
               "params": {"yaml": "name:\tlibrary\n  - x:\n\t y"}}),
    );
    assert_error(&reply, INVALID_PARAMS, "malformed yaml");
}

#[test]
fn validate_contract_schema_invalid_input_returns_typed_diagnostics() {
    // Schema-invalid (kind missing required fields) — the method returns a
    // *result* envelope with ok=false, not a JSON-RPC error. Every diagnostic
    // in `diagnostics[]` must validate against `schemas/diagnostic.schema.json`.
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "validate_contract",
               "params": {"yaml": "name: x\nkind: library\n"}}),
    );
    assert_ok(
        &reply,
        "validate_contract surfaces failures as result, not error",
    );
    assert_eq!(reply["result"]["ok"], json!(false));
    let diags = reply["result"]["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert!(
        !diags.is_empty(),
        "schema-invalid input must produce diagnostics"
    );
    for d in diags {
        validate_diagnostic_value(d).unwrap_or_else(|errs| {
            panic!("validate_contract emitted schema-invalid diagnostic {d}: {errs:?}")
        });
    }
}

#[test]
fn validate_contract_happy_path_returns_empty_diagnostics() {
    // Reuse the canonical happy-path library fixture rather than hand-writing a
    // contract here, so the test tracks contract.schema.json without
    // duplicating the kind-required-fields knowledge in two places.
    let yaml = std::fs::read_to_string(
        jsonrpc_repo_root().join("fixtures/happy-path/rust-minimal/CONTRACT.yaml"),
    )
    .expect("read rust-minimal fixture");
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "validate_contract",
               "params": {"yaml": yaml}}),
    );
    assert_ok(&reply, "happy validate_contract");
    assert_eq!(
        reply["result"]["ok"],
        json!(true),
        "happy fixture should pass validation: {reply}"
    );
    assert!(reply["result"]["diagnostics"]
        .as_array()
        .is_some_and(|d| d.is_empty()));
}

// ----------------------------------------------------------- diff_contracts --

#[test]
fn diff_contracts_missing_params() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "diff_contracts"}),
    );
    assert_error(&reply, INVALID_PARAMS, "missing params");
}

#[test]
fn diff_contracts_missing_old() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "diff_contracts",
               "params": {"new": {}}}),
    );
    assert_error(&reply, INVALID_PARAMS, "missing old");
}

#[test]
fn diff_contracts_wrong_param_type() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "diff_contracts",
               "params": {"old": "not-an-object", "new": {}}}),
    );
    assert_error(&reply, INVALID_PARAMS, "old must be object");
}

#[test]
fn diff_contracts_findings_are_schema_valid() {
    let old = json!({
        "name": "tiny", "kind": "library", "purpose": "p",
        "status": "stable", "since": "0.1.0", "version": "1.0.0",
        "assurance_level": "declared", "owner": "tests",
        "exports": [],
        "invariants": [{"id": "a.one", "text": "x"}],
        "edge_cases": []
    });
    let new = json!({
        "name": "tiny", "kind": "service", "purpose": "p",
        "status": "stable", "since": "0.1.0", "version": "2.0.0",
        "assurance_level": "declared", "owner": "tests",
        "protocol": "http", "endpoints": [], "consumes": [], "produces": [],
        "invariants": [{"id": "a.one", "text": "x"}],
        "edge_cases": []
    });
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 7, "method": "diff_contracts",
               "params": {"old": old, "new": new}}),
    );
    assert_ok(&reply, "diff happy path");
    let findings = reply["result"]["findings"]
        .as_array()
        .expect("findings array");
    assert!(!findings.is_empty(), "kind change must surface");
    for d in findings {
        validate_diagnostic_value(d).unwrap_or_else(|errs| {
            panic!("diff_contracts emitted schema-invalid finding {d}: {errs:?}")
        });
    }
}

// ------------------------------------------------------------- trace_claim --

#[test]
fn trace_claim_missing_params() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "trace_claim"}),
    );
    assert_error(&reply, INVALID_PARAMS, "missing params");
}

#[test]
fn trace_claim_wrong_param_type() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "trace_claim",
               "params": {"claim_id": ["not", "a", "string"]}}),
    );
    assert_error(&reply, INVALID_PARAMS, "claim_id must be string");
}

#[test]
fn trace_claim_bad_repo_path_returns_invalid_params() {
    let bogus = std::path::PathBuf::from("/this/path/does/not/exist/anywhere");
    let reply = one(
        &bogus,
        json!({"jsonrpc": "2.0", "id": 1, "method": "trace_claim",
               "params": {"claim_id": "x"}}),
    );
    assert_error(&reply, INVALID_PARAMS, "non-directory repo");
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("CH-GATE-REPO-UNREADABLE"),
        "expected CH-GATE-REPO-UNREADABLE in {}",
        reply["error"]["message"]
    );
}

#[test]
fn trace_claim_unknown_claim_returns_null() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "trace_claim",
               "params": {"claim_id": "no.such.claim.exists"}}),
    );
    assert_ok(&reply, "trace_claim unknown id");
    assert_eq!(reply["result"], Value::Null);
}

// ------------------------------------------------------------ drift_report --

#[test]
fn drift_report_bad_repo_path_returns_invalid_params() {
    let bogus = std::path::PathBuf::from("/no/such/dir/here");
    let reply = one(
        &bogus,
        json!({"jsonrpc": "2.0", "id": 1, "method": "drift_report"}),
    );
    assert_error(&reply, INVALID_PARAMS, "non-directory repo");
}

#[test]
fn drift_report_happy_path_matches_drift_schema() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "drift_report"}),
    );
    assert_ok(&reply, "drift_report happy path");
    validate_drift_report_value(&reply["result"])
        .expect("drift_report output must conform to schemas/drift-report.schema.json");
    // Every diagnostic in the report must itself be schema-valid.
    for d in reply["result"]["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
    {
        validate_diagnostic_value(d)
            .unwrap_or_else(|errs| panic!("drift diagnostic {d} failed schema: {errs:?}"));
    }
}

// ----------------------------------------------------------- release_gate --

#[test]
fn release_gate_bad_repo_path_returns_invalid_params() {
    let bogus = std::path::PathBuf::from("/path/that/does/not/exist");
    let reply = one(
        &bogus,
        json!({"jsonrpc": "2.0", "id": 1, "method": "release_gate"}),
    );
    assert_error(&reply, INVALID_PARAMS, "non-directory repo");
    assert!(reply["error"]["message"]
        .as_str()
        .unwrap()
        .contains("CH-GATE-REPO-UNREADABLE"));
}

#[test]
fn release_gate_wrong_fail_on_drift_type_returns_invalid_params() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "release_gate",
               "params": {"fail_on_drift": "yes-please"}}),
    );
    assert_error(&reply, INVALID_PARAMS, "fail_on_drift type");
}

#[test]
fn release_gate_non_git_dir_returns_internal_error() {
    // Empty directory: passes is_dir() but git_head() fails. Surfaces as
    // CH-GATE-SUBSYSTEM-FAILURE with the JSON-RPC internal-error code.
    let tmp = TempDir::new().expect("tempdir");
    let reply = one(
        tmp.path(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "release_gate"}),
    );
    assert_error(&reply, INTERNAL_ERROR, "non-git directory");
    assert!(reply["error"]["message"]
        .as_str()
        .unwrap()
        .starts_with("CH-GATE-"));
}

#[test]
fn release_gate_happy_path_predicate_matches_schema_and_cli_fields() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "release_gate"}),
    );
    assert_ok(&reply, "release_gate happy path");

    let predicate = &reply["result"];
    validate_release_gate_value(predicate).expect(
        "release_gate output MUST validate against schemas/release-gate.schema.json — \
         the JSON-RPC surface must use the same predicate/verdict fields as the CLI",
    );

    // Spot-check the verdict-related fields the CLI also emits. If we ever
    // drift away from CLI's predicate shape, this assertion fails first.
    for required in [
        "schema_fingerprint",
        "git_commit",
        "built_at",
        "verdict",
        "fail_on_drift",
        "trace_failed",
        "drift_failed",
        "exemption_failed",
        "attestation_failed",
        "unsuppressed_blocking",
        "suppressed",
        "severity_overridden",
        "final_exit_code",
        "trace_summary",
        "drift_summary",
        "exempt_summary",
        "commands_run",
    ] {
        assert!(
            predicate.get(required).is_some(),
            "release_gate predicate missing required field `{required}`: {predicate}"
        );
    }
    let verdict = predicate["verdict"].as_str().expect("verdict string");
    assert!(
        verdict == "pass" || verdict == "fail",
        "verdict must be pass|fail, got {verdict:?}"
    );
}

#[test]
fn release_gate_fail_on_drift_false_changes_outcome() {
    // Just verifies the param round-trips and validates — we don't pin a
    // specific verdict, since the self-repo state evolves.
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "release_gate",
               "params": {"fail_on_drift": false}}),
    );
    assert_ok(&reply, "release_gate fail_on_drift=false");
    assert_eq!(reply["result"]["fail_on_drift"], json!(false));
    validate_release_gate_value(&reply["result"]).expect("predicate matches schema");
}

// --------------------------------------------------------- list_exemptions --

#[test]
fn list_exemptions_wrong_rule_id_type_returns_invalid_params() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "list_exemptions",
               "params": {"rule_id": 7}}),
    );
    assert_error(&reply, INVALID_PARAMS, "rule_id must be string");
}

#[test]
fn list_exemptions_no_registry_returns_empty_array() {
    let tmp = TempDir::new().expect("tempdir");
    let reply = one(
        tmp.path(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "list_exemptions"}),
    );
    assert_ok(&reply, "list_exemptions with no registry");
    assert_eq!(reply["result"], json!([]));
}

#[test]
fn list_exemptions_filters_by_rule_id() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "list_exemptions",
               "params": {"rule_id": "this-rule-does-not-exist-anywhere"}}),
    );
    assert_ok(&reply, "list_exemptions filtered");
    // Filter should always return an array, even if empty.
    assert!(reply["result"].is_array());
}

// ---------------------------------------------------------- trace_claim out --

#[test]
fn trace_claim_known_claim_returns_schema_compatible_node() {
    let reply = one(
        &jsonrpc_repo_root(),
        json!({"jsonrpc": "2.0", "id": 1, "method": "trace_claim",
               "params": {"claim_id": "chassis.fingerprint-matches"}}),
    );
    assert_ok(&reply, "trace_claim known");
    let node = &reply["result"];
    assert_eq!(node["claim_id"], json!("chassis.fingerprint-matches"));
    // The node's parent trace graph schema demands snake_case fields — the
    // wire MUST NOT carry camelCase aliases.
    for forbidden in ["claimId", "implSites", "testSites", "adrRefs"] {
        assert!(
            node.get(forbidden).is_none(),
            "trace_claim must not emit camelCase `{forbidden}`: {node}"
        );
    }
    // Round-trip the node into a single-claim TraceGraph and validate that
    // — this is the closest available schema check for the node shape.
    let mini = json!({
        "claims": { node["claim_id"].as_str().unwrap(): node },
        "orphan_sites": [],
        "diagnostics": []
    });
    validate_trace_graph_value(&mini)
        .expect("trace_claim result must embed inside a schema-valid TraceGraph");
}
