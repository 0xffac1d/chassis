---
id: ADR-0003
title: Claims model — structured {id, text} form for invariants and edge_cases
status: accepted
date: "2026-04-19"
enforces:
  - rule: CLAIM-ID-MISSING
    description: "Invariant or edge_case uses object form without a stable id (string-only legacy form is allowed but not preferred)."
  - rule: CLAIM-ID-MALFORMED
    description: "Claim id does not match ^[a-z][a-z0-9_.-]*$ (kebab-snake) or exceeds 120 chars."
  - rule: CLAIM-ID-COLLISION
    description: "Two claims within the same CONTRACT.yaml share the same id."
  - rule: TEST-LINKAGE-CLAIM-ID-UNRESOLVED
    description: "test_linkage[].claim_id does not match any invariant or edge_case id in the same CONTRACT.yaml."
applies_to:
  - "schemas/metadata/contract.schema.json"
  - "**/CONTRACT.yaml"
tags:
  - chassis
  - governance
  - claims
---

# ADR-0003: Claims model

## Context

`CONTRACT.yaml` historically accepted invariants and edge_cases as
plain strings:

```yaml
invariants:
  - "compute() never panics."
```

This makes claims unaddressable: `test_linkage` had to refer to the
claim by a prose excerpt ("compute() never panics"), which drifted as
soon as anyone reworded the text. The schema now accepts a structured
form with a stable id:

```yaml
invariants:
  - id: api.no-panic
    text: "compute() never panics."
```

The structured form is preferred. The string form remains schema-valid
for legacy migration only.

## Decision

1. **Structured form is the modern convention.** Every new invariant
   and edge_case should use `{id, text}`. Authoring tools, generators,
   and `chassis init` produce structured form.
2. **`id` pattern.** `^[a-z][a-z0-9_.-]*$`, 2–120 chars. Convention:
   `<domain>.<sub-topic>` (e.g. `bucket.never-exceeds-capacity`,
   `api.allow-returns-deterministic-decision`). The pattern is enforced
   by `schemas/metadata/contract.schema.json`.
3. **Uniqueness.** Claim ids are unique within a single CONTRACT.yaml.
   They are NOT globally unique across the repo — collisions across
   manifests are allowed and expected.
4. **`test_linkage[].claim_id` MUST be a structured id** for new
   manifests. For string-form legacy claims, `claim_id` may be a prose
   excerpt; this is preserved for migration but flagged with
   `TEST-LINKAGE-CLAIM-ID-UNRESOLVED` (warning) when the excerpt does
   not uniquely match.
5. **Migration path.** `chassis migrate-contract` (existing CLI)
   converts string-form invariants/edge_cases to structured form by
   slugifying the first 3–5 words of the text into an id. Reviewer
   must confirm the chosen id before promotion to `verified` assurance.

## Consequences

- Renaming claim text never breaks `test_linkage`. Renaming a claim
  *id* (rare) requires a coordinated update.
- Coverage tools (`chassis coverage-contracts`) can rely on stable
  `claim_id` joins instead of fuzzy text matching.
- The string-form legacy invariants in older manifests continue to
  validate; they generate a `CLAIM-ID-MISSING` warning at info level
  during transition.

## Status

Accepted. Structured form shipped with Milestone A.3
(`docs/roadmap/chassis-completion-plan-2026-04-17.md`). This ADR
formalizes the model retrospectively.
