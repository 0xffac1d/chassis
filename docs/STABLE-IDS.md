# Stable identifier conventions

The load-bearing intellectual asset of the original Chassis. Preserve verbatim in the rewrite — these are the IDs that bind contracts, code, tests, ADRs, and exemptions together.

## Rule IDs
Format: `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`
Examples: `CH-RULE-0007`, `CHASSIS-VALIDATE-SCHEMA`, `RUNTIME-EXEMPTIONS-SCAFFOLD`
Every diagnostic `ruleId` must resolve to an ADR's `enforces[].rule` entry.

## Claim IDs (invariants, edge_cases)
Format: `^[a-z][a-z0-9_.-]*$` — kebab-case, dot-namespaced.
Examples: `standalone.vendor-neutral`, `cli.repo-root-resolution`, `auth.no-anon-write`.
Used by `test_linkage.claim_id` to bind tests to invariants. **Stable across rewrites** — the prose of an invariant may change, the ID does not.

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
