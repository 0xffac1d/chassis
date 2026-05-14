---
id: ADR-0002
title: Assurance ladder for CONTRACT.yaml claims (declared → coherent → verified → enforced → observed)
status: accepted
date: "2026-04-19"
enforces:
  - rule: ASSURANCE-LEVEL-INVALID
    description: "CONTRACT.yaml assurance_level value not in the canonical enum."
  - rule: ASSURANCE-PROMOTION-WITHOUT-EVIDENCE-COHERENT
    description: "assurance_level advanced to coherent without all depends_on resolving."
  - rule: ASSURANCE-PROMOTION-WITHOUT-EVIDENCE-VERIFIED
    description: "assurance_level advanced to verified without test_linkage[].confidence: high for every invariant."
  - rule: ASSURANCE-PROMOTION-WITHOUT-EVIDENCE-ENFORCED
    description: "assurance_level advanced to enforced without a Chassis gate listed in some ADR's enforces[] failing on violation."
  - rule: ASSURANCE-PROMOTION-WITHOUT-EVIDENCE-OBSERVED
    description: "assurance_level advanced to observed without runtime evidence batch entries referencing the claim_id."
applies_to:
  - "schemas/metadata/contract.schema.json"
  - "scripts/chassis/gates/assurance_promotion.py"
  - "**/CONTRACT.yaml"
tags:
  - chassis
  - governance
  - assurance
---

# ADR-0002: Assurance ladder for CONTRACT.yaml claims

## Context

Chassis-governed modules need an ordinal trust signal that scales with the
evidence backing each claim. A flat boolean ("validated yes/no") loses
information; a freeform string drifts. The five-tier ladder shipped in
`schemas/metadata/contract.schema.json` (`assurance_level` enum) was
deliberately ordinal-with-evidence-promotion, but the promotion semantics
were never documented as a decision record.

This ADR pins the semantics so the planned `assurance_promotion.py` gate
(C5 in the maturity plan) can enforce them consistently and so AI coding
agents have a stable contract for proposing promotion in PRs.

## Decision

The five tiers, in strict ordinal order:

1. **`declared`** — schema-valid only. The CONTRACT.yaml parses against
   `schemas/metadata/contract.schema.json`. No claim about evidence is
   made.
2. **`coherent`** — every `depends_on` entry resolves to an existing
   manifest in the repo, and `chassis coherence --format json` reports
   no unresolved upstream.
3. **`verified`** — every invariant and edge_case has a `test_linkage`
   entry with `validation_method: test` (or `runtime-assertion`) and
   `confidence: high`, and the referenced `test_file` exists.
4. **`enforced`** — at least one Chassis gate (a name appearing in some
   ADR's `enforces[]` array) emits an error-severity diagnostic on
   violation of one of this manifest's invariants. The binding is
   typically by file glob in the ADR's `applies_to`.
5. **`observed`** — runtime evidence (an entry in
   `.chassis/runtime-evidence.batch.json` or equivalent) references the
   claim_id, evidencing that the invariant is checked in production
   (monitor, telemetry assertion, or feature-flag canary).

Promotion is monotonic: a manifest must satisfy each lower tier before
declaring a higher one. The `assurance_promotion.py` gate (planned, C5)
enforces this in CI.

## Consequences

- Authors cannot shortcut from `declared` to `enforced` without filling
  in `test_linkage` and shipping a gate. This is intentional friction.
- A manifest can stay at `declared` indefinitely if the team has not
  invested in evidence — Chassis does not force promotion.
- `assurance_level` is optional in the schema; absence is treated as
  `declared`.
- Downstream consumers can layer additional tiers (e.g.
  `formally-verified`) by adding an ADR that supersedes this one and
  extending the enum.

## Status

Accepted. Authoring ADR for a decision already implicit in the schema
since v0.1. Enforcement gate scheduled in C5 of the chassis maturity
plan.
