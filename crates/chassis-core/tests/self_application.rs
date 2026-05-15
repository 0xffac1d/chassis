//! Self-application: repo `CONTRACT.yaml` claims must be backed by `@claim` sites (ADR-0023).

use std::path::Path;

use chassis_core::trace::RULE_NOT_IN_CONTRACT;

#[test]
fn self_application_trace_complete() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let g = chassis_core::trace::build_trace_graph(&root).expect("trace graph");
    let required = [
        "chassis.fingerprint-matches",
        "chassis.schemas-self-contained",
        "chassis.contract-schema-kind-discriminated",
        "chassis.adr-frontmatter-valid",
        "chassis.no-silent-assurance-demotion",
    ];
    for id in required {
        let node = g
            .claims
            .get(id)
            .unwrap_or_else(|| panic!("missing claim node {id}"));
        assert!(
            !node.impl_sites.is_empty() || !node.test_sites.is_empty(),
            "claim {id} must have at least one @claim site (impl or test)"
        );
    }
    let bad: Vec<_> = g
        .diagnostics
        .iter()
        .filter(|d| d.rule_id == RULE_NOT_IN_CONTRACT)
        .filter(|d| {
            required
                .iter()
                .any(|id| d.message.contains(&format!("`{id}`")) || d.message.contains(id))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "unexpected orphan diagnostics for required claims: {bad:?}"
    );
}

#[test]
fn self_application_no_orphan_invariants_root_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let raw = std::fs::read_to_string(root.join("CONTRACT.yaml")).expect("CONTRACT.yaml");
    let v: serde_yaml::Value = serde_yaml::from_str(&raw).expect("yaml");
    let inv = v
        .get("invariants")
        .and_then(|x| x.as_sequence())
        .expect("invariants");
    let g = chassis_core::trace::build_trace_graph(&root).expect("trace graph");
    for row in inv {
        let id = row
            .get("id")
            .and_then(|x| x.as_str())
            .expect("invariant id");
        if id.starts_with("chassis.") {
            let node = g
                .claims
                .get(id)
                .unwrap_or_else(|| panic!("contract invariant {id} missing from graph"));
            assert!(
                !node.impl_sites.is_empty() || !node.test_sites.is_empty(),
                "invariant {id} must have ≥1 @claim site (impl or test)"
            );
        }
    }
}
