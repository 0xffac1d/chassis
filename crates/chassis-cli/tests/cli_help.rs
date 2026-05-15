mod common;

use common::chassis;
use predicates::str::contains;

#[test]
fn top_level_help_lists_every_stable_subcommand() {
    chassis()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("validate"))
        .stdout(contains("diff"))
        .stdout(contains("trace"))
        .stdout(contains("drift"))
        .stdout(contains("export"))
        .stdout(contains("spec-index"))
        .stdout(contains("exempt"))
        .stdout(contains("release-gate"))
        .stdout(contains("attest"));
}

#[test]
fn top_level_help_documents_exit_codes() {
    chassis()
        .arg("--help")
        .assert()
        .success()
        // The long help renders the exit-code table; smoke-test a few rungs.
        .stdout(contains("Exit codes"))
        .stdout(contains("validate failed"))
        .stdout(contains("diff detected"))
        .stdout(contains("drift detected"))
        .stdout(contains("attest verify failed"));
}

#[test]
fn attest_help_lists_sign_and_verify() {
    chassis()
        .args(["attest", "--help"])
        .assert()
        .success()
        .stdout(contains("sign"))
        .stdout(contains("verify"));
}

#[test]
fn exempt_help_lists_verify() {
    chassis()
        .args(["exempt", "--help"])
        .assert()
        .success()
        .stdout(contains("verify"));
}
