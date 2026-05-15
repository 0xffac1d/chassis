mod common;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::Value;
use tempfile::TempDir;

use common::{
    chassis, exit, git_init_with_initial_commit, write, write_keypair, VALID_LIBRARY_YAML,
};

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

fn traced_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    write(
        dir.path(),
        "crates/demo/src/lib.rs",
        r#"
// @claim cli.tests.alpha
pub fn alpha() -> bool { true }

// @claim cli.tests.edge.one
pub fn edge() -> bool { true }

#[test]
// @claim cli.tests.alpha
fn alpha_is_true() { assert!(alpha()); }

#[test]
// @claim cli.tests.edge.one
fn edge_is_true() { assert!(edge()); }
"#,
    );
    git_init_with_initial_commit(dir.path());
    dir
}

#[test]
fn release_gate_happy_path_outputs_end_to_end_summary() {
    let dir = traced_repo();

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["verdict"], "pass");
    assert!(v["schema_fingerprint"]
        .as_str()
        .is_some_and(|s| s.len() == 64));
    assert!(v["git_commit"].as_str().is_some_and(|s| s.len() == 40));
    assert_eq!(v["contract_validation"]["invalid"], 0);
    assert_eq!(v["trace_summary"]["orphan_sites"], 0);
    assert_eq!(v["trace_summary"]["missing_impl"], 0);
    assert_eq!(v["trace_summary"]["missing_tests"], 0);
    assert_eq!(v["drift_summary"]["missing"], 0);
    assert!(dir.path().join("release-gate.json").is_file());
}

#[test]
fn release_gate_invalid_contract_fails_before_trace() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", "name: broken\nkind: library\n");

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::VALIDATE_FAILED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["contract_validation"]["invalid"], 1);
    assert!(v["artifact_path"].is_null());
}

#[test]
fn release_gate_orphan_claim_fails_trace_gate() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    write(
        dir.path(),
        "crates/demo/src/lib.rs",
        r#"
// @claim cli.tests.not-in-contract
pub fn orphan() {}
"#,
    );
    git_init_with_initial_commit(dir.path());

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::DRIFT_DETECTED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["trace_summary"]["orphan_sites"], 1);
}

#[test]
fn release_gate_fail_on_drift_rejects_missing_claim_sites() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    git_init_with_initial_commit(dir.path());

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::DRIFT_DETECTED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], false);
    assert!(v["drift_summary"]["missing"].as_u64().unwrap() >= 1);
    assert!(
        v["drift_summary"]["unsuppressed_blocking"]
            .as_u64()
            .unwrap()
            >= 1
    );
}

#[test]
fn release_gate_bad_exemption_fails() {
    let dir = traced_repo();
    write(
        dir.path(),
        ".chassis/exemptions.yaml",
        r#"version: 2
entries:
  - id: EX-2026-0001
    rule_id: CH-DRIFT-IMPL-MISSING
    reason: "Expired exemption must fail the release gate."
    owner: platform-team@docs.invalid
    created_at: "2024-01-01"
    expires_at: "2024-01-31"
    path: crates/demo/src/lib.rs
    codeowner_acknowledgments:
      - "@platform-team"
"#,
    );
    write(dir.path(), "CODEOWNERS", "crates/demo/** @platform-team\n");

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::EXEMPT_VIOLATION);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["exemption_summary"]["errors"], 1);
}

#[test]
fn release_gate_attestation_tamper_fails_verify() {
    let dir = traced_repo();
    let (_priv_path, pub_path) = write_keypair(dir.path());

    chassis()
        .args([
            "--json",
            "release-gate",
            "--fail-on-drift",
            "--attest",
            "--repo",
        ])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let dsse_path = dir.path().join("release-gate.dsse");
    let mut env: Value =
        serde_json::from_str(&std::fs::read_to_string(&dsse_path).unwrap()).expect("DSSE JSON");
    env["payload"] = Value::String(B64.encode(br#"{"tampered":true}"#));
    std::fs::write(&dsse_path, serde_json::to_string_pretty(&env).unwrap()).unwrap();

    chassis()
        .args(["--json", "attest", "verify"])
        .arg(&dsse_path)
        .args(["--public-key"])
        .arg(pub_path)
        .args(["--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::ATTEST_VERIFY_FAILED);
}
