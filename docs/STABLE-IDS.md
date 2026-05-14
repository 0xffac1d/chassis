# Stable identifier conventions

The IDs that bind contracts, code, tests, ADRs, and exemptions together. These conventions are load-bearing: claim IDs in particular are stable across edits — the prose of an invariant may change, the ID does not.

## Rule IDs
Format: `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`
Examples: `CH-RULE-0007`, `CHASSIS-VALIDATE-SCHEMA`, `RUNTIME-EXEMPTIONS-SCAFFOLD`
Every diagnostic `ruleId` must resolve to an ADR's `enforces[].rule` entry.

## Claim IDs (invariants, edge_cases)
Format: `^[a-z][a-z0-9_.-]*$` — kebab-case, dot-namespaced.
Examples: `standalone.vendor-neutral`, `cli.repo-root-resolution`, `auth.no-anon-write`.
Used by `test_linkage.claim_id` to bind tests to invariants.

## ADR IDs
Format: `ADR-NNNN` (4+ digits zero-padded).
Examples: `ADR-0008`, `ADR-0021`.

## Exemption IDs
Format: `EX-YYYY-NNNN` (year + counter).
Example: `EX-2026-0001`.
Hard limits enforced by the registry: max 90-day lifetime per entry, max 25 active entries repo-wide. CODEOWNERS-gated.

## Assurance levels
Five-rung ladder: `declared → coherent → verified → enforced → observed`.
Each rung is whatever its verifier outputs, not what the contract author claims.
See `ASSURANCE-LADDER.md` for promotion semantics.
