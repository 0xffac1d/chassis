---
id: ADR-0029
title: CI strictness contract
status: accepted
date: "2026-04-27"

enforces:
  - rule: CHASSIS-CI-CONTINUE-ON-ERROR-IN-STRICT
    description: "A GitHub Actions job whose name contains 'strict' has at least one step with continue-on-error: true or a trailing '|| true' shell sentinel."
  - rule: CHASSIS-CI-STANDALONE-NOT-ON-MAIN
    description: "The release-standalone-gate workflow is not configured to run on pull requests targeting main."
  - rule: CHASSIS-CI-NO-ARTIFACT-IN-STRICT
    description: "A strict release-gate step passes --no-artifact, suppressing the audit artifact emission expected of strict runs."
applies_to:
  - ".github/workflows/*.yml"
  - "scripts/chassis/gates/ci_strictness_contract.py"
  - "scripts/ci/check-strict-ci-pass-throughs.sh"
  - "config/chassis/posture.toml"
  - "templates/ci/**"
supersedes: []
tags:
  - chassis
  - ci
  - governance
  - posture
---

# ADR-0029: CI strictness contract

## Context

The chassis ships gates designed to enforce strict structural invariants on
consumer codebases. To certify that those gates work, they must run
strictly against the chassis itself in CI. Today they do not, in three
specific ways:

1. The advisory release-gate job is wired with `continue-on-error: true`,
   so warnings never block.
2. The strict release-gate step passes `--no-artifact`, suppressing the
   audit emission that a strict run is expected to produce.
3. `release-standalone-gate` is gated to run only on `release/*` branches,
   not on PRs targeting `main`. Structural drift can therefore land on
   `main` undetected.

A standalone shell utility (`scripts/ci/check-strict-ci-pass-throughs.sh`)
exists for the first concern but has no rule ID, no ADR backing, and is
invoked manually rather than as a gate. The chassis cannot prove its CI is
strict on its own terms.

## Decision

A new gate at `scripts/chassis/gates/ci_strictness_contract.py` parses
`.github/workflows/*.yml` (and the consumer-template `.yml` files under
`templates/ci/`) and asserts three invariants:

1. Any job whose name matches a `*strict*` pattern has zero
   `continue-on-error: true` step entries and no trailing `|| true` shell
   sentinels.
2. The `release-standalone-gate` workflow includes a trigger that fires on
   pull requests targeting `main`, not just `release/*` branches.
3. Strict release-gate steps do not pass `--no-artifact`.

Findings emit one of `CHASSIS-CI-CONTINUE-ON-ERROR-IN-STRICT`,
`CHASSIS-CI-STANDALONE-NOT-ON-MAIN`, or `CHASSIS-CI-NO-ARTIFACT-IN-STRICT`.

The gate wraps the existing
`scripts/ci/check-strict-ci-pass-throughs.sh` for backward compatibility
and adds the structural checks via `yaml.safe_load`. Configuration extends
`config/chassis/posture.toml` (the closest neighbor; it already governs
profile semantics) with a `[ci_strictness]` block listing the job-name
patterns and policy invariants.

The gate ships warn-only until the chassis CI file is reconciled (the
current `--no-artifact` violation on the strict step is fixed in the
companion CI-tightening PR), then promotes to error.

## Consequences

- A future PR cannot accidentally weaken a strict CI job by re-introducing
  `continue-on-error: true` or a `|| true` shell sentinel — the gate
  catches it.
- The standalone gate's coverage on `main` PRs is enforced; structural
  drift is caught before merge.
- The CI-tightening companion PR formalizes the workflow changes the gate
  expects: removing `continue-on-error` flags, removing `--no-artifact`,
  expanding `release-standalone-gate` triggers.

## Alternatives considered

- **Keep the standalone shell utility as-is, no new gate.** Rejected: the
  utility has no rule ID and is not in the release-gate suite, so it is
  invisible to `binding-link` and the diagnostic catalog. A gate makes the
  invariant first-class.
- **Tighten `.github/workflows/` and call it done without a gate.**
  Rejected: a one-time tightening is not an invariant; the next PR can
  weaken it. The gate makes the invariant durable.

## References

- ADR-0010 (posture model): the configuration neighbor
  (`config/chassis/posture.toml`) where the new `[ci_strictness]` block
  lives.
- ADR-0023 (structural integrity policy): adjacent self-discipline gate
  family.
- `scripts/ci/check-strict-ci-pass-throughs.sh`: existing manual check
  the gate wraps.

## Status

Accepted. Implementation lands as Gate G plus the companion CI-tightening
PR in the chassis self-governance hardening plan.
