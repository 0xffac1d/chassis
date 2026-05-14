---
id: ADR-0009
title: Evidence chain — inference provenance, field_source / field_evidence / reviewer_acknowledgments
status: accepted
date: "2026-04-19"
enforces:
  - rule: EVIDENCE-FIELD-SOURCE-MISSING
    description: "An inference-generated CONTRACT.yaml field claims promotion_stage: verified but lacks an inference.field_source entry."
  - rule: EVIDENCE-PROMOTION-WITHOUT-ACK
    description: "promotion_stage advanced to reviewed or verified without a matching reviewer_acknowledgments entry."
  - rule: EVIDENCE-CONFIDENCE-INVALID
    description: "field_confidence value is outside [0, 1] or refers to a field not present in the manifest."
  - rule: EVIDENCE-CONTRADICTION-UNRESOLVED
    description: "inference.contradictions[] is non-empty for a manifest claiming promotion_stage: verified."
  - rule: PANIC-BUDGET-HARD-ZERO
    description: "A crate marked hard-zero panic budget (no panic!() / unwrap() / expect() / unreachable!() / todo!()) contains a regression."
  - rule: PANIC-BUDGET-REGRESSION
    description: "The number of panic! / unwrap calls in a crate increased vs the baseline in config/chassis/baselines/panic-budgets.json."
  - rule: PANIC-BUDGET-GROWTH-EXCEEDED
    description: "Panic-call growth exceeded the per-crate growth budget (typically 10% per release)."
  - rule: CLAIM-DRIFT-PROOF-MISSING-CMD
    description: "A numeric claim references a proof command that does not exist in the local environment."
  - rule: CLAIM-DRIFT-PROOF-FAILED
    description: "A claim's proof command exited non-zero."
  - rule: CLAIM-DRIFT-MISMATCH
    description: "A claim's asserted numeric value does not match the proof command's output."
  - rule: CLAIM-DRIFT-CROSS-DOC-DIVERGE
    description: "The same claim appears with different numeric values in two or more docs."
  - rule: AGENT-TELL-STUB-NEW
    description: "A new TODO / FIXME / unimplemented!() / placeholder marker was introduced in this change."
  - rule: AGENT-TELL-STUB-UNREGISTERED
    description: "An existing stub (TODO/FIXME) is not registered as a debt entry in the owning manifest."
applies_to:
  - "schemas/metadata/contract.schema.json"
  - "schemas/metadata/chassis-unit.schema.json"
  - "scripts/chassis/gates/panic_budget.py"
  - "scripts/chassis/gates/claim_drift.py"
  - "scripts/chassis/gates/agent_tell.py"
tags:
  - chassis
  - governance
  - evidence
  - inference
---

# ADR-0009: Evidence chain

## Context

Brownfield adoption is the dominant Chassis use case: a team adopts
Chassis on an existing codebase, runs `chassis infer-contracts` to
seed CONTRACT.yamls, and then incrementally promotes inferred fields
to verified. Without a structured evidence chain, the promotion
pipeline becomes "trust the reviewer" — fine for small teams, opaque
for large ones.

The schema's `inference` block (`field_source`, `field_evidence`,
`field_confidence`, `contradictions`, `reviewer_acknowledgments`)
captures the chain in machine-readable form. This ADR pins the
semantics and registers rule IDs for the gates that consume it.

This ADR also registers rule IDs for `panic_budget`, `claim_drift`,
and `agent_tell` gates because all three are "evidence-vs-claim"
gates: they assert that a claim (no panics, a documented number,
no stubs) matches reality.

## Decision

1. **`field_source` records provenance per field.** Values:
   `inferred` (machine-derived), `placeholder` (stub the reviewer
   should replace), `confirmed` (intentional default), `evidence_backed`
   (corroborated by code/test), `verified` (human-reviewed).
2. **`field_evidence` records the evidence chain per field.** Each
   entry is `{kind, path?, detail?, confidence?}` where `kind` is
   `source_file`, `import`, `symbol`, `comment`, `manifest`,
   `docstring`, `pattern`, `state_api`, `corroboration`,
   `use_statement`, `package_reference`, `project_reference`, or
   `using_statement`.
3. **`reviewer_acknowledgments` records human promotion steps.**
   Required when `promotion_stage` is `reviewed` or `verified`.
   Each entry: `{reviewer, stage, at}` (reviewer = email/handle,
   stage = `reviewed` | `verified`, at = ISO-8601 timestamp).
4. **`contradictions` are blocking for `verified`.** A non-empty
   `contradictions` array fails any attempt to advance `promotion_stage`
   to `verified`.
5. **`field_confidence` is informational.** Numeric 0..1 per field.
   Not enforced; consumed by `chassis stats` for reporting.

## Consequences

- The brownfield adoption funnel becomes auditable. A reviewer can
  see which fields are inferred vs verified, which evidence backs
  each, and which contradictions block promotion.
- The `panic_budget`, `claim_drift`, and `agent_tell` gates now
  resolve to specific rule IDs that map cleanly to evidence chain
  failures.
- Inference noise is bounded: tools produce evidence chains, not
  unverified assertions, and the schema makes the chain visible.

## Status

Accepted. Inference shape shipped in Milestone B+; this ADR
formalizes the model and registers the dependent gate rule IDs.
