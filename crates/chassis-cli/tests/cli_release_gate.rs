mod common;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chassis_core::artifact::{
    validate_dsse_envelope_value, validate_in_toto_statement_value, validate_release_gate_value,
};
use serde_json::Value;
use tempfile::TempDir;

use common::{
    chassis, exit, git_init_with_initial_commit, write, write_keypair, VALID_LIBRARY_YAML,
};

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

/// Pull the in-toto Statement from a DSSE envelope on disk, validate it
/// against the canonical schema, and return its predicate value. Kept for
/// tests that round-trip a written envelope through the canonical schema.
#[allow(dead_code)] // Reachable from test cases that opt into round-trip validation.
fn predicate_from_dsse(path: &std::path::Path) -> Value {
    let raw = std::fs::read_to_string(path).expect("DSSE envelope on disk");
    let env: Value = serde_json::from_str(&raw).expect("DSSE envelope JSON");
    validate_dsse_envelope_value(&env).expect("DSSE envelope schema valid");
    let payload_b64 = env["payload"].as_str().expect("payload b64");
    let payload_bytes = B64.decode(payload_b64).expect("payload decodes");
    let stmt: Value = serde_json::from_slice(&payload_bytes).expect("statement JSON");
    validate_in_toto_statement_value(&stmt).expect("in-toto statement schema valid");
    stmt["predicate"].clone()
}

fn traced_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    write(
        dir.path(),
        "crates/demo/src/lib.rs",
        r#"
// @claim cli.tests.alpha
pub fn alpha() -> bool { true }

// @claim cli.tests.edge.one
pub fn edge() -> bool { true }

#[test]
// @claim cli.tests.alpha
fn alpha_is_true() { assert!(alpha()); }

#[test]
// @claim cli.tests.edge.one
fn edge_is_true() { assert!(edge()); }
"#,
    );
    git_init_with_initial_commit(dir.path());
    dir
}

#[test]
fn release_gate_happy_path_outputs_end_to_end_summary() {
    let dir = traced_repo();

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], true);
    assert_eq!(v["verdict"], "pass");
    // Per-axis blocking flags and counters all read clean.
    assert_eq!(v["fail_on_drift"], true);
    assert_eq!(v["trace_failed"], false);
    assert_eq!(v["drift_failed"], false);
    assert_eq!(v["exemption_failed"], false);
    assert_eq!(v["attestation_failed"], false);
    assert_eq!(v["unsuppressed_blocking"], 0);
    assert_eq!(v["suppressed"], 0);
    assert_eq!(v["severity_overridden"], 0);
    assert_eq!(v["final_exit_code"], exit::OK);
    assert!(v["schema_fingerprint"]
        .as_str()
        .is_some_and(|s| s.len() == 64));
    assert!(v["git_commit"].as_str().is_some_and(|s| s.len() == 40));
    assert_eq!(v["contract_validation"]["invalid"], 0);
    assert_eq!(v["trace_summary"]["orphan_sites"], 0);
    assert_eq!(v["trace_summary"]["missing_impl"], 0);
    assert_eq!(v["trace_summary"]["missing_tests"], 0);
    assert_eq!(v["drift_summary"]["missing"], 0);

    let artifact_path = dir.path().join("release-gate.json");
    assert!(artifact_path.is_file());
    let predicate: Value =
        serde_json::from_str(&std::fs::read_to_string(&artifact_path).unwrap()).unwrap();
    validate_release_gate_value(&predicate).expect("written predicate matches schema");
    // The CLI JSON, the artifact predicate, and the predicate's
    // commands_run.exit_code must all name the same final exit code.
    assert_eq!(predicate["verdict"], "pass");
    assert_eq!(predicate["final_exit_code"], exit::OK);
    let cmd = &predicate["commands_run"][0];
    assert_eq!(cmd["exit_code"], exit::OK);
}

#[test]
fn release_gate_invalid_contract_fails_before_trace() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", "name: broken\nkind: library\n");

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::VALIDATE_FAILED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["contract_validation"]["invalid"], 1);
    assert!(v["artifact_path"].is_null());
}

#[test]
fn release_gate_orphan_claim_fails_trace_gate() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    write(
        dir.path(),
        "crates/demo/src/lib.rs",
        r#"
// @claim cli.tests.not-in-contract
pub fn orphan() {}
"#,
    );
    git_init_with_initial_commit(dir.path());

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::DRIFT_DETECTED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["trace_summary"]["orphan_sites"], 1);
}

