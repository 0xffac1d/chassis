mod common;

use serde_json::Value;
use tempfile::TempDir;

use common::{chassis, exit, write, SCHEMA_INVALID_YAML, VALID_LIBRARY_YAML};

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

#[test]
fn validate_happy_path_text() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);

    chassis()
        .args(["validate"])
        .arg(&path)
        .assert()
        .code(exit::OK)
        .stdout(predicates::str::contains("ok"));
}

#[test]
fn validate_happy_path_json_envelope_is_machine_readable() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);

    let assert = chassis()
        .args(["--json", "validate"])
        .arg(&path)
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("stdout is JSON");
    assert_eq!(v["ok"], Value::Bool(true), "ok must be literal true");
    assert!(v["path"].is_string(), "path field must be present");
}

#[test]
fn validate_missing_file_exits_66() {
    let assert = chassis()
        .args(["--json", "validate", "/nonexistent/CONTRACT.yaml"])
        .assert()
        .code(exit::MISSING_FILE);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON error envelope");
    assert_eq!(v["ok"], Value::Bool(false));
    assert_eq!(v["error"]["code"], Value::String("CLI-MISSING-FILE".into()));
}

#[test]
fn validate_malformed_yaml_exits_65() {
    let dir = TempDir::new().unwrap();
    // Tab indentation after a `:` is a hard YAML parse error.
    let path = write(dir.path(), "broken.yaml", "name: foo\n\tkind: library\n");

    let assert = chassis()
        .args(["--json", "validate"])
        .arg(&path)
        .assert()
        .code(exit::MALFORMED_INPUT);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON error envelope");
    assert_eq!(v["error"]["code"], "CLI-MALFORMED-INPUT");
}

#[test]
fn validate_schema_invalid_exits_2_and_emits_errors_array() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "CONTRACT.yaml", SCHEMA_INVALID_YAML);

    let assert = chassis()
        .args(["--json", "validate"])
        .arg(&path)
        .assert()
        .code(exit::VALIDATE_FAILED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["ok"], Value::Bool(false));
    let errors = v["errors"].as_array().expect("errors array");
    assert!(
        !errors.is_empty(),
        "schema-invalid contract must surface at least one validator error"
    );
}

#[test]
fn validate_schema_invalid_text_mode_writes_to_stderr() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "CONTRACT.yaml", SCHEMA_INVALID_YAML);

    chassis()
        .args(["validate"])
        .arg(&path)
        .assert()
        .code(exit::VALIDATE_FAILED)
        .stderr(predicates::str::contains("validate failed"));
}
