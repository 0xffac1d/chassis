---
id: ADR-0025
title: Self-governance warning ratchet — discipline and bump protocol
status: accepted
date: "2026-04-30"
supersedes:
  - ADR-0024
applies_to:
  - ".chassis/ratchet.yaml"
  - "scripts/ci/self-governance-ratchet.py"
  - "ARTIFACTS/final-export/self-governance-ratchet.json"
enforces:
  - rule: CHASSIS-RATCHET-UNAUTHORIZED-INCREASE
    description: "current_warnings > previous_baseline with no bump_justification_path configured."
  - rule: CHASSIS-RATCHET-MISSING-JUSTIFICATION
    description: "bump_justification_path is set but the file does not exist on disk."
  - rule: CHASSIS-RATCHET-CEILING-EXCEEDED
    description: "current_warnings > max_allowed_warning_count, regardless of ADR coverage."
---

# ADR-0025: Self-governance warning ratchet — discipline and bump protocol

## Status

Accepted — 2026-04-30. Supersedes ADR-0024 (which silently raised the
ceiling from 107 to 110 to "bless an increase"; that pattern is now
prohibited).

## Context

The Chassis self-governance ratchet (`.chassis/ratchet.yaml` +
`scripts/ci/self-governance-ratchet.py`) tallies pre-existing
`CHANGED-UNSCAFFOLDED-FILE` warnings against governed zones (`docs/chassis`,
`schemas`, `scripts/chassis`). The release goal is **0 warnings** (or
explicit, scoped, expiring exemptions).

The ratchet exists to make the count strictly monotone-non-increasing
unless an authorial decision is recorded. Three new test files added
between the previous baseline and the start of this pass pushed the
count to **110** (and the live audit subsequently surfaced **113**); the
existing ratchet config silently bumped `current_max_warnings` from 107
to 110 with a thin "blesses the increase" justification (ADR-0024).
That pattern defeats the purpose of the ratchet — silent ceiling moves
are indistinguishable from real regressions.

## Decision

1. **Anchor the baseline.** `.chassis/ratchet.yaml` carries
   `previous_baseline: 107` as the authoritative ADR-anchored count.
   The baseline is the count below which the ratchet trivially passes.
2. **No silent ceiling moves.** Raising
   `max_allowed_warning_count` above `previous_baseline` requires a
   committed `bump_justification_path` (an ADR with the required fields
   listed below). The gate refuses to pass if the path is empty or the
   referenced file does not exist on disk.
3. **Hard cap.** Even with a valid ADR, the ratchet still fails if
   `current_warnings > max_allowed_warning_count`. Bumps are bounded.
4. **Prefer remediation.** When the count rises, the first action is
   real scaffolding (`chassis scaffold <blueprint> <name>`,
   `chassis.module.yaml` ownership claim, governed-zone refinement).
   Bumps are the fallback when remediation is unavailable in this pass.

## Resolution of the 107 → 110 incident

| Field                    | Value |
|--------------------------|-------|
| `previous_baseline`      | 107 |
| `current_warnings`       | 100 (post-remediation; was 113 live / 110 claimed pre-remediation) |
| `delta`                  | -7 (post-remediation; was +6 live / +3 claimed pre-remediation) |
| `max_allowed_warning_count` | 107 (returned to baseline; the ADR-0024 bump to 110 is reverted) |
| `bump_justification_path` | `null` (no bump active) |
| `release_target_warning_count` | 0 |
| `next_pass_target_warnings`     | 50 |

### Exact warning categories added between baseline and the bump

The three files that pushed the changed-audit tally from 107 to 110 all
sit in the `scripts/chassis/tests` governed zone with
`ruleId: CHANGED-UNSCAFFOLDED-FILE`:

1. `scripts/chassis/tests/test_init_alias_semantics.py` (added in commit
   `6e3c8272`).
2. `scripts/chassis/tests/test_report_integrity_export.py` (added in
   commit `ea2df9d9`).
3. `scripts/chassis/tests/test_wheel_package_contents.py` (added in
   commit `ea2df9d9`).

The live audit subsequently showed **113** because three additional
cross-cutting tests had landed in `scripts/chassis/tests/` since the
ADR-0024 bump (visible in `git log --diff-filter=A --
chassis/scripts/chassis/tests/`).

### Remediation applied in this pass

* Added a new `cli-test-suite` blueprint in `.chassis/structure.yaml`
  describing the cross-cutting Chassis CLI regression suite under
  `scripts/chassis/tests/`.
