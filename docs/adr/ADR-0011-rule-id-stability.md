---
id: ADR-0011
title: "Rule ID stability — grammar, immutability, supersession"
status: accepted
date: "2026-05-14"
enforces:
  - rule: RULE-ID-GRAMMAR
    description: "Rule IDs match ^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$."
  - rule: RULE-ID-IMMUTABILITY
    description: "Accepted rule IDs never change meaning or spelling."
  - rule: RULE-ID-SUPERSEDES-PATTERN
    description: "Retiring a rule introduces a new rule ID + ADR supersession linkage; old IDs are never reused."
applies_to:
  - "docs/adr/**"
  - "schemas/diagnostic.schema.json"
tags:
  - foundation
  - governance
---

## Context

Diagnostics, exemptions, and CI gates route on stable rule identifiers. `reference/adrs-original/ADR-0011-ruleid-stability.md` focused on Python gate emission and transitional `*-GENERIC` buckets; this repository restarts tooling in Rust/TypeScript while preserving identifier discipline.

## Decision

### Grammar

Rule IDs MUST match `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$` (see `docs/STABLE-IDS.md`). This pattern is enforced structurally by `schemas/adr.schema.json` for ADR frontmatter and SHOULD be enforced for emitted diagnostics once codegen binds.

### Immutability

Once a rule ID appears under an ADR with `status: accepted`, **the token is immutable**:

- Do not rename to fix typos — publish a new rule ID instead.
- Do not broaden/narrow semantics under the same token — publish a new rule ID referencing the superseded ADR.

### Supersession

When a rule concept splits or changes materially:

1. Author a new ADR (or bump existing) accepting replacement rule IDs.
2. Mark the obsolete ADR `status: superseded` with `superseded_by` pointing at the successor ADR.
3. Keep the obsolete rule ID documented **only historically**; diagnostics MUST transition to the new IDs.

Transition buckets such as `*-GENERIC` are **not** introduced in this salvage unless a future ADR explicitly resurrects them.

## Consequences

- Binding diagnostics to ADRs stays injective: one meaning per rule token for all time.
- Occasional duplication of similar rule names is preferable to silent semantic drift.

## Relationship to predecessor

Focus shifts from Python gate migration to immutability promises suitable for a smaller OSS kernel; orphan/collision tooling arrives with diagnostics CLI.
