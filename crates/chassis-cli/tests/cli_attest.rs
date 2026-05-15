mod common;

use chassis_core::artifact::{validate_dsse_envelope_value, validate_in_toto_statement_value};
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

use common::{
    chassis, exit, git_init_with_initial_commit, write, write_keypair, VALID_LIBRARY_YAML,
};

fn stdout(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(out.get_output().stdout.clone()).expect("utf8 stdout")
}

/// Build a minimal signable repo: git history, a CONTRACT.yaml, and a keypair.
/// Returns the tempdir plus the envelope output path.
fn signable_repo() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let _ = write(dir.path(), "CONTRACT.yaml", VALID_LIBRARY_YAML);
    let (_priv, _pub) = write_keypair(dir.path());
    git_init_with_initial_commit(dir.path());
    let out = dir.path().join("release-gate.dsse.json");
    (dir, out)
}

#[test]
fn attest_sign_happy_path_writes_dsse_envelope_validating_against_schema() {
    let (dir, out) = signable_repo();

    chassis()
        .args(["attest", "sign", "--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let raw = fs::read_to_string(&out).expect("envelope written to disk");
    let env: Value = serde_json::from_str(&raw).expect("DSSE envelope is JSON");
    validate_dsse_envelope_value(&env)
        .expect("sign output must validate against the canonical DSSE envelope schema");
}

#[test]
fn attest_sign_json_returns_machine_readable_summary() {
    let (dir, out) = signable_repo();

    let assert = chassis()
        .args(["--json", "attest", "sign", "--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON summary");
    assert_eq!(v["ok"], Value::Bool(true));
    assert!(v["sha256"].is_string(), "summary must carry sha256 digest");
    assert!(v["out"].is_string(), "summary must echo the output path");
}

#[test]
// @claim chassis.attest-key-policy-fail-closed
fn attest_sign_missing_private_key_exits_66() {
    let (dir, out) = signable_repo();
    fs::remove_file(dir.path().join(".chassis/keys/release.priv")).expect("priv key cleanup");

    let assert = chassis()
        .args(["--json", "attest", "sign", "--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::MISSING_FILE);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    // The fail-closed key resolver maps a missing release key to
    // CH-ATTEST-KEY-MISSING (not the generic CLI-MISSING-FILE) so callers can
    // see *why* the file matters — ephemeral signing must be an explicit
    // opt-in, not a silent fallback.
    assert_eq!(v["error"]["code"], "CH-ATTEST-KEY-MISSING");
}

#[test]
fn attest_sign_malformed_private_key_exits_65() {
    let (dir, out) = signable_repo();
    fs::write(dir.path().join(".chassis/keys/release.priv"), "not-hex").unwrap();

    chassis()
        .args(["--json", "attest", "sign", "--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::MALFORMED_INPUT);
}

#[test]
fn attest_verify_happy_path_round_trips_signed_envelope() {
    let (dir, out) = signable_repo();
    chassis()
        .args(["attest", "sign", "--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let assert = chassis()
        .args(["--json", "attest", "verify"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let stmt: Value = serde_json::from_str(&stdout(&assert)).expect("Statement JSON");
    validate_in_toto_statement_value(&stmt)
        .expect("verify output must validate against in-toto Statement schema");
}

#[test]
fn attest_verify_missing_envelope_file_exits_66() {
    let (dir, _out) = signable_repo();

    chassis()
        .args(["--json", "attest", "verify"])
        .arg(dir.path().join("does-not-exist.dsse.json"))
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::MISSING_FILE);
}

#[test]
fn attest_verify_malformed_envelope_json_exits_65() {
    let (dir, out) = signable_repo();
    fs::write(&out, "{ not-json }").unwrap();

    chassis()
        .args(["--json", "attest", "verify"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::MALFORMED_INPUT);
}

#[test]
fn attest_verify_schema_invalid_envelope_exits_attest_verify_failed() {
    // Parseable JSON, but missing the `payload` field required by the DSSE
    // envelope schema. The CLI surfaces this as ATTEST_VERIFY_FAILED rather
    // than MALFORMED_INPUT — a structurally bogus envelope cannot be trusted.
    let (dir, out) = signable_repo();
    fs::write(
        &out,
        r#"{"payloadType":"application/vnd.in-toto+json","signatures":[]}"#,
    )
    .unwrap();

    chassis()
        .args(["--json", "attest", "verify"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::ATTEST_VERIFY_FAILED);
}

#[test]
fn attest_verify_tampered_payload_exits_6() {
    let (dir, out) = signable_repo();
    chassis()
        .args(["attest", "sign", "--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    // Replace the payload bytes with a different, valid-base64 string. The
    // signature no longer matches the PAE bytes, so verification must fail.
    let mut env: Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    env["payload"] = Value::String(
        base64_encode(br#"{"_type":"https://in-toto.io/Statement/v1","subject":[],"predicateType":"x","predicate":{}}"#),
    );
    fs::write(&out, serde_json::to_string_pretty(&env).unwrap()).unwrap();

    let assert = chassis()
        .args(["--json", "attest", "verify"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::ATTEST_VERIFY_FAILED);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON envelope");
    assert_eq!(v["ok"], Value::Bool(false));
    assert!(v["error"]["code"]
        .as_str()
        .unwrap()
        .starts_with("CH-ATTEST-"));
}

#[test]
fn attest_verify_wrong_public_key_exits_6() {
    let (dir, out) = signable_repo();
    chassis()
        .args(["attest", "sign", "--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    // Overwrite the public key with a fresh, unrelated keypair's verifying key.
    // `alt` must outlive the read; if it dropped at the end of the inner block
    // the temp dir would be deleted before we could read alt_pub.
    let alt = TempDir::new().unwrap();
    let (_priv, alt_pub) = write_keypair(alt.path());
    let alt_pub_hex = fs::read_to_string(&alt_pub).unwrap();
    fs::write(dir.path().join(".chassis/keys/release.pub"), alt_pub_hex).unwrap();

    chassis()
        .args(["--json", "attest", "verify"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::ATTEST_VERIFY_FAILED);
}

fn base64_encode(input: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.encode(input)
}

// -- key policy tests ---------------------------------------------------------

#[test]
fn attest_sign_with_explicit_private_key_passes() {
    // A signer that names its private key explicitly is release-grade: the
    // operator named the key, so the resulting attestation can be audited
    // against that named identity.
    let (dir, out) = signable_repo();
    let priv_path = dir.path().join(".chassis/keys/release.priv");

    let assert = chassis()
        .args(["--json", "attest", "sign", "--private-key"])
        .arg(&priv_path)
        .args(["--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON summary");
    assert_eq!(v["ok"], Value::Bool(true));
    assert_eq!(v["release_grade"], Value::Bool(true));
    assert!(
        v["public_key_fingerprint"]
            .as_str()
            .is_some_and(|s| s.len() == 64),
        "release-grade signing surfaces the 64-hex public-key fingerprint"
    );
}

#[test]
fn attest_sign_ephemeral_key_writes_non_release_grade_artifact() {
    // Ephemeral signing is an explicit opt-in. The on-disk envelope still
    // verifies, but the CLI surfaces release_grade=false and writes the
    // public key as a sibling so the demo verifier knows where to look.
    let (dir, out) = signable_repo();
    // Remove the default release key so we know the ephemeral path is what
    // actually fired.
    fs::remove_file(dir.path().join(".chassis/keys/release.priv")).unwrap();

    let assert = chassis()
        .args(["--json", "attest", "sign", "--ephemeral-key", "--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .code(exit::OK);

    let v: Value = serde_json::from_str(&stdout(&assert)).expect("JSON summary");
    assert_eq!(v["ok"], Value::Bool(true));
    assert_eq!(
        v["release_grade"],
        Value::Bool(false),
        "ephemeral signing must be flagged non-release-grade"
    );
    let pk_path = v["public_key_path"]
        .as_str()
        .expect("ephemeral path must surface its public key file")
        .to_string();
    let fp = v["public_key_fingerprint"]
        .as_str()
        .expect("ephemeral path must surface a fingerprint")
        .to_string();
    assert_eq!(fp.len(), 64);
    let pk_hex = fs::read_to_string(&pk_path).expect("ephemeral public key written");
    assert_eq!(pk_hex.trim(), fp, "fingerprint must match file contents");
}

#[test]
fn attest_sign_rejects_ephemeral_with_explicit_private_key() {
    // Clap should refuse the combination; "release-grade" and "throwaway"
    // are mutually exclusive by design.
    let (dir, out) = signable_repo();
    let priv_path = dir.path().join(".chassis/keys/release.priv");

    chassis()
        .args(["attest", "sign", "--ephemeral-key", "--private-key"])
        .arg(&priv_path)
        .args(["--out"])
        .arg(&out)
        .arg("--repo")
        .arg(dir.path())
        .assert()
        .failure();
}