* Added a new governed zone `scripts/chassis/tests` with
  `allowed_blueprints: [cli-test-suite]` (the existing more-specific
  `scripts/chassis/tests/fixtures` zone for `test-fixture` retains
  precedence inside its subtree).
* Added `scripts/chassis/tests/chassis.module.yaml` claiming the entire
  cross-cutting test suite as one ownership unit.

This retired all 13 unscaffolded warnings under `scripts/chassis/tests/`
(including the three new files above), dropping the live count from 113
to **100**. The ratchet ceiling was therefore returned to **107** and
the bump justification cleared.

## Owner

Chassis maintainers (`pyproject.toml` `[project].authors`). Direct
inquiries to the same group via the `chassis-maintainers` GitHub team
(or whichever team is named in `.exemptions/registry.yaml`'s
`approver_team`).

## Reason (when a future bump becomes necessary)

A future bump may be unavoidable when:

1. A cross-cutting governed-zone subtree appears that cannot be cleanly
   covered by an existing or new blueprint within the current pass.
2. A schema family or doc tree is renamed and the migration must land
   atomically before the new owners can be authored.
3. A vendored upstream import lands en bloc and remediation is queued
   behind a separate ADR.

In every case the bump must be temporary, owner-bound, and have a
remediation date.

## Remediation plan (if a future bump is required)

1. File a new ADR named `ADR-NNNN-self-governance-ratchet-bump-<slug>.md`
   that supersedes this one and carries the **required fields** below.
2. Set `bump_justification_path` in `.chassis/ratchet.yaml` to that
   ADR's repo-relative path.
3. Set `max_allowed_warning_count` to the smallest ceiling that admits
   the new count (never larger than `previous_baseline + delta`).
4. Within the remediation window, retire warnings via
   `chassis scaffold` / `chassis.module.yaml` claims; once the live
   count returns to `previous_baseline`, lower the ceiling back and
   clear `bump_justification_path`.

## Required fields for any future bump ADR

A bump ADR is **invalid** if any of these fields are missing or empty:

| Field                    | Notes |
|--------------------------|-------|
| `previous_baseline`      | The integer count being bumped above. |
| `current_warnings`       | Live tally at ADR-write time. |
| `delta`                  | `current_warnings - previous_baseline`. |
| Exact warning categories | List of `ruleId` + path entries the bump admits. |
| `owner`                  | A team or named maintainer accountable for the remediation. |
| `reason`                 | Why scaffolding/ownership was not applicable in this pass. |
| `remediation_plan`       | Concrete steps to retire the surplus warnings. |
| `remediation_date`       | ISO-8601 date by which the count returns to baseline (no longer than 90 days out, mirroring the exemption lifetime in ADR-0004). |

## Consequences

- The gate (`scripts/ci/self-governance-ratchet.py`) refuses to pass
  silently. Any rise above `previous_baseline` requires a committed,
  on-disk ADR; a missing or invalid path is a hard fail.
- The artefact (`ARTIFACTS/final-export/self-governance-ratchet.json`)
  carries `previous_baseline`, `current_warnings`, `delta`,
  `max_allowed_warning_count`, `release_target_warning_count`,
  `bump_justification_path`, `bump_justification_present`, `status`,
  and `ratchet_ok` so downstream consumers can reason about the gap.
- ADR-0024 is **superseded**. Its body is preserved as historical
  context inside that file, with frontmatter `status: superseded` pointing
  to this ADR.

## Evidence

- `.chassis/ratchet.yaml` — canonical schema and the live values.
- `scripts/ci/self-governance-ratchet.py` — gate implementation,
  including `evaluate_ratchet()` (the importable pure evaluator the
  unit tests pin against).
- `scripts/chassis/tests/test_self_governance_ratchet.py` — gate
  regression tests covering decrease / unchanged / increase-without-ADR
  / increase-with-valid-ADR / missing-file / ceiling-exceeded.
- `ARTIFACTS/final-export/self-governance-ratchet.json` — live tally
  produced by the gate.
- `ARTIFACTS/final-export/self-governance-ratchet.md` — human-readable
  summary regenerated alongside the JSON.
- `.chassis/structure.yaml` — blueprint and zone updates that retired
  the 13 `scripts/chassis/tests/` warnings.
- `scripts/chassis/tests/chassis.module.yaml` — ownership claim for the
  cross-cutting CLI regression suite.
