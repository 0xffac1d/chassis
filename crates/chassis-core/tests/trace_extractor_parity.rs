//! Regex vs tree-sitter trace backends must agree on claim sites for this repo.
#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::path::Path;

use chrono::Utc;

use chassis_core::trace::{build_trace_graph_at_with, ClaimSite, TraceExtractBackend, TraceGraph};

fn norm_sites(g: &TraceGraph) -> BTreeSet<String> {
    let mut s: BTreeSet<String> = BTreeSet::new();
    for node in g.claims.values() {
        for st in &node.impl_sites {
            s.insert(site_key(st));
        }
        for st in &node.test_sites {
            s.insert(site_key(st));
        }
    }
    s
}

fn site_key(c: &ClaimSite) -> String {
    format!(
        "{}:{}:{}:{:?}",
        c.file.display(),
        c.line,
        c.claim_id,
        c.kind
    )
}

#[test]
fn regex_and_tree_sitter_trace_parity_on_repo() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let reg = build_trace_graph_at_with(&root, Utc::now(), TraceExtractBackend::Regex)
        .expect("regex trace");
    let ts = build_trace_graph_at_with(&root, Utc::now(), TraceExtractBackend::TreeSitter)
        .expect("tree-sitter trace");
    assert_eq!(
        norm_sites(&reg),
        norm_sites(&ts),
        "claim sites must match between regex and tree-sitter extractors"
    );
}
