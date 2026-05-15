//! `chassis spec-index export|validate|link` integration tests.

mod common;

use std::fs;

use assert_cmd::assert::Assert;
use serde_json::Value;
use tempfile::TempDir;

use common::{chassis, exit, git_init_with_initial_commit, write, VALID_LIBRARY_YAML};

const SPEC_SOURCE: &str = r#"
version: 1
chassis_preset_version: 1
feature_id: fixture-export
title: "f"
constitution_principles:
  - id: P1
    text: x
non_goals: []
requirements:
  - id: REQ-001
    title: r
    description: d
    acceptance_criteria: ["a"]
    claim_ids:
      - cli.tests.alpha
      - cli.tests.edge.one
    related_task_ids: [TASK-001]
    touched_paths:
      - crates/demo/src/lib.rs
tasks:
  - id: TASK-001
    title: t
    depends_on: []
    touched_paths:
      - crates/demo/src/lib.rs
implementation_constraints: []
"#;

fn stdout(out: &Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

fn traced_repo(dir: &std::path::Path) {
    write(dir, "CONTRACT.yaml", VALID_LIBRARY_YAML);
    write(
        dir,
        "crates/demo/src/lib.rs",
        r#"
// @claim cli.tests.alpha
pub fn alpha() {}
// @claim cli.tests.edge.one
pub fn edge() {}
#[test]
// @claim cli.tests.alpha
fn t1() {}
#[test]
// @claim cli.tests.edge.one
fn t2() {}
"#,
    );
    git_init_with_initial_commit(dir);
}

#[test]
fn spec_index_export_is_deterministic_json() {
    let dir = TempDir::new().unwrap();
    traced_repo(dir.path());
    let src = dir.path().join("spec.yaml");
    fs::write(&src, SPEC_SOURCE.trim_start()).expect("write source");
    let out = dir.path().join("artifacts/spec-index.json");
    let a1 = chassis()
        .args(["--repo"])
        .arg(dir.path())
        .args(["spec-index", "export", "--from"])
        .arg(&src)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(exit::OK);
    let d1 = stdout(&a1);

    let a2 = chassis()
        .args(["--repo"])
        .arg(dir.path())
        .args(["spec-index", "export", "--from"])
        .arg(&src)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(exit::OK);
    let d2 = stdout(&a2);
    assert_eq!(d1, d2, "export stdout must be stable across runs");

    let raw = fs::read_to_string(&out).expect("read emitted json");
    let v1: Value = serde_json::from_str(&raw).expect("json");
    let v2: Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v1, v2);

    let json = chassis()
        .args(["--json", "--repo"])
        .arg(dir.path())
        .args(["spec-index", "export", "--from"])
        .arg(&src)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(exit::OK);
    let env: Value = serde_json::from_str(&stdout(&json)).expect("export json");
    assert_eq!(env["ok"], true);
    assert!(env["digest"].as_str().unwrap().len() == 64);
}

#[test]
fn spec_index_validate_passes_for_exported_index() {
    let dir = TempDir::new().unwrap();
    traced_repo(dir.path());
    let src = dir.path().join("spec.yaml");
    fs::write(&src, SPEC_SOURCE.trim_start()).expect("write source");
    let out = dir.path().join("artifacts/spec-index.json");
    chassis()
        .args(["--repo"])
        .arg(dir.path())
        .args(["spec-index", "export", "--from"])
        .arg(&src)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(exit::OK);

    chassis()
        .args(["spec-index", "validate"])
        .arg(&out)
        .assert()
        .code(exit::OK);
}

#[test]
fn spec_index_link_passes_when_trace_matches_contract() {
    let dir = TempDir::new().unwrap();
    traced_repo(dir.path());
    let src = dir.path().join("spec.yaml");
    fs::write(&src, SPEC_SOURCE.trim_start()).expect("write source");
    let out = dir.path().join("artifacts/spec-index.json");
    chassis()
        .args(["--repo"])
        .arg(dir.path())
        .args(["spec-index", "export", "--from"])
        .arg(&src)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(exit::OK);

    let assert = chassis()
        .args(["--json", "--repo"])
        .arg(dir.path())
        .args(["spec-index", "link", "--index"])
        .arg(&out)
        .assert()
        .code(exit::OK);
    let v: Value = serde_json::from_str(&stdout(&assert)).expect("link json");
    assert_eq!(v["ok"], true);
}

#[test]
fn spec_index_link_fails_on_unknown_claim() {
    let dir = TempDir::new().unwrap();
    traced_repo(dir.path());
    let bad = include_str!("fixtures/spec_index_bad_unknown_claim.json");
    let path = dir.path().join("artifacts/spec-index.json");
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, bad).expect("write bad spec");

    let assert = chassis()
        .args(["--json", "--repo"])
        .arg(dir.path())
        .args(["spec-index", "link", "--index"])
        .arg(&path)
        .assert()
        .code(exit::VALIDATE_FAILED);
    let v: Value = serde_json::from_str(&stdout(&assert)).expect("link json");
    assert_eq!(v["ok"], false);
    let diags = v["diagnostics"].as_array().unwrap();
    assert!(
        diags
            .iter()
            .any(|d| d["ruleId"] == "CH-SPEC-UNKNOWN-CLAIM-REF"),
        "expected unknown claim ref diagnostic: {diags:?}"
    );
}
