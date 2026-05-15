#![forbid(unsafe_code)]

//! Integration tests for JSON-RPC over stdin (newline-delimited).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};

fn jsonrpc_repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spawn_jsonrpc(repo: &Path) -> std::process::Child {
    Command::new(cargo_bin("chassis-jsonrpc"))
        .env("CHASSIS_REPO_ROOT", repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn chassis-jsonrpc")
}

// @claim chassis.jsonrpc-not-mcp
#[test]
fn jsonrpc_surface_is_documented_as_not_mcp() {
    let repo = jsonrpc_repo_root();
    let source =
        std::fs::read_to_string(repo.join("crates/chassis-jsonrpc/src/main.rs")).expect("source");

    assert!(
        source.contains("JSON-RPC 2.0 server over stdio")
            && source.contains("not")
            && source.contains("Model Context Protocol"),
        "jsonrpc source must document the surface as custom JSON-RPC, not MCP"
    );
    assert!(
        !source.contains("MCP server"),
        "jsonrpc source must not advertise itself as an MCP server"
    );
}

fn read_ndjson_reply(reader: &mut impl BufRead) -> Value {
    let mut buf = String::new();
    reader.read_line(&mut buf).expect("read line");
    serde_json::from_str(buf.trim()).expect("valid json response")
}

#[test]
fn malformed_json_returns_parse_error_negative_32700() {
    let tmp = std::env::temp_dir().join(format!(
        "chassis-jsonrpc-malformed-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    let mut child = spawn_jsonrpc(&tmp);
    {
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(stdin, "{{broken").unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":991,"method":"release_gate"}}"#
        )
        .unwrap();
    }

    let stdout = child.wait_with_output().expect("wait").stdout;
    let mut lines = BufReader::new(stdout.as_slice());

    let first: Value = read_ndjson_reply(&mut lines);
    assert_eq!(first["jsonrpc"], "2.0");
    assert_eq!(first["error"]["code"], -32700);
    assert_eq!(first["error"]["message"], "Parse error");

    let second: Value = read_ndjson_reply(&mut lines);
    assert_eq!(second["id"], json!(991));
    // The temp dir is not a git repo: release_gate must fail fast with the
    // stable source-archive / missing-.git rule id (not a generic subsystem error).
    let msg = second
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        msg.starts_with("CH-GATE-GIT-METADATA-REQUIRED:"),
        "expected CH-GATE-GIT-METADATA-REQUIRED, got: {msg}"
    );
    assert_eq!(second["error"]["code"], -32602);
}

#[test]
fn validate_contract_returns_result_envelope() {
    let repo = jsonrpc_repo_root();
    let mut child = spawn_jsonrpc(&repo);
    {
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","id":3,"method":"validate_contract","params":{"yaml":"name: x\nkind: library\n"}})
        )
        .unwrap();
    }

    let out = child.wait_with_output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).expect("reply");
    assert!(
        matches!(v["result"]["ok"].as_bool(), Some(false)),
        "expected failing validation envelope, got {}",
        serde_json::to_string(&v).unwrap()
    );
    assert!(
        v["result"]["diagnostics"]
            .as_array()
            .is_some_and(|d| !d.is_empty()),
        "{}",
        serde_json::to_string(&v["result"]).unwrap()
    );

    // Governance: every diagnostic the RPC method emits must conform to
    // schemas/diagnostic.schema.json AND its ruleId must resolve to an
    // accepted ADR. This guards against the "chassis.contract" regression that
    // previously shipped invalid `violated.convention` values on the wire.
    use chassis_core::diagnostic::{validate_diagnostics_diagnostic, Diagnostic};
    use chassis_core::diagnostic_registry::AdrRuleRegistry;
    let reg = AdrRuleRegistry::load(repo.canonicalize().unwrap().as_path())
        .expect("ADR registry loads from repo");
    let arr = v["result"]["diagnostics"].as_array().unwrap();
    for raw in arr {
        validate_diagnostics_diagnostic(raw).unwrap_or_else(|errs| {
            panic!("validate_contract emitted schema-invalid diagnostic: {errs:?} for {raw}")
        });
        let typed: Diagnostic =
            serde_json::from_value(raw.clone()).expect("diagnostic deserializes");
        typed.assert_adr_bound(&reg);
    }
}

#[test]
fn diff_contracts_findings_are_schema_valid_and_adr_bound() {
    let repo = jsonrpc_repo_root();
    let mut child = spawn_jsonrpc(&repo);
    {
        let mut stdin = child.stdin.take().expect("stdin");
        // Build a minimal pair of contract documents that the diff surface
        // accepts and that produces at least one Diagnostic finding
        // (kind changed: library -> service).
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
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","id":7,"method":"diff_contracts","params":{"old": old, "new": new}})
        )
        .unwrap();
    }

    let out = child.wait_with_output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).expect("reply");
    let findings = v["result"]["findings"]
        .as_array()
        .expect("findings array")
        .clone();
    assert!(!findings.is_empty(), "diff_contracts produced no findings");

    use chassis_core::diagnostic::{validate_diagnostics_diagnostic, Diagnostic};
    use chassis_core::diagnostic_registry::AdrRuleRegistry;
    let reg = AdrRuleRegistry::load(repo.canonicalize().unwrap().as_path())
        .expect("ADR registry loads from repo");
    for raw in findings {
        validate_diagnostics_diagnostic(&raw).unwrap_or_else(|errs| {
            panic!("diff_contracts emitted schema-invalid finding: {errs:?} for {raw}")
        });
        let typed: Diagnostic = serde_json::from_value(raw.clone()).expect("finding deserializes");
        typed.assert_adr_bound(&reg);
    }
}

#[test]
fn trace_claim_hits_known_root_contract_claim() {
    let repo = jsonrpc_repo_root();
    let mut child = spawn_jsonrpc(&repo);
    {
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","id":10,"method":"trace_claim","params":{"claim_id":"chassis.fingerprint-matches"}})
        )
        .unwrap();
    }

    let out = child.wait_with_output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).expect("reply");
    // Trace artifacts conform to schemas/trace-graph.schema.json, which is
    // snake_case — see crates/chassis-core/src/trace/types.rs.
    assert_eq!(
        v["result"]["claim_id"].as_str().unwrap(),
        "chassis.fingerprint-matches"
    );
    assert!(
        v["result"]["contract_path"]
            .as_str()
            .unwrap()
            .contains("CONTRACT.yaml"),
        "got {}",
        serde_json::to_string(&v["result"]).unwrap()
    );
    assert!(
        v["result"].get("claimId").is_none(),
        "trace_claim must not emit camelCase claimId"
    );
}
