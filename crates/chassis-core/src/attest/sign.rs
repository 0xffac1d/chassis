#![forbid(unsafe_code)]

//! DSSE envelope (Ed25519) over an in-toto JSON payload.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use super::assemble::Statement;
use super::AttestError;

pub const CH_ATTEST_SIGN_FAILED: &str = "CH-ATTEST-SIGN-FAILED";
pub const CH_ATTEST_VERIFY_FAILED: &str = "CH-ATTEST-VERIFY-FAILED";
pub const CH_ATTEST_SUBJECT_MISMATCH: &str = "CH-ATTEST-SUBJECT-MISMATCH";
pub const CH_ATTEST_NOT_FOUND: &str = "CH-ATTEST-NOT-FOUND";

/// `payloadType` for in-toto Statement bytes (`application/vnd.in-toto+json`).
pub const PAYLOAD_TYPE_IN_TOTO_JSON: &str = "application/vnd.in-toto+json";

/// Pre-Authentication Encoding for DSSE (same byte layout as `DSSEv1` spec).
pub fn dsse_pae(payload_type: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"DSSEv1 ");
    out.extend_from_slice(payload_type.len().to_string().as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload_type.as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload.len().to_string().as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload);
    out
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DsseEnvelope {
    pub payload: String,
    #[serde(rename = "payloadType")]
    pub payload_type: String,
    pub signatures: Vec<DsseSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DsseSignature {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyid: Option<String>,
    pub sig: String,
}

pub fn generate_keypair() -> SigningKey {
    let mut rng = OsRng;
    SigningKey::generate(&mut rng)
}

pub fn verifying_key_for(signing: &SigningKey) -> VerifyingKey {
    signing.verifying_key()
}

/// 64 hex chars (32 bytes) for a `VerifyingKey`.
pub fn verifying_key_from_hex(s: &str) -> Result<VerifyingKey, String> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", t.len()));
    }
    let mut b = [0u8; 32];
    for i in 0..32 {
        b[i] = u8::from_str_radix(&t[i * 2..i * 2 + 2], 16)
            .map_err(|_| "invalid hex digit".to_string())?;
    }
    VerifyingKey::from_bytes(&b).map_err(|e| e.to_string())
}

/// 64 hex chars (32 bytes) for a [`SigningKey`] secret scalar.
pub fn signing_key_from_hex(s: &str) -> Result<SigningKey, String> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.len() != 64 {
        return Err(format!(
            "expected 64 hex chars for secret key, got {}",
            t.len()
        ));
    }
    let mut b = [0u8; 32];
    for i in 0..32 {
        b[i] = u8::from_str_radix(&t[i * 2..i * 2 + 2], 16)
            .map_err(|_| "invalid hex digit".to_string())?;
    }
    Ok(SigningKey::from_bytes(&b))
}

pub fn sign_statement(stmt: &Statement, key: &SigningKey) -> Result<DsseEnvelope, AttestError> {
    let payload_bytes = serde_json::to_vec(stmt).map_err(|e| AttestError::Json(e.to_string()))?;
    let pae = dsse_pae(PAYLOAD_TYPE_IN_TOTO_JSON, &payload_bytes);
    let sig = key.sign(&pae);
    Ok(DsseEnvelope {
        payload: B64.encode(&payload_bytes),
        payload_type: PAYLOAD_TYPE_IN_TOTO_JSON.to_string(),
        signatures: vec![DsseSignature {
            keyid: None,
            sig: B64.encode(sig.to_bytes()),
        }],
    })
}

pub fn verify_envelope(
    envelope: &DsseEnvelope,
    public_key: &VerifyingKey,
) -> Result<Statement, AttestError> {
    if envelope.payload_type != PAYLOAD_TYPE_IN_TOTO_JSON {
        return Err(AttestError::Sign(CH_ATTEST_VERIFY_FAILED.to_string()));
    }
    let payload_bytes = B64
        .decode(envelope.payload.as_bytes())
        .map_err(|e| AttestError::Sign(format!("{CH_ATTEST_VERIFY_FAILED}: base64 {e}")))?;
    let pae = dsse_pae(&envelope.payload_type, &payload_bytes);
    let sig_b64 = envelope
        .signatures
        .first()
        .ok_or_else(|| AttestError::Sign(CH_ATTEST_VERIFY_FAILED.to_string()))?;
    let sig_bytes = B64
        .decode(sig_b64.sig.as_bytes())
        .map_err(|e| AttestError::Sign(format!("{CH_ATTEST_VERIFY_FAILED}: sig {e}")))?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| AttestError::Sign(CH_ATTEST_VERIFY_FAILED.to_string()))?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_array);
    public_key
        .verify(&pae, &sig)
        .map_err(|_| AttestError::Sign(CH_ATTEST_VERIFY_FAILED.to_string()))?;
    let stmt: Statement =
        serde_json::from_slice(&payload_bytes).map_err(|e| AttestError::Json(e.to_string()))?;
    let v = serde_json::to_value(&stmt).map_err(|e| AttestError::Json(e.to_string()))?;
    super::assemble::validate_statement_json(&v).map_err(AttestError::StatementSchema)?;
    Ok(stmt)
}

pub fn verify_subject_matches_repo(
    stmt: &Statement,
    repo_root: &std::path::Path,
) -> Result<(), AttestError> {
    let fp = crate::fingerprint::compute(repo_root)?;
    let digest = stmt.subject.first().map(|s| s.digest.sha256.as_str());
    if digest == Some(fp.as_str()) {
        Ok(())
    } else {
        Err(AttestError::Sign(CH_ATTEST_SUBJECT_MISMATCH.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::drift::report::{DriftReport, DriftSummaryCounts};
    use crate::trace::types::TraceGraph;

    use std::path::Path;

    #[test]
    fn sign_verify_round_trip() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let sk = generate_keypair();
        let vk = verifying_key_for(&sk);

        let stmt = crate::attest::assemble::assemble(
            &repo,
            &TraceGraph {
                claims: Default::default(),
                orphan_sites: vec![],
                diagnostics: vec![],
            },
            &DriftReport {
                version: 1,
                summary: DriftSummaryCounts {
                    stale: 0,
                    abandoned: 0,
                    missing: 0,
                },
                diagnostics: vec![],
            },
            None,
            vec![],
            Utc::now(),
        )
        .expect("assemble");

        let env = sign_statement(&stmt, &sk).expect("sign");
        let back = verify_envelope(&env, &vk).expect("verify");
        assert_eq!(back, stmt);
    }

    #[test]
    fn tampered_payload_fails_verify() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let sk = generate_keypair();
        let vk = verifying_key_for(&sk);
        let stmt = crate::attest::assemble::assemble(
            &repo,
            &TraceGraph {
                claims: Default::default(),
                orphan_sites: vec![],
                diagnostics: vec![],
            },
            &DriftReport {
                version: 1,
                summary: DriftSummaryCounts {
                    stale: 0,
                    abandoned: 0,
                    missing: 0,
                },
                diagnostics: vec![],
            },
            None,
            vec![],
            Utc::now(),
        )
        .expect("assemble");
        let mut env = sign_statement(&stmt, &sk).expect("sign");
        env.payload = B64.encode(b"not-json");
        assert!(verify_envelope(&env, &vk).is_err());
    }
}
