//! Scanner ingest + policy-input wiring (fixtures).
#![forbid(unsafe_code)]

use chassis_core::exports::{build_policy_input, ContractFact, ExemptionFacts, RepoFacts};
use chassis_core::scanner::{ingest_sarif_bytes, ScannerTool};
use chassis_core::trace::types::TraceGraph;
use serde_json::{json, Value};
use std::path::Path;

fn fixture_bytes(name: &str) -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/scanner")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn ingested_scanner_summary_rolls_into_policy_input_diagnostics() {
    let summary = ingest_sarif_bytes(ScannerTool::Semgrep, &fixture_bytes("semgrep-clean.sarif"))
        .expect("ingest clean sarif");
    assert_eq!(summary.tool, ScannerTool::Semgrep);

    let trace = TraceGraph {
        claims: Default::default(),
        orphan_sites: vec![],
        diagnostics: vec![],
    };
    let contract_doc = json!({
        "name": "demo",
        "kind": "library",
        "purpose": "p",
        "status": "stable",
        "since": "0.1.0",
        "version": "0.1.0",
        "assurance_level": "declared",
        "owner": "platform",
        "invariants": [],
        "edge_cases": []
    });
    let contract: ContractFact = chassis_core::exports::contract_fact(
        std::path::PathBuf::from("CONTRACT.yaml"),
        contract_doc,
    )
    .expect("contract fact");
    let repo = RepoFacts {
        root: ".".to_string(),
        git_commit: None,
        schema_fingerprint: None,
    };
    let input = build_policy_input(
        repo,
        vec![contract],
        &trace,
        chassis_core::drift::report::DriftSummaryCounts {
            stale: 0,
            abandoned: 0,
            missing: 0,
        },
        vec![],
        ExemptionFacts {
            registry: None,
            diagnostics: vec![],
        },
        None,
        vec![],
        vec![summary],
        false,
    );
    let v: Value = serde_json::to_value(&input).expect("policy input json");
    chassis_core::exports::validate_policy_input_value(&v).expect("policy validates");
}
