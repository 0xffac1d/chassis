# Assurance ladder

Five rungs. A claim advances rungs only when a verifier emits evidence at that rung. The contract author **declares** intent; the system **verifies** the rest.

**MVP scope (per ADR-0001):** only `declared` is implementable today via `chassis-core`'s `CanonicalMetadataContractValidator`. The other four rungs require infrastructure (coherence walker, test-runner integration, runtime enforcement point, telemetry pipeline) that does not yet ship from this repo. The `chassis <verb>` invocations below describe the intended verifier surface; the verbs `validate`, `trace`, `drift`, `export`, `release-gate`, and `attest sign|verify` already ship in `crates/chassis-cli/` against the `chassis` binary, but none of them advance a claim past `declared` on their own.

## declared
The claim exists in a contract and passes JSON Schema validation against `schemas/contract.schema.json`. No semantic check.
Verifier today: `chassis validate <path>` (the shipped CLI), which calls `chassis_core::validators::CanonicalMetadataContractValidator`.
Evidence: a contract file with the claim ID present.

## coherent
The claim resolves consistently across the repository: every referenced ADR, test file, code symbol, and exemption resolves. No dangling refs.
Verifier (planned): `chassis coherence`. Not implemented; the trace graph from `chassis trace` is a prerequisite but is not, by itself, a coherence check.
Evidence: a coherence-report.json with zero unresolved refs for this claim.

## verified
At least one test references this claim ID via `test_linkage.claim_id` and that test passes in CI.
Verifier (partial): `chassis trace` joins claim IDs to source and test files today; the test-runner integration that closes the loop (pass/fail per claim) does not ship yet.
Evidence: a passing test result tagged with the claim ID, captured in the attestation artifact.

## enforced
A runtime check (admission controller, sidecar guard, CI gate) actively rejects code or actions that violate the claim. Not just a test that the violation *could* happen — an enforcement point that prevents it from happening in the first place.
Verifier: the enforcement point itself, emitting evidence on every block. Not implemented.
Evidence: enforcement-event log with the claim ID and the rejected input.

## observed
Production telemetry confirms the claim holds at runtime — not in CI, not in tests, in production.
Verifier: telemetry pipeline. Not implemented.
Evidence: observed-event log with the claim ID and a sample window.

## Promotion rules
- A claim is at the lowest rung for which evidence exists in the current release attestation.
- Rungs do not skip: a claim cannot be `enforced` without first being `verified`.
- Rungs can demote: if a `verified` claim's test starts failing, it demotes to `coherent`.
- Demotion is logged. The attestation artifact records the rung at the moment of release, not the historical max.
- `chassis-core::diff` enforces `CH-DIFF-ASSURANCE-DEMOTED` so a demotion in `CONTRACT.yaml` never lands silently.