#[test]
fn release_gate_fail_on_drift_rejects_missing_claim_sites() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    git_init_with_initial_commit(dir.path());

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::DRIFT_DETECTED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], false);
    assert!(v["drift_summary"]["missing"].as_u64().unwrap() >= 1);
    assert!(
        v["drift_summary"]["unsuppressed_blocking"]
            .as_u64()
            .unwrap()
            >= 1
    );
}

#[test]
fn release_gate_bad_exemption_fails() {
    let dir = traced_repo();
    write(
        dir.path(),
        ".chassis/exemptions.yaml",
        r#"version: 2
entries:
  - id: EX-2026-0001
    rule_id: CH-DRIFT-IMPL-MISSING
    reason: "Expired exemption must fail the release gate."
    owner: platform-team@docs.invalid
    created_at: "2024-01-01"
    expires_at: "2024-01-31"
    path: crates/demo/src/lib.rs
    codeowner_acknowledgments:
      - "@platform-team"
"#,
    );
    write(dir.path(), "CODEOWNERS", "crates/demo/** @platform-team\n");

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::EXEMPT_VIOLATION);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("release-gate JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["exemption_summary"]["errors"], 1);
}

// -- key policy tests ---------------------------------------------------------

#[test]
fn release_gate_attest_with_no_key_fails_closed() {
    // The traced repo here intentionally has no .chassis/keys/release.priv.
    // Asking for --attest in that state must fail with CH-ATTEST-KEY-MISSING
    // rather than fabricating a throwaway keypair on the fly — that was the
    // old behavior the new policy explicitly forbids.
    let dir = traced_repo();
    assert!(
        !dir.path().join(".chassis/keys/release.priv").exists(),
        "fixture precondition: no release key present"
    );

    let assert = chassis()
        .args([
            "--json",
            "release-gate",
            "--fail-on-drift",
            "--attest",
            "--repo",
        ])
        .arg(dir.path())
        .assert()
        .code(exit::MISSING_FILE);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["error"]["code"], "CH-ATTEST-KEY-MISSING");

    // No DSSE envelope was written — the gate cannot lie about being signed.
    assert!(!dir.path().join("release-gate.dsse").exists());
}

#[test]
fn release_gate_attest_with_release_key_passes_release_grade() {
    let dir = traced_repo();
    let (_priv, _pub) = write_keypair(dir.path());

    let assert = chassis()
        .args([
            "--json",
            "release-gate",
            "--fail-on-drift",
            "--attest",
            "--repo",
        ])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["ok"], true);
    assert_eq!(v["attestation_release_grade"], true);
    assert!(v["attestation_path"].is_string());
    assert!(
        v["attestation_public_key_fingerprint"]
            .as_str()
            .is_some_and(|s| s.len() == 64),
        "release-grade path must surface the verifying-key fingerprint"
    );
}

