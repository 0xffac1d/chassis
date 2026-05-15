#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::Claim;
use crate::diagnostic::Diagnostic;

/// Whether a traced site backs implementation behaviour or automated tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SiteKind {
    Impl,
    Test,
}

/// One occurrence of `@claim` in Rust or TS source before a backed site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimSite {
    pub file: PathBuf,
    pub line: usize,
    pub claim_id: String,
    pub kind: SiteKind,
}

/// Whether this claim originates from CONTRACT `invariants` vs `edge_cases`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimContractKind {
    Invariant,
    EdgeCase,
}

/// Contract-backed claim joined with traced sites plus optional references.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimNode {
    pub claim_id: String,
    pub contract_path: PathBuf,
    pub contract_kind: ClaimContractKind,
    pub claim_record: Claim,
    pub impl_sites: Vec<ClaimSite>,
    pub test_sites: Vec<ClaimSite>,
    pub adr_refs: Vec<String>,
    pub active_exemptions: Vec<String>,
}

/// Resolved trace graph keyed by canonical claim IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceGraph {
    pub claims: BTreeMap<String, ClaimNode>,
    pub orphan_sites: Vec<ClaimSite>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub enum TraceError {
    Io(std::io::Error),
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TraceError {}

impl From<std::io::Error> for TraceError {
    fn from(value: std::io::Error) -> Self {
        TraceError::Io(value)
    }
}
