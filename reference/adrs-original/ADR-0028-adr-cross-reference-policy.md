---
id: ADR-0028
title: ADR cross-reference policy
status: accepted
date: "2026-04-27"

enforces:
  - rule: CHASSIS-ADR-XREF-UNRESOLVED-ID
    description: "A guide, ADR, or canonical doc references an ADR-NNNN id that does not appear in docs/index.json."
  - rule: CHASSIS-ADR-XREF-RULE-NOT-ENFORCED
    description: "A guide, ADR, or canonical doc mentions a rule-id-shaped token that is not present in any ADR's enforces[] list."
  - rule: CHASSIS-ADR-XREF-SUPERSEDED-CITED
    description: "A guide cites an ADR whose status is superseded without preceding the citation with the literal '(superseded)'."
applies_to:
  - "docs/chassis/guides/**/*.md"
  - "docs/adr/*.md"
  - "AGENTS.md"
  - "CLAUDE.md"
  - "skills/**/*.md"
  - "scripts/chassis/gates/adr_cross_reference.py"
  - "config/chassis/adr-cross-reference.toml"
  - "docs/index.json"
supersedes: []
tags:
  - chassis
  - adr
  - doc-binding
  - governance
---

# ADR-0028: ADR cross-reference policy

## Context

ADRs are the source of truth for chassis governance rules. Guides
frequently cite ADRs and rule IDs in prose to explain why a particular
behavior is required. The chassis depends on these citations being
accurate: a guide that cites `ADR-0017` for pin policy or
`CHASSIS-LAYOUT-FORBIDDEN-FILE` for a forbidden-file rationale is making a
claim that the cited authority exists and still applies.

Today nothing checks the citations. A renamed rule, a deleted ADR, or a
superseded decision can leave guide prose pointing at a phantom authority.
Agents writing new guides have no automatic check that the rule IDs they
mention are real.

## Decision

A new gate at `scripts/chassis/gates/adr_cross_reference.py` scans every
guide, ADR, canonical doc, and skill markdown body for two token shapes:

- `ADR-NNNN` (4+ digits) — must resolve in `docs/index.json`.
- Rule-id-shaped tokens matching `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$` — must
  appear in some ADR's `enforces[]` list.

Findings emit one of three rule IDs:

- `CHASSIS-ADR-XREF-UNRESOLVED-ID` — `ADR-NNNN` not in `docs/index.json`.
- `CHASSIS-ADR-XREF-RULE-NOT-ENFORCED` — rule-id token has no ADR backing.
- `CHASSIS-ADR-XREF-SUPERSEDED-CITED` — citing a superseded ADR without
  the literal `(superseded)` qualifier preceding it.

The gate reuses the `_load_rule_registry` pattern from
`scripts/chassis/gates/binding_link.py` and the `repo_layout.repo_root`
helper. Configuration lives in `config/chassis/adr-cross-reference.toml`
with scope globs for guides, ADRs, `AGENTS.md`, `CLAUDE.md`, and skills.

The gate ships warn-only for two minor releases with a baseline at
`config/chassis/baselines/adr-cross-reference.json` to absorb the existing
inventory of stale citations. Promotion to error follows.

## Consequences

- Guide authors get fast feedback when they cite a rule that does not
  exist (typo, renamed rule, removed ADR).
- ADR-supersedes hygiene improves: superseded ADRs cited as still-current
  authority are flagged.
- The chassis self-validates the same authority chain it asks downstream
  consumer repos to maintain.

## Alternatives considered

- **Validate only in the docs build, not in the gate suite.** Rejected:
  the chassis ships gates that consumers are expected to run; running the
  same checks here keeps the gate suite uniform.
- **Soft warn forever (no promotion to error).** Rejected: the
  warn-with-baseline pattern is preferred so the inventory ratchets down
  rather than accumulating indefinitely.

## References

- ADR-0011 (ruleId stability discipline): defines the rule-id token shape
  this gate matches.
- ADR-0014 (governance tiers): superseded-ADR citation policy aligns with
  the existing governance lifecycle.
- `scripts/chassis/gates/binding_link.py`: source of the `_load_rule_registry`
  pattern.
- `docs/index.json`: ADR registry; the gate validates citations against it.

## Status

Accepted. Implementation lands as Gate F in the chassis self-governance
hardening plan, after Tranche 2 (which adds guide nodes to the doc-graph
and creates many new internal cross-references).
