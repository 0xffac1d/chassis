---
id: ADR-0020
title: "Exemption registry rule IDs (CH-EXEMPT-*) for Wave 2 verifier"
status: draft
date: "2026-05-14"
enforces:
  - rule: CH-EXEMPT-QUOTA-EXCEEDED
    description: "Error. Active exemption count exceeds the ADR-0004 ceiling of 25 entries."
  - rule: CH-EXEMPT-LIFETIME-EXCEEDED
    description: "Error. expires_at - created_at exceeds the ADR-0004 90-day cap (also fires when expires_at precedes created_at)."
  - rule: CH-EXEMPT-EXPIRED
    description: "Error. Entry's expires_at is in the past but the entry is still present in the registry — ADR-0004 forbids silent persistence past expiry."
  - rule: CH-EXEMPT-MISSING-CODEOWNERS
    description: "Error. Required CODEOWNERS signoff (union across all paths covered by the entry) is missing from codeowner_acknowledgments."
  - rule: CH-EXEMPT-DUPLICATE-ID
    description: "Error. Two entries share the same exemption id; registry must be id-injective."
  - rule: CH-EXEMPT-MALFORMED-ID
    description: "Error. Exemption id does not match the EX-YYYY-NNNN grammar from docs/STABLE-IDS.md."
  - rule: CH-EXEMPT-RULE-NOT-IN-ADR
    description: "Warning. The exempted rule_id does not resolve to any ADR's enforces[]. Surface only — CI gate may promote to error."
  - rule: CH-EXEMPT-REMOVED-BY-SWEEPER
    description: "Info. The sweeper removed an expired entry from the registry — always logged for audit."
  - rule: CH-EXEMPT-PATHS-EMPTY
    description: "Error. An exemption's paths set is empty; at least one path is required (ADR-0004 scope requirement)."
  - rule: CH-EXEMPT-CODEOWNERS-PARSE-ERROR
    description: "Error. The CODEOWNERS file could not be parsed — verification cannot proceed without a known signoff map."
  - rule: CH-EXEMPT-NOT-FOUND
    description: "Error. remove() was called with an id that no entry in the registry matches. Routing aid for downstream tools."
applies_to:
  - "crates/chassis-core/src/exempt/**"
  - "schemas/exemption-registry.schema.json"
  - "fixtures/exempt/**"
tags:
  - wave-2
  - exemptions
  - diagnostics
---

> **Status note.** This ADR is a stub authored during Wave 2 Session E (exemption registry library). It will be promoted to `status: accepted` at Wave 2 close-out after both the contract-diff (Session D) and exemption (this session) Diagnostic structs have been hoisted to `crates/chassis-core/src/diagnostic.rs`. The rule IDs themselves are stable per ADR-0011.

## Context

ADR-0004 establishes the substantive policy (90-day lifetime cap, 25 active ceiling, CODEOWNERS union approval) but does not enumerate the diagnostic surface. Wave 2 introduces the first Rust verifier for the registry, plus a sweeper. Both emit `Diagnostic` instances against the ADR-0018 envelope. Each finding needs a stable `ruleId` per ADR-0011 so CI gates, exemption tooling, and MCP surfaces can route on them without parsing message text.

## Decision

All exemption-registry diagnostics use the `CH-EXEMPT-*` prefix. The eleven rule IDs above bind to the surfaces described in the table at `crates/chassis-core/src/exempt/mod.rs`. Severities follow the column in that table; consumers that want a stricter posture may promote a `warning` to `error` but the canonical severity is fixed per ADR-0011 immutability.

### `source` and `subject`

ADR-0018 requires `source` and `subject` on every Wave 2 emission. The exempt module sets:

| Field    | Value                                                           |
|----------|-----------------------------------------------------------------|
| `source` | The constant `"chassis exempt"`.                                |
| `subject`| The offending exemption id; the literal `"registry"` for whole-registry findings (e.g. quota). |

### `detail` payloads

Several rules emit machine-readable detail under the schema's `detail` object:

- `CH-EXEMPT-QUOTA-EXCEEDED` — `{ activeCount, maxActive }`.
- `CH-EXEMPT-LIFETIME-EXCEEDED` — `{ lifetimeDays, maxLifetimeDays, createdAt, expiresAt }`.
- `CH-EXEMPT-EXPIRED` — `{ expiresAt }`.
- `CH-EXEMPT-MISSING-CODEOWNERS` — `{ missing, required }`.
- `CH-EXEMPT-REMOVED-BY-SWEEPER` — `{ id, ruleId, expiresAt }`.
- `CH-EXEMPT-RULE-NOT-IN-ADR` — `{ ruleId }`.

Other rules emit no `detail` (their `subject` and `message` already carry the routable data).

## Consequences

- Every emission point in `crates/chassis-core/src/exempt/` is one-to-one with a rule above; new emission surfaces require a new rule ID and an ADR update (per ADR-0011 immutability).
- CI gates can promote `CH-EXEMPT-RULE-NOT-IN-ADR` from warning to error without breaking schema compatibility.
- Fixtures under `fixtures/exempt/` pin the rule-ID surface; refactoring within the module cannot silently change which rule fires for a given input.

## Relationship to predecessor ADRs

- **ADR-0004** is the policy; this ADR is its diagnostic surface.
- **ADR-0011** governs the rule-ID grammar and immutability promise.
- **ADR-0018** defines the envelope into which these diagnostics are written.
