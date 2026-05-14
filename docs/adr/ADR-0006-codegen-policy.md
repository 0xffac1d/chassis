---
id: ADR-0006
title: Codegen and generated-artifact policy — committed trees are drift-gated, on-demand trees stay under generated/
status: accepted
date: "2026-04-19"
enforces:
  - rule: CODEGEN-COMMITTED-NON-RUST
    description: "A generated/<lang>/ tree is committed to the repository (on-demand outputs under generated/ must stay .gitignored; committed canonical trees live outside generated/ — crates/chassis-schemas/ for Rust, codegen/ts-types/dist/ for @chassis/types per ADR-0020)."
  - rule: CODEGEN-DRIFT-RUST
    description: "`chassis codegen --check --lang rust` reports drift between emitter output and committed crate."
  - rule: CODEGEN-EMITTER-MISSING
    description: "A schema referenced by another schema or by codegen output is missing from schemas/."
  - rule: CODEGEN-DUPLICATE-SCHEMA-TREE
    description: "A schemas/ subtree is duplicated under crates/ or generated/ (use include_str! references instead)."
  - rule: DECOMPOSITION-LOC-FILE-CAP
    description: "A source file exceeds the per-file LoC cap recorded in config/chassis/baselines/decomposition.json."
  - rule: DECOMPOSITION-LOC-CRATE-CAP
    description: "A Rust crate exceeds the per-crate LoC cap recorded in config/chassis/baselines/decomposition.json."
  - rule: DECOMPOSITION-GRAPH-EDGE-MISSING
    description: "A `use` or import statement crosses a crate boundary that is not declared in Cargo.toml dependencies or architecture.yaml."
  - rule: DECOMPOSITION-API-GROWTH
    description: "A crate's exported API surface grew beyond the configured budget without an ADR justifying the growth."
applies_to:
  - "scripts/chassis/codegen/**"
  - "crates/chassis-schemas/**"
  - "generated/**"
  - "schemas/**"
tags:
  - chassis
  - codegen
  - decomposition
---

# ADR-0006: Codegen and generated-artifact policy

## Status note (2026-04-23 update)

Decision #1 below stated "Only the Rust crate `crates/chassis-schemas/`
is committed." That was the policy at the time, but **a second
committed canonical tree was added**: `codegen/ts-types/dist/` and
`codegen/ts-types/fingerprint.sha256` (the `@chassis/types` npm
package build), per ADR-0020 and its 2026-04-23 `@chassis/types`
distribution update. Consumers vendoring Chassis that depend on
`file:./vendor/chassis/codegen/ts-types`
resolve the package at install time with no chassis build step; a
committed `dist/` is required for that path to work from a fresh
clone of the vendored tree.

The `generated/<lang>/` policy below is **unchanged** —
`codegen/ts-types/` is a separately-namespaced npm package build, not
a `generated/<lang>/` tree, and the `CODEGEN-COMMITTED-NON-RUST` rule
still scopes only to `generated/<lang>/`. The decomposition rule IDs
(`DECOMPOSITION-*`) are unaffected.

Canonical taxonomy now lives in [`REPO_BOUNDARY.md`](../../REPO_BOUNDARY.md)
§ Generated artifacts (three classes). Drift for `@chassis/types` is
gated by the `ts-types-build` job in `.github/workflows/ci.yml`.

## Context

Chassis ships JSON Schemas plus per-language emitters that generate
typed bindings (Rust, TypeScript, Python, C#, Go, TypeSpec). Without a
clear policy on what's committed vs on-demand, generated artifacts
sprawl across the repo and `git diff` becomes useless. The policy was
loosely documented in REPO_BOUNDARY.md; this ADR pins it.

This ADR also doubles as the registration ADR for the `decomposition`
gate's specific rule IDs, since module decomposition discipline is
adjacent to codegen output discipline (both concern what code lives
where).

## Decision

1. **Committed canonical trees.** Two committed generated trees, each
   drift-gated:
   - `crates/chassis-schemas/` is the canonical in-repo Rust consumer
     of the domain schemas. Default output for `chassis codegen --lang
     rust` writes there.
   - `codegen/ts-types/dist/` + `codegen/ts-types/fingerprint.sha256`
     are the `@chassis/types` npm package build, added per ADR-0020's
     2026-04-23 update so sibling consumers using `file:` install
     resolve the package without a build step.
2. **All other languages (TS/Python/C#/Go/TypeSpec) are on-demand.**
   Output under `generated/<lang>/` stays gitignored — that path
   pattern is the specific scope of `CODEGEN-COMMITTED-NON-RUST`.
   Consumers regenerate in their own pipelines. `codegen/ts-types/`
   is outside `generated/` and is not affected by this rule.
3. **No duplicate schema tree.** The Rust crate references
   repo-root schemas via `include_str!("../../../../schemas/...")`.
   Duplicating `schemas/domain/` under `crates/chassis-schemas/` is a
   `CODEGEN-DUPLICATE-SCHEMA-TREE` violation, enforced by
   `validate-distribution-layout`.
4. **Drift gate is part of release-gate.** `chassis codegen --check
   --lang rust` is enforced (no longer advisory) — the emitter and the
   committed crate must agree byte-for-byte. The `ts-types-build` job
   in `.github/workflows/ci.yml` enforces the same contract for
   `codegen/ts-types/dist/` (rebuild + `git diff --exit-code` +
   `verify-fingerprint.mjs` + `npm pack` artifact-inclusion check),
   re-run at release time by the `generated-artifacts` group of
   `chassis release-standalone-gate`.
5. **Decomposition baselines.** Per-file and per-crate LoC caps live
   in `config/chassis/baselines/decomposition.json` and are enforced
   by the decomposition gate. Caps are baseline-snapshotted, not
   absolute — adopters set their own ceilings.

## Consequences

- The repo stays small (no committed JS/Python output).
- Cross-language consumers regenerate in their own CI; the cost is
  borne by the consumer, not the substrate.
- Decomposition gate findings now resolve to specific rule IDs
  (`DECOMPOSITION-LOC-FILE-CAP`, etc.) instead of the
  `DECOMPOSITION-GENERIC` fallback.

## Status

Accepted. The Rust emitter rewrite that resolved the prior drift
shipped in Milestone F.1; this ADR documents the resulting policy.
