---
id: ADR-0024
title: Self-governance warning ratchet ceiling bump (107 → 110) [superseded]
status: superseded
date: "2026-04-30"
superseded_by: ADR-0025
---

# ADR-0024: Self-governance warning ratchet ceiling bump (107 → 110) [SUPERSEDED]

## Status

**Superseded** by [ADR-0025](ADR-0025-self-governance-ratchet-bump.md) on
2026-04-30. The unconditional bump captured here was reverted: the ceiling is
back at the **107** baseline and the live warning count is **100**, retired by
adding the `cli-test-suite` blueprint and a `chassis.module.yaml` ownership
manifest at `scripts/chassis/tests/`.

This file is retained as historical context. It must not be cited as authority
for any future bump; the new bump protocol is documented in the superseding ADR
and in `.chassis/ratchet.yaml`'s schema header.

## Context

The in-tree `CHANGED-UNSCAFFOLDED-FILE` warning tally under `chassis audit .
--changed` rose from **107** to **110** after additional governed-zone paths
appeared in the diff (scan semantics and repository growth). The original
disposition was to raise `current_max_warnings` to 110 with this ADR as a thin
justification.

The pattern of “ratchet rose, raise the ceiling, point at an ADR that just
blesses the increase” defeats the purpose of the ratchet — it is
indistinguishable from a silent ceiling move. The replacement ADR formalises a
strict protocol (required fields, owner, remediation plan and date, hard
ceiling that cannot be silently exceeded) and required real remediation before
any bump may stand.

## Decision

Raise `current_max_warnings` in `.chassis/ratchet.yaml` from **107** to **110**
so CI remains green while module-by-module scaffolding retires warnings.

This decision was **reverted** in favour of the protocol in ADR-0025; the ratchet
 again uses **107** as the ceiling with `bump_justification_path: null`.

## Consequences

- **Owner:** Chassis maintainers (see `pyproject.toml` authors).
- **Remediation:** Continue scaffolding `chassis.module.yaml` manifests or
  documented exemptions; target remains `release_target_warnings: 0`.
- **Review:** Revisit when the live warning count drops materially or before any
  **ceiling** increase (any new bump requires a new ADR per ADR-0025).

**Resolution (current):** Live count returned to **100** (≤ 107 baseline).
`.chassis/ratchet.yaml` carries `max_allowed_warning_count: 107`,
`bump_justification_path: null`. See [ADR-0025](ADR-0025-self-governance-ratchet-bump.md).

## Evidence

- `ARTIFACTS/final-export/self-governance-ratchet.json` — live tally and `ratchet_ok`.
- `scripts/ci/self-governance-ratchet.py` — enforces ceiling and justification file
  when `max_allowed_warning_count` exceeds `previous_baseline`.