#[test]
fn release_gate_attest_with_explicit_private_key_passes() {
    let dir = traced_repo();
    let (priv_path, _pub) = write_keypair(dir.path());
    // Move the keypair out of the conventional location so the test cannot
    // accidentally pick it up by default.
    let alt_priv = dir.path().join("custom/release.priv");
    std::fs::create_dir_all(alt_priv.parent().unwrap()).unwrap();
    std::fs::rename(&priv_path, &alt_priv).unwrap();
    assert!(!priv_path.exists());

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--attest"])
        .args(["--private-key"])
        .arg(&alt_priv)
        .args(["--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["ok"], true);
    assert_eq!(v["attestation_release_grade"], true);
}

#[test]
fn release_gate_attest_ephemeral_key_marks_non_release_grade() {
    let dir = traced_repo();
    // Deliberately leave .chassis/keys/release.priv absent — ephemeral is the
    // only path that should succeed here.
    assert!(!dir.path().join(".chassis/keys/release.priv").exists());

    let assert = chassis()
        .args([
            "--json",
            "release-gate",
            "--fail-on-drift",
            "--attest",
            "--ephemeral-key",
            "--repo",
        ])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["ok"], true);
    assert_eq!(
        v["attestation_release_grade"], false,
        "ephemeral signing must surface a non-release-grade flag in CLI output"
    );
    let pk_path = v["attestation_public_key_path"]
        .as_str()
        .expect("ephemeral path must surface a sibling public-key file");
    assert!(
        std::path::Path::new(pk_path).exists(),
        "ephemeral public key was written to {pk_path}"
    );
    let fp = v["attestation_public_key_fingerprint"]
        .as_str()
        .expect("fingerprint must be present");
    assert_eq!(fp.len(), 64);
}

#[test]
fn release_gate_attestation_verify_passes_for_correct_key() {
    let dir = traced_repo();
    let (_priv, pub_path) = write_keypair(dir.path());

    chassis()
        .args([
            "--json",
            "release-gate",
            "--fail-on-drift",
            "--attest",
            "--repo",
        ])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let dsse_path = dir.path().join("release-gate.dsse");
    chassis()
        .args(["--json", "attest", "verify"])
        .arg(&dsse_path)
        .args(["--public-key"])
        .arg(pub_path)
        .args(["--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);
}

#[test]
fn release_gate_attestation_tamper_fails_verify() {
    let dir = traced_repo();
    let (_priv_path, pub_path) = write_keypair(dir.path());

    chassis()
        .args([
            "--json",
            "release-gate",
            "--fail-on-drift",
            "--attest",
            "--repo",
        ])
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let dsse_path = dir.path().join("release-gate.dsse");
    let mut env: Value =
        serde_json::from_str(&std::fs::read_to_string(&dsse_path).unwrap()).expect("DSSE JSON");
    env["payload"] = Value::String(B64.encode(br#"{"tampered":true}"#));
    std::fs::write(&dsse_path, serde_json::to_string_pretty(&env).unwrap()).unwrap();

    chassis()
        .args(["--json", "attest", "verify"])
        .arg(&dsse_path)
        .args(["--public-key"])
        .arg(pub_path)
        .args(["--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::ATTEST_VERIFY_FAILED);
}

// -- signed-predicate honesty tests ------------------------------------------

#[test]
fn release_gate_signed_predicate_validates_against_schema() {
    let dir = traced_repo();
    let (_priv, _pub) = write_keypair(dir.path());

    let assert = chassis()
        .args([
            "--json",
            "release-gate",
            "--fail-on-drift",
            "--attest",
            "--repo",
        ])
        .arg(dir.path())
        .assert()
        .code(exit::OK);
    let cli: Value = serde_json::from_str(&stdout(&assert)).expect("CLI JSON");

    let dsse_path = dir.path().join("release-gate.dsse");
    let predicate = predicate_from_dsse(&dsse_path);
    validate_release_gate_value(&predicate).expect("signed predicate matches schema");

    // The signed predicate must report the same verdict / per-axis flags /
    // final_exit_code as the CLI envelope. A verifier reading only the signed
    // artifact reaches the same conclusion as the CLI text.
    assert_eq!(predicate["verdict"], cli["verdict"]);
    assert_eq!(predicate["fail_on_drift"], cli["fail_on_drift"]);
    assert_eq!(predicate["trace_failed"], cli["trace_failed"]);
    assert_eq!(predicate["drift_failed"], cli["drift_failed"]);
    assert_eq!(predicate["exemption_failed"], cli["exemption_failed"]);
    assert_eq!(predicate["attestation_failed"], cli["attestation_failed"]);
    assert_eq!(
        predicate["unsuppressed_blocking"],
        cli["unsuppressed_blocking"]
    );
    assert_eq!(predicate["suppressed"], cli["suppressed"]);
    assert_eq!(predicate["severity_overridden"], cli["severity_overridden"]);
    assert_eq!(predicate["final_exit_code"], cli["final_exit_code"]);
}

#[test]
fn release_gate_signed_predicate_carries_blocking_reasons_when_failing() {
    // CONTRACT.yaml declares claims but no impl/test sites exist - trace and
    // drift both fail. Confirm the signed predicate names which axes blocked.
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    let (_priv, _pub) = write_keypair(dir.path());
    git_init_with_initial_commit(dir.path());

    let assert = chassis()
        .args([
            "--json",
            "release-gate",
            "--fail-on-drift",
            "--attest",
            "--repo",
        ])
        .arg(dir.path())
        .assert()
        .code(exit::DRIFT_DETECTED);
    let cli: Value = serde_json::from_str(&stdout(&assert)).expect("CLI JSON");
    assert_eq!(cli["verdict"], "fail");

    let predicate = predicate_from_dsse(&dir.path().join("release-gate.dsse"));
    validate_release_gate_value(&predicate).expect("signed predicate matches schema");
    assert_eq!(predicate["verdict"], "fail");
    let trace_failed = predicate["trace_failed"].as_bool().unwrap();
    let drift_failed = predicate["drift_failed"].as_bool().unwrap();
    assert!(
        trace_failed || drift_failed,
        "signed predicate must name at least one blocking axis when verdict=fail"
    );
    assert_eq!(predicate["final_exit_code"], exit::DRIFT_DETECTED);
    assert_eq!(
        predicate["commands_run"][0]["exit_code"],
        exit::DRIFT_DETECTED
    );
}

#[test]
fn release_gate_final_exit_code_matches_process_exit_code() {
    // Cover every shipped non-internal release-gate exit through the
    // pipeline. The CLI JSON's `final_exit_code`, the on-disk predicate's
    // top-level `final_exit_code`, and its `commands_run[0].exit_code` must
    // all equal the process exit code.
    struct Case<F: FnOnce(&std::path::Path)> {
        setup: F,
        expected_code: i32,
        emits_predicate: bool,
    }

    fn run_case<F: FnOnce(&std::path::Path)>(case: Case<F>) {
        let dir = TempDir::new().unwrap();
        (case.setup)(dir.path());

        let assert = chassis()
            .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
            .arg(dir.path())
            .assert()
            .code(case.expected_code);
        let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");

        assert_eq!(
            v["final_exit_code"], case.expected_code,
            "CLI JSON final_exit_code must equal the process exit code"
        );

        if case.emits_predicate {
            let predicate: Value = serde_json::from_str(
                &std::fs::read_to_string(dir.path().join("release-gate.json")).unwrap(),
            )
            .unwrap();
            validate_release_gate_value(&predicate).expect("on-disk predicate matches schema");
            assert_eq!(predicate["final_exit_code"], case.expected_code);
            assert_eq!(
                predicate["commands_run"][0]["exit_code"], case.expected_code,
                "commands_run.exit_code in the artifact must reflect the real final outcome"
            );
        }
    }

    // OK: clean traced repo.
    run_case(Case {
        setup: |p| {
            write(p, "CONTRACT.yaml", VALID_LIBRARY_YAML);
            write(
                p,
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
            git_init_with_initial_commit(p);
        },
        expected_code: exit::OK,
        emits_predicate: true,
    });

    // DRIFT_DETECTED: claims declared with no impl/test sites.
    run_case(Case {
        setup: |p| {
            write(p, "CONTRACT.yaml", VALID_LIBRARY_YAML);
            git_init_with_initial_commit(p);
        },
        expected_code: exit::DRIFT_DETECTED,
        emits_predicate: true,
    });

    // EXEMPT_VIOLATION: traced repo + expired exemption.
    run_case(Case {
        setup: |p| {
            write(p, "CONTRACT.yaml", VALID_LIBRARY_YAML);
            write(
                p,
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
            write(
                p,
                ".chassis/exemptions.yaml",
                r#"version: 2
entries:
  - id: EX-2026-0001
    rule_id: CH-DRIFT-IMPL-MISSING
    reason: "Expired exemption."
    owner: platform-team@docs.invalid
    created_at: "2024-01-01"
    expires_at: "2024-01-31"
    path: crates/demo/src/lib.rs
    codeowner_acknowledgments:
      - "@platform-team"
"#,
            );
            write(p, "CODEOWNERS", "crates/demo/** @platform-team\n");
            git_init_with_initial_commit(p);
        },
        expected_code: exit::EXEMPT_VIOLATION,
        emits_predicate: true,
    });

    // VALIDATE_FAILED: invalid contract - short-circuits before predicate.
    run_case(Case {
        setup: |p| {
            write(p, "CONTRACT.yaml", "name: broken\nkind: library\n");
        },
        expected_code: exit::VALIDATE_FAILED,
        emits_predicate: false,
    });
}

#[test]
fn release_gate_valid_suppression_is_counted() {
    // A drift diagnostic (CH-DRIFT-IMPL-MISSING) is suppressed by an active
    // global exemption. The release-gate predicate's `suppressed` counter
    // moves up; `unsuppressed_blocking` stays at zero. The verdict must
    // therefore be pass with no per-axis failure flags lit.
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    write(dir.path(), "CODEOWNERS", "* @platform-team\n");
    write(
        dir.path(),
        ".chassis/exemptions.yaml",
        r#"version: 2
allow_global: true
entries:
  - id: EX-2026-2001
    rule_id: CH-DRIFT-IMPL-MISSING
    reason: "Sandboxed demo: implementation files are intentionally absent."
    owner: platform-team@docs.invalid
    created_at: "2026-05-01"
    expires_at: "2026-07-30"
    path: "**"
    allow_global: true
    status: active
    codeowner_acknowledgments:
      - "@platform-team"
"#,
    );
    git_init_with_initial_commit(dir.path());
    // Trace shape comes from the working tree, so writing the @claim sites
    // *after* the initial commit gives us claims with impl/test sites
    // (trace_failed=false) whose backing files have no git history. Drift
    // then fires CH-DRIFT-IMPL-MISSING, which the global exemption
    // suppresses.
    write(
        dir.path(),
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

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);
    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");

    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["drift_failed"], false);
    assert_eq!(v["exemption_failed"], false);
    assert_eq!(v["unsuppressed_blocking"], 0);
    assert!(
        v["suppressed"].as_u64().unwrap() >= 1,
        "active matching exemption must increment suppressed counter, got {v}"
    );
    // Audit trail: every suppression action produces a CH-EXEMPT-APPLIED info
    // diagnostic. Counting these is the proof that we did not silently drop a
    // finding — we recorded *why* it stopped blocking.
    assert!(
        v["exemption_summary"]["audit"].as_u64().unwrap() >= 1,
        "suppression must leave an audit trail (CH-EXEMPT-APPLIED count), got {v}"
    );
    assert_eq!(
        v["exemption_summary"]["audit"], v["suppressed"],
        "one audit entry per suppressed finding",
    );

    let predicate: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("release-gate.json")).unwrap(),
    )
    .unwrap();
    validate_release_gate_value(&predicate).expect("predicate matches schema");
    assert_eq!(predicate["suppressed"], v["suppressed"]);
}

#[test]
fn release_gate_severity_override_is_counted() {
    // The exemption downgrades CH-DRIFT-IMPL-MISSING from error to info.
    // The diagnostic stays visible (downgraded) but no longer blocks; the
    // release-gate predicate's `severity_overridden` counter advances.
    let dir = TempDir::new().unwrap();
    write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    write(dir.path(), "CODEOWNERS", "* @platform-team\n");
    write(
        dir.path(),
        ".chassis/exemptions.yaml",
        r#"version: 2
allow_global: true
entries:
  - id: EX-2026-2002
    rule_id: CH-DRIFT-IMPL-MISSING
    reason: "Sandboxed demo: downgrade implementation-missing drift to info."
    owner: platform-team@docs.invalid
    created_at: "2026-05-01"
    expires_at: "2026-07-30"
    path: "**"
    allow_global: true
    status: active
    severity_override: info
    codeowner_acknowledgments:
      - "@platform-team"
"#,
    );
    git_init_with_initial_commit(dir.path());
    // See note in `release_gate_valid_suppression_is_counted`: writing the
    // @claim sites after the initial commit produces trace-clean +
    // drift-noisy state that the exemption can downgrade.
    write(
        dir.path(),
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

    let assert = chassis()
        .args(["--json", "release-gate", "--fail-on-drift", "--repo"])
        .arg(dir.path())
        .assert()
        .code(exit::OK);
    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");

    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["drift_failed"], false);
    assert_eq!(v["unsuppressed_blocking"], 0);
    assert!(
        v["severity_overridden"].as_u64().unwrap() >= 1,
        "active severity_override exemption must increment counter, got {v}"
    );
    // Severity-override is the other audited action: the finding stays
    // visible (downgraded) and a CH-EXEMPT-APPLIED audit entry is emitted.
    assert!(
        v["exemption_summary"]["audit"].as_u64().unwrap() >= 1,
        "severity downgrade must leave an audit trail (CH-EXEMPT-APPLIED count), got {v}"
    );
    assert_eq!(
        v["exemption_summary"]["audit"], v["severity_overridden"],
        "one audit entry per overridden finding",
    );

    let predicate: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("release-gate.json")).unwrap(),
    )
    .unwrap();
    validate_release_gate_value(&predicate).expect("predicate matches schema");
    assert_eq!(predicate["severity_overridden"], v["severity_overridden"]);
}
