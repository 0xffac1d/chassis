mod common;

use serde_json::Value;
use tempfile::TempDir;

use common::{chassis, exit, write};

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

const EMPTY_REGISTRY: &str = "version: 2\nentries: []\n";

const SINGLE_ACTIVE_REGISTRY: &str = "version: 2
entries:
  - id: EX-2026-0001
    rule_id: CH-DIFF-CLAIM-REMOVED
    reason: \"Vendored upstream parser uses unwrap() on tokenizer state; rewrite tracked in CHASSIS-142.\"
    owner: platform-team@docs.invalid
    created_at: \"2026-04-01\"
    expires_at: \"2026-06-30\"
    path: crates/legacy/src/parser.rs
    codeowner_acknowledgments:
      - \"@platform-team\"
";

const EXPIRED_REGISTRY: &str = "version: 2
entries:
  - id: EX-2026-0001
    rule_id: CH-DIFF-CLAIM-REMOVED
    reason: \"Stale exemption that should already have been swept.\"
    owner: platform-team@docs.invalid
    created_at: \"2024-01-01\"
    expires_at: \"2024-01-31\"
    path: crates/legacy/src/parser.rs
    codeowner_acknowledgments:
      - \"@platform-team\"
";

const CODEOWNERS_VALID: &str = "crates/legacy/** @platform-team\n";

#[test]
fn exempt_verify_empty_registry_exits_0() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), ".chassis/exemptions.yaml", EMPTY_REGISTRY);
    write(dir.path(), "CODEOWNERS", CODEOWNERS_VALID);

    let assert = chassis()
        .args(["--json", "exempt", "verify", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["ok"], Value::Bool(true));
    assert!(v["diagnostics"].is_array());
}

#[test]
fn exempt_verify_single_active_entry_exits_0() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        ".chassis/exemptions.yaml",
        SINGLE_ACTIVE_REGISTRY,
    );
    write(dir.path(), "CODEOWNERS", CODEOWNERS_VALID);

    chassis()
        .args(["exempt", "verify", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);
}

#[test]
fn exempt_verify_expired_entry_exits_3() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), ".chassis/exemptions.yaml", EXPIRED_REGISTRY);
    write(dir.path(), "CODEOWNERS", CODEOWNERS_VALID);

    let assert = chassis()
        .args(["--json", "exempt", "verify", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::EXEMPT_VIOLATION);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["ok"], Value::Bool(false));
    let diags = v["diagnostics"].as_array().expect("diagnostics array");
    let any_expired = diags
        .iter()
        .any(|d| d["ruleId"] == "CH-EXEMPT-EXPIRED" && d["severity"] == "error");
    assert!(
        any_expired,
        "expected a CH-EXEMPT-EXPIRED error diagnostic; got {:?}",
        diags
    );
}

#[test]
fn exempt_verify_missing_registry_exits_66() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CODEOWNERS", CODEOWNERS_VALID);

    let assert = chassis()
        .args(["--json", "exempt", "verify", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::MISSING_FILE);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["error"]["code"], "CLI-MISSING-FILE");
}

#[test]
fn exempt_verify_malformed_registry_yaml_exits_65() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        ".chassis/exemptions.yaml",
        "version: 2\n\tentries: []\n",
    );
    write(dir.path(), "CODEOWNERS", CODEOWNERS_VALID);

    chassis()
        .args(["--json", "exempt", "verify", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::MALFORMED_INPUT);
}

#[test]
fn exempt_verify_schema_invalid_registry_exits_65() {
    // Parseable as YAML but `version: "two"` doesn't deserialize into Registry.
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        ".chassis/exemptions.yaml",
        "version: two\nentries: []\n",
    );
    write(dir.path(), "CODEOWNERS", CODEOWNERS_VALID);

    chassis()
        .args(["--json", "exempt", "verify", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::MALFORMED_INPUT);
}
