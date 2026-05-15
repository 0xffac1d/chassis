//! End-to-end policy check: the trace graph must accept the ADR-0023 grammar
//! and surface the rejected pre-ADR-0023 JSDoc form as `CH-TRACE-MALFORMED-CLAIM`
//! so a TypeScript claim written that way fails the gate instead of silently
//! disappearing from the graph.

use std::fs;
use std::path::PathBuf;

use chassis_core::diagnostic::Severity;
use chassis_core::trace::build_trace_graph;

const REJECTED_JSDOC_TS: &str = r#"
/** @claim demo.rejected */
export function rejected(): number {
  return 1;
}
"#;

const ACCEPTED_LINE_TS: &str = r#"
// @claim demo.accepted
export function accepted(): number {
  return 1;
}
"#;

const MIN_CONTRACT: &str = r#"
name: scratch
kind: library
version: "0.1.0"
purpose: "fixture for trace-graph policy test"
status: pre-alpha
since: "0.1.0"
assurance_level: declared
owner: chassis maintainers
exports:
  - path: "packages/scratch/src/mod.ts"
    kind: module
invariants:
  - id: demo.accepted
    text: "fixture invariant — accepted form"
edge_cases: []
"#;

fn scratch_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("chassis-trace-policy-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("packages/scratch/src")).unwrap();
    fs::write(root.join("CONTRACT.yaml"), MIN_CONTRACT).unwrap();
    fs::write(
        root.join("packages/scratch/src/accepted.ts"),
        ACCEPTED_LINE_TS,
    )
    .unwrap();
    fs::write(
        root.join("packages/scratch/src/rejected.ts"),
        REJECTED_JSDOC_TS,
    )
    .unwrap();
    root
}

#[test]
fn graph_admits_accepted_form_and_rejects_jsdoc() {
    let root = scratch_root();
    let g = build_trace_graph(&root).expect("trace graph builds");

    let accepted = g
        .claims
        .get("demo.accepted")
        .expect("accepted claim present");
    assert!(
        !accepted.impl_sites.is_empty(),
        "accepted `// @claim` site must land in graph: {accepted:?}"
    );

    let malformed: Vec<_> = g
        .diagnostics
        .iter()
        .filter(|d| d.rule_id == "CH-TRACE-MALFORMED-CLAIM" && d.severity == Severity::Error)
        .collect();
    assert!(
        !malformed.is_empty(),
        "rejected JSDoc form must produce CH-TRACE-MALFORMED-CLAIM at graph level"
    );

    let rejected_id_orphaned = g.claims.contains_key("demo.rejected")
        && g.claims["demo.rejected"]
            .impl_sites
            .iter()
            .any(|s| s.file.to_string_lossy().ends_with("rejected.ts"));
    assert!(
        !rejected_id_orphaned,
        "rejected JSDoc form must not contribute a site to the graph"
    );

    let _ = fs::remove_dir_all(&root);
}
