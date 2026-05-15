use std::path::Path;

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn cli_help_lists_core_commands() {
    let mut cmd = Command::cargo_bin("chassis").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(contains("validate"))
        .stdout(contains("trace"))
        .stdout(contains("drift"))
        .stdout(contains("attest"));
}

#[test]
fn validate_root_contract_yaml() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cmd = Command::cargo_bin("chassis").unwrap();
    cmd.current_dir(&repo)
        .args(["validate", "CONTRACT.yaml", "--repo"])
        .arg(&repo);
    cmd.assert().success();
}
