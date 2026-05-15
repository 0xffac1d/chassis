mod common;

use serde_json::Value;
use tempfile::TempDir;

use common::{
    chassis, exit, write, BREAKING_VERSION_NOT_BUMPED_NEW, BREAKING_VERSION_NOT_BUMPED_OLD,
    VALID_LIBRARY_NEW_YAML, VALID_LIBRARY_YAML,
};

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

#[test]
fn diff_additive_change_exits_0() {
    let dir = TempDir::new().unwrap();
    let old = write(dir.path(), "old.yaml", VALID_LIBRARY_YAML);
    let new = write(dir.path(), "new.yaml", VALID_LIBRARY_NEW_YAML);

    chassis()
        .args(["diff"])
        .arg(&old)
        .arg(&new)
        .assert()
        .code(exit::OK);
}

#[test]
fn diff_additive_change_json_payload_has_findings_array() {
    let dir = TempDir::new().unwrap();
    let old = write(dir.path(), "old.yaml", VALID_LIBRARY_YAML);
    let new = write(dir.path(), "new.yaml", VALID_LIBRARY_NEW_YAML);

    let assert = chassis()
        .args(["--json", "diff"])
        .arg(&old)
        .arg(&new)
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON DiffReport");
    assert!(v["schema_version"].is_string(), "DiffReport.schema_version");
    assert!(v["findings"].is_array(), "DiffReport.findings");
}

#[test]
fn diff_breaking_change_exits_4() {
    let dir = TempDir::new().unwrap();
    let old = write(dir.path(), "old.yaml", BREAKING_VERSION_NOT_BUMPED_OLD);
    let new = write(dir.path(), "new.yaml", BREAKING_VERSION_NOT_BUMPED_NEW);

    let assert = chassis()
        .args(["--json", "diff"])
        .arg(&old)
        .arg(&new)
        .assert()
        .code(exit::DIFF_BREAKING);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON DiffReport");
    let findings = v["findings"].as_array().expect("findings array");
    let any_breaking = findings.iter().any(|f| {
        f.get("detail")
            .and_then(|d| d.get("classification"))
            .and_then(|c| c.as_str())
            == Some("breaking")
    });
    assert!(
        any_breaking,
        "exit code is DIFF_BREAKING but no finding carries classification=breaking"
    );
}

#[test]
fn diff_missing_old_file_exits_66() {
    let dir = TempDir::new().unwrap();
    let new = write(dir.path(), "new.yaml", VALID_LIBRARY_YAML);

    chassis()
        .args(["--json", "diff", "/nonexistent.yaml"])
        .arg(&new)
        .assert()
        .code(exit::MISSING_FILE);
}

#[test]
fn diff_malformed_old_yaml_exits_65() {
    let dir = TempDir::new().unwrap();
    let old = write(dir.path(), "old.yaml", "name: x\n\tkind: library\n");
    let new = write(dir.path(), "new.yaml", VALID_LIBRARY_YAML);

    chassis()
        .args(["--json", "diff"])
        .arg(&old)
        .arg(&new)
        .assert()
        .code(exit::MALFORMED_INPUT);
}

#[test]
fn diff_empty_json_object_is_malformed_for_diff_engine() {
    // The diff engine requires non-empty object inputs; `{}` is the canonical
    // adversarial parse-error fixture under fixtures/diff/parse-error/.
    let dir = TempDir::new().unwrap();
    let old = write(dir.path(), "old.yaml", "{}");
    let new = write(dir.path(), "new.yaml", VALID_LIBRARY_YAML);

    chassis()
        .args(["--json", "diff"])
        .arg(&old)
        .arg(&new)
        .assert()
        .code(exit::MALFORMED_INPUT);
}
