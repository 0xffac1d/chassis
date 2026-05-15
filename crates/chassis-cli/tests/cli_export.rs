mod common;

use chassis_core::artifact::{
    validate_cedar_facts_value, validate_eventcatalog_metadata_value, validate_opa_input_value,
    validate_policy_input_value,
};
use serde_json::Value;
use tempfile::TempDir;

use common::{chassis, exit, git_init_with_initial_commit, write, VALID_LIBRARY_YAML};

const EVENT_STREAM_YAML: &str = r#"
name: "account-events"
kind: event-stream
version: "0.1.0"
purpose: "Publish account lifecycle events for downstream consumers."
status: stable
since: "0.1.0"
assurance_level: declared
owner: platform
source: accounts
payload:
  format: json
  schema_ref: schemas/account-event.json
delivery: at-least-once
consumers:
  - billing
invariants:
  - id: account-events.published
    text: "Account events are published to the stream."
edge_cases: []
"#;

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

fn repo_with_contract(body: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", body);
    git_init_with_initial_commit(dir.path());
    dir
}

#[test]
fn export_chassis_policy_input_validates() {
    let dir = repo_with_contract(VALID_LIBRARY_YAML);

    let assert = chassis()
        .args(["--json", "--repo"])
        .arg(dir.path())
        .args(["export", "--format", "chassis"])
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON policy input");
    validate_policy_input_value(&v).expect("policy input export validates");
    assert!(!v["contracts"].as_array().unwrap().is_empty());
    assert!(!v["claims"].as_array().unwrap().is_empty());
}

#[test]
fn export_opa_wraps_chassis_input() {
    let dir = repo_with_contract(VALID_LIBRARY_YAML);

    let assert = chassis()
        .args(["--json", "--repo"])
        .arg(dir.path())
        .args(["export", "--format", "opa"])
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON OPA input");
    validate_opa_input_value(&v).expect("OPA input export validates");
    assert!(v.get("input").is_some());
    assert!(!v["input"]["contracts"].as_array().unwrap().is_empty());
}

#[test]
fn export_cedar_emits_entity_action_resource_facts() {
    let dir = repo_with_contract(VALID_LIBRARY_YAML);

    let assert = chassis()
        .args(["--json", "--repo"])
        .arg(dir.path())
        .args(["export", "--format", "cedar"])
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON Cedar-style facts");
    validate_cedar_facts_value(&v).expect("Cedar-style facts export validates");
    assert!(v["entities"].as_array().unwrap().len() >= 2);
    assert!(v["actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["name"] == "drift"));
    assert!(!v["resources"].as_array().unwrap().is_empty());
}

#[test]
fn export_eventcatalog_uses_event_stream_metadata_only() {
    let dir = repo_with_contract(EVENT_STREAM_YAML);

    let assert = chassis()
        .args(["--json", "--repo"])
        .arg(dir.path())
        .args(["export", "--format", "eventcatalog"])
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON EventCatalog metadata");
    validate_eventcatalog_metadata_value(&v).expect("EventCatalog metadata export validates");
    assert_eq!(v["messages"].as_array().unwrap().len(), 1);
    assert_eq!(v["services"].as_array().unwrap().len(), 0);
    assert!(v["metadata"]["note"]
        .as_str()
        .unwrap()
        .contains("Export-only metadata"));
}
