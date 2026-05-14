---
id: ADR-0018
title: "Diagnostic envelope — shared shape for Wave 2 verifiers and downstream consumers"
status: accepted
date: "2026-05-14"
enforces:
  - rule: DIAGNOSTIC-ENVELOPE-CANONICAL
    description: "Wave 2 verifiers emit diagnostics conforming to schemas/diagnostic.schema.json plus the envelope fields in this ADR."
  - rule: DIAGNOSTIC-RULE-ID-REQUIRED
    description: "Every emitted diagnostic includes ruleId matching RULE-ID-GRAMMAR (ADR-0011)."
  - rule: DIAGNOSTIC-SEVERITY-CLOSED-ENUM
    description: "severity is one of error | warning | info only."
applies_to:
  - "schemas/diagnostic.schema.json"
  - "crates/chassis-core/**"
  - "packages/chassis-cli/**"
tags:
  - foundation
  - diagnostics
  - wave-2
---

## Context

Wave 2 introduces multiple verifiers (contract-diff, exemption flows, deeper kind validation). Each emits structured findings. Downstream tools — Wave 3 trace graph, Wave 4 CLI aggregation, MCP surfaces — need **one** composable envelope so outputs can be merged, sorted, and routed without per-tool adapters.

`schemas/diagnostic.schema.json` already defines the core finding shape used across the repository.

## Decision

### Canonical schema

The diagnostic envelope is **`schemas/diagnostic.schema.json`**. All Wave 2 components that emit JSON diagnostics MUST validate instances against this schema (or a strictly backward-compatible minor bump of it).

### Required envelope fields (already locked in schema)

Every emitted diagnostic MUST include:

| JSON field   | Role |
|--------------|------|
| `ruleId`     | Stable rule identifier (`^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`). Binds to ADR `enforces` via ADR-0011. |
| `severity`   | Exactly one of `error`, `warning`, `info`. |
| `message`    | Human-readable one-line summary. |

These satisfy **DIAGNOSTIC-RULE-ID-REQUIRED** and **DIAGNOSTIC-SEVERITY-CLOSED-ENUM**.

### Wave 2 emission additions

Wave 2 verifiers MUST additionally populate:

| JSON field | Requirement | Purpose |
|------------|-------------|---------|
| `source`   | **Required** on Wave 2 emitters | Logical producer id (e.g. `chassis validate`, `chassis diff`, `chassis exempt`). Lets routers attribute findings without parsing message text. |
| `subject`  | **Required** on Wave 2 emitters | Stable string naming what the finding is *about* — contract path, exemption id, claim id, rule scope, etc. May duplicate `location.path` when the subject is file-scoped; still required for uniform filtering. |

Optional fields (`violated`, `docs`, `fix`, `location`) SHOULD be used when the verifier has authoritative routing or remediation data.

### Structured payload

Machine-readable, rule-specific data MUST live under the schema’s existing **`detail`** object (not a parallel `details` key). Examples: `{ "field": "owner", "kind": "library" }` for a removed-required-field diff rule; `{ "exemptionId": "…" }` for exemption quota violations.

Naming note: prose and ADRs may say “details”; the canonical JSON property remains **`detail`** for backward compatibility with schema version `1.x`.

### Versioning

Changes to validation semantics follow ADR-0008. Additive optional envelope keys bump diagnostic schema **minor**; tightening (new required fields without a compatibility window) bumps **major** or ships as a parallel schema file.

## Consequences

- Contract-diff, exemption tooling, and kind validators can emit streams that concatenate cleanly.
- Consumers depend on `source` + `subject` for stable grouping; `ruleId` remains the primary policy hook.

## Relationship to ADR-0011

Rule ID grammar and immutability are unchanged; this ADR scopes **how** Wave 2 tools wrap findings around those identifiers.
