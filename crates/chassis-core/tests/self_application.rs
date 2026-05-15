//! Self-application: repo `CONTRACT.yaml` claims must be backed by `@claim` sites (ADR-0023).

use std::path::Path;

use chassis_core::trace::RULE_NOT_IN_CONTRACT;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn self_application_trace_complete() {
    let root = repo_root();
    let g = chassis_core::trace::build_trace_graph(&root).expect("trace graph");
    let required = root_contract_claim_ids(&root);
    for id in &required {
        let node = g
            .claims
            .get(id)
            .unwrap_or_else(|| panic!("missing claim node {id}"));
        assert!(
            !node.impl_sites.is_empty(),
            "claim {id} must have at least one implementation @claim site"
        );
        assert!(
            !node.test_sites.is_empty(),
            "claim {id} must have at least one test @claim site"
        );
        assert!(
            !node.adr_refs.is_empty(),
            "claim {id} must be mentioned by at least one active ADR"
        );
        assert!(
            node.claim_record
                .test_linkage
                .as_ref()
                .is_some_and(|links| !links.is_empty()),
            "claim {id} must declare CONTRACT.yaml test_linkage"
        );
    }
    assert!(
        g.orphan_sites.is_empty(),
        "trace graph must not contain orphan @claim sites: {:?}",
        g.orphan_sites
    );
    let bad: Vec<_> = g
        .diagnostics
        .iter()
        .filter(|d| d.rule_id == RULE_NOT_IN_CONTRACT)
        .collect();
    assert!(bad.is_empty(), "unexpected orphan diagnostics: {bad:?}");
}

fn root_contract_claim_ids(root: &Path) -> Vec<String> {
    let raw = std::fs::read_to_string(root.join("CONTRACT.yaml")).expect("CONTRACT.yaml");
    let v: serde_yaml::Value = serde_yaml::from_str(&raw).expect("yaml");
    ["invariants", "edge_cases"]
        .iter()
        .flat_map(|field| {
            v.get(*field)
                .and_then(|x| x.as_sequence())
                .into_iter()
                .flatten()
        })
        .filter_map(|row| row.get("id").and_then(|x| x.as_str()))
        .filter(|id| id.starts_with("chassis."))
        .map(ToString::to_string)
        .collect()
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

// @claim chassis.reference-python-non-canonical
#[test]
fn reference_python_tree_is_not_canonical_trace_surface() {
    let root = repo_root();
    let g = chassis_core::trace::build_trace_graph(&root).expect("trace graph");
    let node = g
        .claims
        .get("chassis.reference-python-non-canonical")
        .expect("reference-python claim is declared");

    assert!(
        node.impl_sites
            .iter()
            .chain(node.test_sites.iter())
            .all(|site| !site.file.starts_with("reference/python-cli")),
        "reference/python-cli must remain reference material, not canonical trace source"
    );
}
