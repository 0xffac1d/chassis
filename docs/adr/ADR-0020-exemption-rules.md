---
id: ADR-0020
title: "Exemption registry rule IDs (CH-EXEMPT-*) for Wave 2 verifier"
status: accepted
date: "2026-05-14"
enforces:
  - rule: CH-EXEMPT-QUOTA-EXCEEDED
    description: "Error. Active exemption count exceeds the ADR-0004 ceiling of 25 entries."
  - rule: CH-EXEMPT-LIFETIME-EXCEEDED
    description: "Error. expires_at - created_at exceeds the ADR-0004 90-day cap (also fires when expires_at precedes created_at)."
  - rule: CH-EXEMPT-EXPIRED
    description: "Error. Entry has `status: active` and `expires_at` is in the past — ADR-0004 forbids silent persistence of active waivers past expiry. `status: expired` does NOT fire this rule."
  - rule: CH-EXEMPT-EXPIRED-RETAINED
    description: "Info. Entry has `status: expired` — retained only as audit evidence (never participates in active suppression). Symmetric to CH-EXEMPT-REVOKED."
  - rule: CH-EXEMPT-APPLIED
    description: "Info. Audit trail emitted by the apply pipeline when an exemption suppresses or downgrades a finding. `detail` carries `exemptionId`, the matched `ruleId`/`findingId`, and any `severityFrom`/`severityTo` override."
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
  - rule: CH-EXEMPT-REVOKED
    description: "Info. Entry has `status: revoked` — retained only as audit evidence (never participates in active suppression)."
  - rule: CH-EXEMPT-LEGACY-ALIAS
    description: "Info. Parsed JSON used v1 field aliases (`rule`, `scope`, `created`, `expires`, `ticket`); surfaced by `registry_parse_str_with_diagnostics`."
  - rule: CH-EXEMPT-GLOBAL-WITHOUT-OPT-IN
    description: "Error. Scope uses wildcards or repo-root `/` without both registry-level `allow_global: true` and per-entry `allow_global: true`."
  - rule: CH-EXEMPT-MISSING-RULE-OR-FINDING
    description: "Error. Entry lacks a non-empty `rule_id`, `finding_id`, and legacy `rule`."
applies_to:
  - "crates/chassis-core/src/exempt/**"
  - "schemas/exemption-registry.schema.json"
  - "fixtures/exempt/**"
tags:
  - wave-2
  - exemptions
  - diagnostics
---

## Context

ADR-0004 establishes the substantive policy (90-day lifetime cap, 25 active ceiling, CODEOWNERS union approval) but does not enumerate the diagnostic surface. Wave 2 introduces the first Rust verifier for the registry in `crates/chassis-core/src/exempt/`, plus a sweeper. Both emit `crate::diagnostic::Diagnostic` values against the ADR-0018 envelope. Each finding uses a stable `ruleId` per ADR-0011 so CI gates, exemption tooling, and MCP surfaces can route without parsing message text.

## Decision

All exemption-registry diagnostics use the `CH-EXEMPT-*` prefix. The rule IDs enumerated in ADR‑0020 `enforces[]` bind to the surfaces described in `crates/chassis-core/src/exempt/mod.rs`.

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
- `CH-EXEMPT-EXPIRED` — `{ expiresAt, status }` where `status == "active"` (the only status under which this rule fires).
- `CH-EXEMPT-EXPIRED-RETAINED` — `{ expiresAt }` (no `status` — fires only for `status: expired`).
- `CH-EXEMPT-APPLIED` — `{ exemptionId, action: "suppressed" | "severity-override", ruleId?, findingId?, path?, severityFrom?, severityTo? }`.
- `CH-EXEMPT-MISSING-CODEOWNERS` — `{ missing, required }`.
- `CH-EXEMPT-REMOVED-BY-SWEEPER` — `{ id, ruleId, expiresAt }`.
- `CH-EXEMPT-RULE-NOT-IN-ADR` — `{ ruleId }`.
- `CH-EXEMPT-GLOBAL-WITHOUT-OPT-IN` — no structured `detail` (subject=id).
- `CH-EXEMPT-MISSING-RULE-OR-FINDING` — no structured `detail` (subject=id).
- `CH-EXEMPT-LEGACY-ALIAS` — `{ legacyKeys: string[] }`.

Other rules emit no `detail` (their `subject` and `message` already carry the routable data).

## Expired-status policy

ADR-0004 says "expired exemptions fail CI until renewed or removed; entries are not silently deleted." That policy bites the **violation case**, not the audit case. We distinguish them so the registry can hold history without re-firing the same error every CI run:

| Entry state                                       | Effect                                                              |
|---------------------------------------------------|---------------------------------------------------------------------|
| `status: active`, `expires_at >= today`           | Eligible to suppress (subject to scope + signoff checks).           |
| `status: active`, `expires_at < today`            | **Error: `CH-EXEMPT-EXPIRED`.** Never suppresses. Fail-closed.      |
| `status: expired` (regardless of `expires_at`)    | **Info: `CH-EXEMPT-EXPIRED-RETAINED`.** Never suppresses. Audit-only. |
| `status: revoked`                                 | **Info: `CH-EXEMPT-REVOKED`.** Never suppresses. Audit-only.        |

`status: expired` is the curated "we acknowledge this is no longer active and we are keeping it for audit" lifecycle state — moving to it is how an owner explicitly resolves the `CH-EXEMPT-EXPIRED` error without losing the trail.

## Apply pipeline

The apply pipeline (`crates/chassis-core/src/exempt/apply.rs`) takes a vector of findings, the registry, and `now`, and returns:

- **unsuppressed** findings (pass-through, possibly with severity downgrades),
- **suppressed** records (finding + matching exemption + action),
- **audit** diagnostics (`CH-EXEMPT-APPLIED`, one per suppression or override).

Matching is fail-closed: only entries with `status: active` and `created_at <= today <= expires_at` are eligible. An entry's path-set must match the finding's `location.path` via glob; wildcard / repo-root paths additionally require both registry-level and per-entry `allow_global: true`. When both `rule_id` and `finding_id` are set on an entry, **both** must match the finding (intersection — the more restrictive interpretation). `severity_override` downgrades the finding's severity but the original severity is preserved on the audit record so evidence is never lost.

## Consequences

- Every emission point in `crates/chassis-core/src/exempt/` is one-to-one with a rule above; new emission surfaces require a new rule ID and an ADR update (per ADR-0011 immutability).
- CI gates can promote `CH-EXEMPT-RULE-NOT-IN-ADR` from warning to error without breaking schema compatibility.
- Fixtures under `fixtures/exempt/` pin the rule-ID surface; refactoring within the module cannot silently change which rule fires for a given input.
- The `CH-EXEMPT-EXPIRED` / `CH-EXEMPT-EXPIRED-RETAINED` split lets teams resolve expired waivers without re-introducing a CI failure on every subsequent run, while preserving the audit history ADR-0004 requires.

## Relationship to predecessor ADRs

- **ADR-0004** is the policy; this ADR is its diagnostic surface.
- **ADR-0011** governs the rule-ID grammar and immutability promise.
- **ADR-0018** defines the envelope into which these diagnostics are written.
