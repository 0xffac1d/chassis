---
id: ADR-0004
title: "Exemption quota — 90-day lifetime cap, 25 active ceiling, CODEOWNERS governance"
status: accepted
date: "2026-05-14"
enforces:
  - rule: EXEMPTION-MAX-LIFETIME-90D
    description: "No exemption entry may exceed 90 days between created and expires (inclusive)."
  - rule: EXEMPTION-MAX-ACTIVE-25
    description: "No more than 25 simultaneously active exemptions repo-wide unless quota explicitly raised via governed change."
  - rule: EXEMPTION-CODEOWNERS-REQUIRED
    description: "Every exemption write requires CODEOWNERS-approved review on the registry path."
  - rule: EXEMPTION-EXPIRY-BEHAVIOR
    description: "Expired exemptions fail CI until renewed or removed; entries are not silently deleted."
applies_to:
  - "schemas/exemption-registry.schema.json"
  - ".github/CODEOWNERS"
  - "CODEOWNERS"
  - ".exemptions/**"
tags:
  - foundation
  - exemptions
---

## Context

Exemptions prevent unchecked suppression growth while allowing bounded variance from declared rules. `reference/adrs-original/ADR-0004-exemption-quota.md` described quotas against Python tooling paths; this ADR re-binds the policy to the salvaged registry schema and future Rust/TypeScript CLIs.

## Decision

### Caps & enforcement surfaces

- **90-day maximum lifetime:** `expires - created ≤ 90 days` (calendar-date fields normalized to UTC ISO dates). Enforced **both** at authoring/write time (CLI / editor hook) **and** in CI (registry validator). Dual enforcement catches stale local edits and bypass attempts.
- **25 active entries ceiling:** Enforced **both** at write time and CI. Raising the quota requires an explicit registry edit reviewed under the same CODEOWNERS rules (visibility > stealth).

### What “active” means

An exemption is **active** iff today’s UTC date satisfies `created ≤ today ≤ expires` **and** it has not been manually marked `revoked` / `inactive` per registry schema semantics.

Git history is diagnostic context only — quota evaluation uses **registry intent + calendar dates**, not branch topology.

### CODEOWNERS authority across regions

GitHub evaluates CODEOWNERS by patterns on concrete paths. When an exemption scope spans multiple ownership regions:

1. Split the scope into path literals / globs.
2. Resolve CODEOWNERS matches **per covered path** using repository precedence (`/.github/CODEOWNERS` wins over `/CODEOWNERS` when both exist; same file uses last matching rule per GitHub semantics).
3. **Approval union:** reviewers from **every distinct owning team pattern hit** must approve the exemption PR unless the registry entry is narrowed to a single ownership region.

### Expiry behavior

Expired entries **fail CI** (`EXEMPTION-EXPIRED`) until authors renew (new id + fresh rationale) or remove the entry. Entries remain visible in the registry as expired rows for audit — **no silent auto-deletion**.

### Relationship to inline suppressions

Inline suppressions must continue to cite registry ids once suppression linting lands (future wave); this ADR defines quotas only.

## Consequences

- Teams cannot stockpile perpetual suppressions; renewal forces intent refresh.
- CI remains the ultimate backstop even if local tooling is skipped.

## Relationship to predecessor

The predecessor referenced Python `exempt.py` and per-file caps. Wave 1 retains lifetime + repo-wide cap but defers per-file caps until exemption CLI lands; dual enforcement (write + CI) is explicit here.
