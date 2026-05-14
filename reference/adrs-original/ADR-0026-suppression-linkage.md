---
id: ADR-0026
title: Suppression linkage
status: accepted
date: "2026-04-27"

enforces:
  - rule: CHASSIS-SUPPRESSION-MISSING-EXEMPTION-ID
    description: "An inline suppression token (e.g., # noqa, #[allow(...)], // eslint-disable, // @ts-ignore, # type: ignore, # pragma: no cover, <!-- chassis-skip -->) is not followed by an EX-YYYY-NNNN reference."
  - rule: CHASSIS-SUPPRESSION-EXEMPTION-UNKNOWN
    description: "An inline suppression cites an EX-YYYY-NNNN id that is not present in .exemptions/registry.yaml."
  - rule: CHASSIS-SUPPRESSION-EXEMPTION-EXPIRED
    description: "An inline suppression cites an EX-YYYY-NNNN entry whose expires_at date has passed."
  - rule: CHASSIS-SUPPRESSION-OVER-PER-FILE-QUOTA
    description: "More inline suppressions exist in a single file than the per_file_max quota allows."
applies_to:
  - "**/*.py"
  - "**/*.rs"
  - "**/*.ts"
  - "**/*.tsx"
  - "**/*.js"
  - "**/*.sh"
  - ".exemptions/registry.yaml"
  - "schemas/exemption/registry.schema.json"
  - "scripts/chassis/gates/suppression_linkage.py"
  - "config/chassis/suppression-linkage.toml"
supersedes: []
tags:
  - chassis
  - exemptions
  - governance
  - suppression
---

# ADR-0026: Suppression linkage

## Context

ADR-0004 establishes the exemption quota policy and registry shape: 25 total
entries, 1 per file, 90-day max lifetime, CODEOWNERS-protected. The
`exemption-quota` gate enforces the registry's internal consistency. But no
gate enforces the inverse direction: that every code-level suppression
token (`# noqa`, `# type: ignore`, `# pragma: no cover`, `#[allow(...)]`,
`// eslint-disable`, `// @ts-ignore`, `<!-- chassis-skip -->`) references an
active `EX-YYYY-NNNN` entry.

The current state: `.exemptions/registry.yaml` has zero entries, and
suppressions in the codebase carry no exemption ids. The registry is a
dead surface; suppressions accumulate freely. The chassis enforces this
discipline on consumer codebases via shipped templates but does not apply
it to itself.

## Decision

A new gate at `scripts/chassis/gates/suppression_linkage.py` walks the
repository and matches suppression tokens against an `EX-YYYY-NNNN`
follow-up reference on the same line or the immediately preceding
trailer. Each finding emits one of four rule IDs:

- `CHASSIS-SUPPRESSION-MISSING-EXEMPTION-ID` — token without an id.
- `CHASSIS-SUPPRESSION-EXEMPTION-UNKNOWN` — id does not resolve in
  `.exemptions/registry.yaml`.
- `CHASSIS-SUPPRESSION-EXEMPTION-EXPIRED` — id resolves but `expires_at` is
  past.
- `CHASSIS-SUPPRESSION-OVER-PER-FILE-QUOTA` — file exceeds the per-file
  cap.

The schema at `schemas/exemption/registry.schema.json` is extended with an
optional `bound_paths: [str]` per entry so an exemption can declare which
paths may cite it. The new config at `config/chassis/suppression-linkage.toml`
defines scanner suffixes and a `[ramp] grandfather_until = "2026-07-01"`
window. The shipped consumer template
(`templates/ci/gate-configs/suppression-linkage.consumer.toml`) omits the
grandfather window so consumers adopt strict enforcement from day 1.

The chassis adopts a three-step ramp because the registry is currently
empty and a strict day-1 enforcement would block every existing in-repo
suppression:

1. Land warn-only and write the inventory of unbound suppressions to
   `ARTIFACTS/chassis/suppression-linkage-inventory.json`.
2. Populate `.exemptions/registry.yaml` from the inventory; switch the gate
   to error.
3. Ratchet `total_max` downward over time as suppressions are removed.

## Consequences

- The `.exemptions/registry.yaml` file becomes load-bearing. Suppressions
  must point to a real entry with owner, reason, and expiry.
- New suppressions require a registry entry in the same PR.
- The chassis dogfoods the suppression-linkage discipline it has been
  shipping in consumer templates.
- Expired exemptions cause CI failure 90 days after creation, forcing a
  conscious decision to renew or remove.

## Alternatives considered

- **Hard-fail on day 1.** Rejected: an empty registry would block every
  existing suppression and cause a flag-day. The three-step ramp gives the
  team time to inventory and triage.
- **Per-language gates (one for Python, one for Rust, etc.).** Rejected: the
  rule is uniform across languages; a single gate with a suffix-driven
  scanner is simpler to maintain and gives a unified inventory.
- **Tie suppression linkage to commit hooks instead of CI.** Rejected:
  commit hooks are bypassable (`--no-verify`); CI is the authoritative
  surface for governance gates.

## References

- ADR-0004 (exemption quota policy): defines the registry shape and quota
  rules this gate consumes.
- ADR-0011 (ruleId stability discipline): every ruleId emitted by this gate
  must resolve here.
- `.exemptions/registry.yaml`: the live registry (currently empty).
- `scripts/chassis/exempt.py`: existing CLI for registry management;
  `_load(root)` and `_validate` are reused by the gate.
- `templates/ci/gate-configs/`: consumer-template directory where the
  strict consumer profile ships.

## Status

Accepted. Gate implementation lands as a three-PR ramp (Gate C in the
chassis self-governance hardening plan).
