mod common;

use chassis_core::artifact::validate_trace_graph_value;
use serde_json::Value;
use tempfile::TempDir;

use common::{chassis, exit, write, VALID_LIBRARY_YAML};

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

#[test]
fn trace_happy_path_on_empty_dir_exits_0() {
    let dir = TempDir::new().unwrap();

    chassis()
        .args(["trace", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);
}

#[test]
fn trace_json_output_validates_against_trace_graph_schema() {
    let dir = TempDir::new().unwrap();
    // Drop in a contract so the graph has a claim node; impl_sites stays empty.
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);

    let assert = chassis()
        .args(["--json", "trace", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON trace graph");
    validate_trace_graph_value(&v).expect("trace graph must validate against canonical schema");
    assert!(
        v["claims"].is_object(),
        "trace graph must contain a `claims` object"
    );
}

#[test]
fn trace_mermaid_overrides_json_and_emits_graph_header() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);

    chassis()
        .args(["--json", "trace", "--mermaid", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK)
        // Renderer should produce a Mermaid `graph` opening line, not a JSON document.
        .stdout(predicates::str::contains("graph"));
}

#[test]
fn trace_missing_repo_exits_nonzero() {
    let assert = chassis()
        .args(["--json", "trace", "--repo", "/nonexistent/path/no/repo"])
        .assert()
        .failure();

    let code = assert.get_output().status.code().expect("exit code");
    assert_ne!(code, exit::OK, "trace on missing repo must exit nonzero");
}

#[test]
fn trace_malformed_contract_yaml_surfaces_failure() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", ":\n\t- not yaml\n");

    let assert = chassis()
        .args(["--json", "trace", "--repo"])
        .arg(dir.path())
        .assert()
        .failure();

    let code = assert.get_output().status.code().expect("exit code");
    assert_ne!(
        code,
        exit::OK,
        "trace over malformed contract must exit nonzero"
    );
}
