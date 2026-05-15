//! Test-side `@claim` sites paired with `contract_claim_markers`.

use chassis_core::contract_claim_markers;

// @claim chassis.archive-self-verifying
#[test]
fn test_claim_archive_self_verifying() {
    contract_claim_markers::trace_anchor_archive_self_verifying();
}

// @claim chassis.no-private-keys-tracked
#[test]
fn test_claim_no_private_keys_tracked() {
    contract_claim_markers::trace_anchor_no_private_keys_tracked();
}

// @claim chassis.trace-tree-sitter-parity
#[test]
fn test_claim_trace_treesitter_parity() {
    contract_claim_markers::trace_anchor_trace_treesitter_parity();
}

// @claim chassis.spec-kit-markdown-bridge
#[test]
fn test_claim_spec_kit_markdown_bridge() {
    contract_claim_markers::trace_anchor_spec_kit_markdown_bridge();
}

// @claim chassis.provenance-cosign-slsa
#[test]
fn test_claim_provenance_cosign_slsa() {
    contract_claim_markers::trace_anchor_provenance_cosign_slsa();
}

// @claim chassis.evidence-digest-roundtrip
#[test]
fn test_claim_evidence_digest_roundtrip() {
    contract_claim_markers::trace_anchor_evidence_digest_roundtrip();
}
