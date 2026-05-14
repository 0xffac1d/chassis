---
id: ADR-0004
title: Exemption quota policy — 25 max, 90-day max lifetime, CODEOWNERS-protected
status: accepted
date: "2026-04-19"
enforces:
  - rule: EXEMPTION-QUOTA-EXCEEDED
    description: ".exemptions/registry.yaml has more than 25 active entries (configurable via quota.total_max)."
  - rule: EXEMPTION-QUOTA-PER-FILE-EXCEEDED
    description: "More than one active exemption applies to a single file (configurable via quota.per_file_max)."
  - rule: EXEMPTION-EXPIRED
    description: "An exemption's expires date has passed; it must be removed or renewed (within 90-day cap)."
  - rule: EXEMPTION-LIFETIME-EXCEEDED
    description: "Exemption expires more than 90 days after created; rejected by `chassis exempt add`."
  - rule: EXEMPTION-MALFORMED-ID
    description: "Exemption id does not match ^EX-\\d{4}-\\d{4}$."
  - rule: EXEMPTION-MISSING-OWNER
    description: "Exemption entry omits the owner field."
  - rule: EXEMPTION-REASON-TOO-SHORT
    description: "reason field is shorter than 40 characters; prevents 'temp fix' / 'todo later' one-liners."
  - rule: SUPPRESSION-WITHOUT-EXEMPTION
    description: "Inline suppression marker (#[allow(...)], // eslint-disable, etc.) lacks a paired EX-YYYY-NNNN reference."
applies_to:
  - ".exemptions/registry.yaml"
  - "schemas/exemption/registry.schema.json"
  - "scripts/chassis/exempt.py"
tags:
  - chassis
  - governance
  - exemptions
---

# ADR-0004: Exemption quota policy

## Context

Every static-analysis ecosystem grows a thicket of inline suppressions
(`#[allow(...)]`, `// eslint-disable-next-line`, `# noqa`) over time.
Without a quota, these accumulate silently and erode the value of the
underlying check. The `.exemptions/registry.yaml` mechanism makes
suppressions explicit, expiring, and CODEOWNERS-protected; this ADR
pins the quotas.

## Decision

1. **Hard quota: 25 active entries total.** Configurable per repo via
   `.exemptions/registry.yaml::quota.total_max`. Default of 25 is the
   ship value for this distribution. Exceeding the quota fails CI.
2. **Per-file quota: 1 active entry per file.** Stops "swiss cheese"
   suppression in long files. Configurable via
   `quota.per_file_max`.
3. **Lifetime cap: 90 days.** `expires - created <= 90 days`. Enforced
   at write time by `chassis exempt add` (refuses to record entries
   exceeding the cap). Renewals require a new entry with a fresh
   `created` date and updated `reason`.
4. **Required fields.** `id` (`EX-YYYY-NNNN`), `rule` (a registered
   ruleId), `scope` (path or glob), `reason` (40+ chars),
   `ticket`, `owner`, `created`, `expires`, `adr` (the ADR
   defining the rule). All schema-enforced.
5. **CODEOWNERS protection.** `.exemptions/` directory is in
   `CODEOWNERS` and requires a maintainer review for every PR.
6. **Inline suppression must reference an entry.** Any `#[allow(...)]`,
   `// eslint-disable*`, or equivalent must include an
   `EX-YYYY-NNNN` token in the same line or the line above. The
   `validate-suppressions` static check enforces this.

## Consequences

- Suppressions become a constrained, auditable resource rather than a
  free-for-all.
- Renewals force conscious re-justification every 90 days; "temp fix"
  forever-suppressions are not possible.
- A team that genuinely needs more than 25 exemptions can raise the
  quota by editing `quota.total_max`, but the change is visible in PR
  review and requires CODEOWNERS sign-off.

## Status

Accepted. Schema and CLI shipped in Milestone E.1
(`docs/roadmap/chassis-completion-plan-2026-04-17.md`).
