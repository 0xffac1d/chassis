#![forbid(unsafe_code)]

//! Integration tests for JSON-RPC over stdin (newline-delimited).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};

fn mcp_repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spawn_mcp(repo: &Path) -> std::process::Child {
    Command::new(cargo_bin("chassis-mcp"))
        .env("CHASSIS_REPO_ROOT", repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn chassis-mcp")
}

fn read_ndjson_reply(reader: &mut impl BufRead) -> Value {
    let mut buf = String::new();
    reader.read_line(&mut buf).expect("read line");
    serde_json::from_str(buf.trim()).expect("valid json response")
}

#[test]
fn malformed_json_returns_parse_error_negative_32700() {
    let tmp = std::env::temp_dir().join(format!(
        "chassis-mcp-malformed-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    let mut child = spawn_mcp(&tmp);
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
    assert_eq!(
        second.pointer("/error/message").and_then(Value::as_str),
        Some(chassis_core::attest::CH_ATTEST_NOT_FOUND)
    );
    assert_eq!(second["error"]["code"], -32603);
}

#[test]
fn validate_contract_returns_result_envelope() {
    let repo = mcp_repo_root();
    let mut child = spawn_mcp(&repo);
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
}

#[test]
fn trace_claim_hits_known_root_contract_claim() {
    let repo = mcp_repo_root();
    let mut child = spawn_mcp(&repo);
    {
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","id":10,"method":"trace_claim","params":{"claim_id":"chassis.tests-green"}})
        )
        .unwrap();
    }

    let out = child.wait_with_output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).expect("reply");
    assert_eq!(
        v["result"]["claimId"].as_str().unwrap(),
        "chassis.tests-green"
    );
    assert!(
        v["result"]["contractPath"]
            .as_str()
            .unwrap()
            .contains("CONTRACT.yaml"),
        "got {}",
        serde_json::to_string(&v["result"]).unwrap()
    );
}
