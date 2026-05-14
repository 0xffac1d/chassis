---
id: ADR-0003
title: "Claims model — claim IDs, grammar, invariants vs edge cases, rule linkage"
status: accepted
date: "2026-05-14"
enforces:
  - rule: CLAIM-ID-GRAMMAR
    description: "Claim ids match the canonical grammar and length bounds."
  - rule: CLAIM-NAMESPACE-DOTTED
    description: "New claim ids SHOULD use dotted namespaces (<domain>.<topic>). Monolithic ids remain legal if grammar-compliant."
  - rule: INVARIANT-EDGE-CASE-DISTINCTION
    description: "Invariants vs edge_cases share grammar but differ in verifier semantics (must always hold vs bounded exceptions)."
applies_to:
  - "schemas/contract.schema.json"
  - "**/CONTRACT.yaml"
tags:
  - foundation
  - claims
---

## Context

Contracts encode behavioral intent as machine-addressable **claims**. The predecessor ADR (`reference/adrs-original/ADR-0003-claims-model.md`) introduced structured `{id, text}` rows and string legacy compatibility. This rebuild tightens the CONTRACT schema and removes ambiguity between prose-only claims and linkage-ready identifiers.

## Decision

### Grammar (formal)

Let `LETTER := [a-z]`, `CONT := [a-z0-9_.-]`.

```
claim_id := LETTER CONT{1,119}     # total length 2..120 inclusive
```

Equivalently the regex `^[a-z][a-z0-9_.-]*$` with `minLength: 2`, `maxLength: 120`.

**Human-facing shape:** dot-namespaced phrases such as `auth.no-anon-write` (`<domain>.<qualifiers...>`). Dots are optional but encouraged (`CLAIM-NAMESPACE-DOTTED`).

### Invariants vs `edge_cases`

Both are claims (same ID grammar, same structured `{id, text}` representation in YAML).

| Slot | Meaning | Verifier posture |
|------|---------|------------------|
| `invariants` | Properties that must **always** hold in correct implementations | Fail closed: violations are defects unless covered by an explicit, registered exemption. |
| `edge_cases` | Bounded scenarios defining **expected** behavior (often defensive UX, failure modes, ambiguous inputs) | Document + test targets; violations may be severity-tiered but remain explicitly enumerated behaviors. |

Trace tooling treats both as first-class claim IDs for linkage; distinction drives **default severity** and review prompts, not schema shape.

### Claim IDs vs rule IDs vs ADRs

- **Claim IDs** originate from authored `CONTRACT.yaml` (`invariants[].id`, `edge_cases[].id`). They address “what the module promises.”
- **Rule IDs** originate from **accepted ADRs** (`enforces[].rule`). They address “what check failed” in diagnostics and exemptions.
- **ADRs** are the authoritative registry for rule IDs; diagnostics MUST cite a rule ID bound to an ADR (see ADR-0011).

### Identifier hygiene

- Claim IDs are unique **within** a single CONTRACT document (collision is invalid authoring).
- Claim IDs are **not** required to be globally unique across the repository — different modules may reuse the same phrase only when intentionally aligned; tooling SHOULD warn on accidental collisions in the same ownership boundary.

### Legacy strings

String-only invariant rows are **removed from the canonical tightened schema** (Wave 1). Migration tooling may translate legacy manifests out-of-band; Wave 1 generators MUST emit structured `{id, text}` rows.

## Consequences

- Test linkage and trace graphs key solely on stable claim IDs — never on prose excerpts.
- Downstream schema tightening can require IDs without ambiguities inherent to free text.

## Relationship to predecessor

The predecessor preserved string-form compatibility and Python-oriented migration CLI verbs. This repository drops string-form claims from the canonical schema to align with spec-first authoring and smaller validators, while preserving grammar and namespace guidance.
