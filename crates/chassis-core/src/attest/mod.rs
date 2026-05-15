#![forbid(unsafe_code)]

//! Release-gate attestation: in-toto Statement + DSSE envelope (ADR-0022).

pub mod assemble;
pub mod predicate;
pub mod sign;

use std::fmt;

pub use assemble::{
    assemble, validate_statement_json, DigestSet, GateOutcome, Statement, SubjectDescriptor,
    PREDICATE_TYPE, STATEMENT_TYPE,
};
pub use predicate::{
    validate_release_gate_predicate, CommandRun, DriftSummary, ExemptSummary, ReleaseGatePredicate,
    TraceSummary, Verdict, CH_ATTEST_PREDICATE_INVALID,
};
pub use sign::{
    dsse_pae, generate_keypair, sign_statement, signing_key_from_hex, validate_dsse_envelope,
    validate_dsse_envelope_json, verify_envelope, verify_subject_matches_repo, verifying_key_for,
    verifying_key_from_hex, DsseEnvelope, DsseSignature, CH_ATTEST_ENVELOPE_SCHEMA,
    CH_ATTEST_NOT_FOUND, CH_ATTEST_SIGN_FAILED, CH_ATTEST_SUBJECT_MISMATCH,
    CH_ATTEST_VERIFY_FAILED, PAYLOAD_TYPE_IN_TOTO_JSON,
};

use crate::fingerprint::FingerprintError;

#[derive(Debug)]
pub enum AttestError {
    Fingerprint(FingerprintError),
    Git(String),
    Json(String),
    PredicateSchema(Vec<String>),
    StatementSchema(Vec<String>),
    EnvelopeSchema(Vec<String>),
    Sign(String),
}

impl fmt::Display for AttestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttestError::Fingerprint(e) => write!(f, "{e}"),
            AttestError::Git(s) => write!(f, "{s}"),
            AttestError::Json(s) => write!(f, "{s}"),
            AttestError::PredicateSchema(v) => write!(f, "{v:?}"),
            AttestError::StatementSchema(v) => write!(f, "{v:?}"),
            AttestError::EnvelopeSchema(v) => {
                write!(f, "{} {v:?}", sign::CH_ATTEST_ENVELOPE_SCHEMA)
            }
            AttestError::Sign(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for AttestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AttestError::Fingerprint(e) => Some(e),
            _ => None,
        }
    }
}

impl From<FingerprintError> for AttestError {
    fn from(value: FingerprintError) -> Self {
        AttestError::Fingerprint(value)
    }
}
