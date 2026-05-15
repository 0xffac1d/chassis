mod common;

use chassis_core::artifact::validate_drift_report_value;
use serde_json::Value;
use tempfile::TempDir;

use common::{chassis, exit, write, VALID_LIBRARY_YAML};

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

#[test]
fn drift_happy_path_no_contracts_exits_0_with_zero_summary() {
    let dir = TempDir::new().unwrap();

    let assert = chassis()
        .args(["--json", "drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON drift report");
    validate_drift_report_value(&v).expect("drift report must validate against canonical schema");
    assert_eq!(v["summary"]["stale"], 0);
    assert_eq!(v["summary"]["abandoned"], 0);
    assert_eq!(v["summary"]["missing"], 0);
}

#[test]
fn drift_missing_impl_sites_reports_missing_and_exits_5() {
    // A contract whose invariants have no @claim site anywhere in the tree
    // ⇒ every claim is `missing` ⇒ exit code DRIFT_DETECTED.
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);

    let assert = chassis()
        .args(["--json", "drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::DRIFT_DETECTED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON drift report");
    validate_drift_report_value(&v).expect("drift report must validate against canonical schema");
    let missing = v["summary"]["missing"].as_u64().expect("missing");
    assert!(
        missing >= 1,
        "expected at least one `missing` claim, got summary={}",
        v["summary"]
    );
}

#[test]
fn drift_missing_repo_dir_exits_nonzero() {
    let assert = chassis()
        .args(["--json", "drift", "--repo", "/nonexistent/path/no/repo"])
        .assert()
        .failure();

    let code = assert.get_output().status.code().expect("exit code");
    assert_ne!(code, exit::OK);
}

#[test]
fn drift_malformed_contract_yaml_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", ":\n\t- not yaml\n");

    let assert = chassis()
        .args(["--json", "drift", "--repo"])
        .arg(dir.path())
        .assert()
        .failure();

    let code = assert.get_output().status.code().expect("exit code");
    assert_ne!(code, exit::OK);
}
