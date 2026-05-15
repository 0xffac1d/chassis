//! Shared helpers for chassis-cli integration tests.
//!
//! `mod common;` from each `tests/*.rs` file pulls these in. Cargo only treats
//! top-level `.rs` files under `tests/` as integration test crates, so this
//! subdirectory is invisible to the test runner.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::Command as AssertCmd;

pub const VALID_LIBRARY_YAML: &str = include_str!("../fixtures/library_minimal.yaml");
pub const VALID_LIBRARY_NEW_YAML: &str = include_str!("../fixtures/library_minimal_v2.yaml");
pub const BREAKING_VERSION_NOT_BUMPED_OLD: &str =
    include_str!("../fixtures/breaking_version_not_bumped_old.yaml");
pub const BREAKING_VERSION_NOT_BUMPED_NEW: &str =
    include_str!("../fixtures/breaking_version_not_bumped_new.yaml");
pub const SCHEMA_INVALID_YAML: &str = include_str!("../fixtures/schema_invalid.yaml");

/// Build a fresh `chassis` invocation.
pub fn chassis() -> AssertCmd {
    AssertCmd::cargo_bin("chassis").expect("binary built")
}

/// Write a file under `dir`, creating parents as needed.
pub fn write(dir: &Path, rel: &str, body: &str) -> PathBuf {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(&p, body).expect("write");
    p
}

/// `git init` + initial commit using the host `git` binary. Required for
/// commands that touch git history (drift, attest sign/verify).
pub fn git_init_with_initial_commit(dir: &Path) {
    // Use a deterministic identity so commit creation does not need a global
    // git config in the test environment.
    let env = [
        ("GIT_AUTHOR_NAME", "chassis-tests"),
        ("GIT_AUTHOR_EMAIL", "tests@chassis.invalid"),
        ("GIT_COMMITTER_NAME", "chassis-tests"),
        ("GIT_COMMITTER_EMAIL", "tests@chassis.invalid"),
    ];
    run_git(dir, &env, &["init", "--quiet", "--initial-branch=main"]);
    run_git(dir, &env, &["add", "-A"]);
    run_git(
        dir,
        &env,
        &["commit", "--quiet", "--allow-empty", "-m", "init"],
    );
}

/// Commit any currently-modified working tree contents.
pub fn git_commit_all(dir: &Path, message: &str) {
    let env = [
        ("GIT_AUTHOR_NAME", "chassis-tests"),
        ("GIT_AUTHOR_EMAIL", "tests@chassis.invalid"),
        ("GIT_COMMITTER_NAME", "chassis-tests"),
        ("GIT_COMMITTER_EMAIL", "tests@chassis.invalid"),
    ];
    run_git(dir, &env, &["add", "-A"]);
    run_git(dir, &env, &["commit", "--quiet", "-m", message]);
}

fn run_git(dir: &Path, env: &[(&str, &str)], args: &[&str]) {
    let mut c = Command::new("git");
    c.current_dir(dir);
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.args(args).output().expect("git on PATH");
    assert!(
        out.status.success(),
        "git {args:?} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Write a fixed Ed25519 keypair (priv hex, then pub hex) under `dir/.chassis/keys/`.
/// Uses chassis-core's keypair generator so the pair is canonical.
pub fn write_keypair(dir: &Path) -> (PathBuf, PathBuf) {
    use chassis_core::attest::sign::{generate_keypair, verifying_key_for};

    let sk = generate_keypair();
    let vk = verifying_key_for(&sk);
    let priv_hex: String = sk.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let pub_hex: String = vk.to_bytes().iter().map(|b| format!("{b:02x}")).collect();

    let priv_path = write(dir, ".chassis/keys/release.priv", &priv_hex);
    let pub_path = write(dir, ".chassis/keys/release.pub", &pub_hex);
    (priv_path, pub_path)
}

/// Stable CLI exit codes — mirror `main.rs::exit`. Centralized so test files
/// stay in lockstep with the binary when the surface evolves.
pub mod exit {
    pub const OK: i32 = 0;
    pub const VALIDATE_FAILED: i32 = 2;
    pub const EXEMPT_VIOLATION: i32 = 3;
    pub const DIFF_BREAKING: i32 = 4;
    pub const DRIFT_DETECTED: i32 = 5;
    pub const ATTEST_VERIFY_FAILED: i32 = 6;
    pub const MALFORMED_INPUT: i32 = 65;
    pub const MISSING_FILE: i32 = 66;
    pub const INTERNAL: i32 = 70;
}
