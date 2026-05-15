//! Deterministic Mermaid rendering for [`super::TraceGraph`].

use crate::trace::types::TraceGraph;

/// Produce a deterministic Mermaid `graph TD` listing all claim IDs (sorted).
pub fn render_mermaid(graph: &TraceGraph) -> String {
    let mut out = String::from("graph TD\n");
    for (id, _) in graph.claims.iter() {
        let safe = id.replace(['.', '-', ':'], "_");
        let safe_label = id.replace('"', "'");
        let line = format!("  cid_{safe}[\"{lbl}\"]", safe = safe, lbl = safe_label);
        out.push_str(&line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Claim;
    use crate::trace::types::ClaimContractKind;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    #[test]
    fn fixture_expected_mermaid() {
        let mut claims = BTreeMap::new();
        claims.insert(
            "demo.alpha".into(),
            crate::trace::types::ClaimNode {
                claim_id: "demo.alpha".into(),
                contract_path: "CONTRACT.yaml".into(),
                contract_kind: ClaimContractKind::Invariant,
                claim_record: Claim {
                    id: "demo.alpha".into(),
                    text: "x".into(),
                    test_linkage: None,
                },
                impl_sites: vec![],
                test_sites: vec![],
                adr_refs: vec![],
                active_exemptions: vec![],
            },
        );

        let g = TraceGraph {
            claims,
            orphan_sites: vec![],
            diagnostics: vec![],
        };

        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/trace-render/expected.mmd");
        let expected = fs::read_to_string(&path).expect("read expected.mmd");
        assert_eq!(render_mermaid(&g), expected);
    }
}
