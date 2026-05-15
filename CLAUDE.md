# Project context

`chassis` is a typed metadata vocabulary plus JSON-Schema validators for spec-driven AI development. It is the **verifiable-adherence layer**: given a spec/intent artifact (typically captured by [GitHub Spec Kit](https://github.com/github/spec-kit)), `chassis` answers whether the code still honors it — via a trace graph (spec ↔ code ↔ test), drift detection, breaking-change diff, signed attestation, and an exemption registry with hard expiry. See `docs/adr/ADR-0001-project-scope-and-positioning.md` for the scope decision and `README.md` for the elevator pitch.

The repo started as a salvage of a prior, much larger codebase (also called Chassis). The audit kept ~15% — the load-bearing vocabulary and validator kernel — and discarded the rest. Salvage + fixup are complete; the kernel compiles and its tests pass. Build-out from here is forward-only.

## Scope

**Rust + TypeScript only.** Python/Go/C# codegen targets from the original are out of scope. See ADR-0001.

## What's in the tree

| Path | Status | Role |
|------|--------|------|
| `schemas/` | canonical | Base metadata schemas plus eight per-kind contract branches under `schemas/contract-kinds/` (contract parent at `schemas/contract.schema.json` v3.x). |
| `crates/chassis-core/` | builds, tested | Rust types + validators + `diff/` (contract-diff) + `exempt/` (registry verifier). `cargo check` + `cargo test` clean on Rust ≥ 1.86 (`rust-toolchain.toml`). |
| `packages/chassis-types/` | builds | TypeScript `.d.ts` generated from canonical schemas (contract + kinds + metadata) via `json-schema-to-typescript`. `dist/` is committed; rebuild with `npm run build`. |
| `fixtures/happy-path/` | valid | One minimal contract per kind (`*-minimal`) plus `typescript-vite`; `rust-minimal` is exercised by `chassis-core` integration tests. |
| `fixtures/adversarial/` | reference | Surgical invalid contracts per kind + `invalid-schema`; exercised by `chassis-core` validator tests. |
| `docs/adr/` | active | Foundation ADRs through ADR-0021 (scope, ladder, claims, exemptions, schema semver, fingerprint, diff/exempt/kind rules). |
| `docs/STABLE-IDS.md`, `docs/ASSURANCE-LADDER.md` | active | Load-bearing vocabulary docs |
| `reference/python-cli/` | reference only | Original Python implementations; semantic spec for the Rust/TS implementations to come. **`mcp_server.py` is the highest-priority study target** — the primary integration path forward is an MCP server. |
| `reference/schemas-extended/` | reference | Original schemas for component / api / data / service / event / state — design input for kind-discriminated subschemas |
| `reference/adrs-original/` | reference | 32 historical ADRs. Reference only; re-author as new ADRs in `docs/adr/` if still binding. |
| `reference/artifacts/release-gate.example.json` | reference | Shape of the attestation artifact this project should produce |
| `reference/docs-original/` | reference | Historical and process docs (AGENTS, DECISIONS, PROTOCOL, OBJECTIVES-REGISTRY, ROADMAP from the prior project; HISTORY and CONTRACT-SCHEMA-LOOSENESS-SURVEY from this project's setup). Do not rely on any command, path, or decision here without verifying against the current tree. |
| `reference/fixtures-deferred/` | reference | `illegal-layout` and `brownfield-messy` — fixtures awaiting machinery that doesn't exist yet (layout validator, `chassis bootstrap`). Move back to `fixtures/` when their enforcers land. |

## Contract schema status

`schemas/contract.schema.json` is **tightened**: ten universal required fields (`name`, `kind`, `purpose`, `status`, `since`, `version`, `assurance_level`, `invariants`, `edge_cases`, `owner`) plus kind-specific requirements via `oneOf` across eight kinds (`library`, `cli`, `component`, `endpoint`, `entity`, `service`, `event-stream`, `feature-flag`). Legacy kinds (`crate`, `package`, …) migrate forward by picking the closest supported kind. Claims are structured `{id, text}` only. Schema semver is `3.x` starting Wave 2 (`contract.schema.json` v3.0.0, kind branches refactored to `$ref` per-kind subschemas in `schemas/contract-kinds/`); consult `docs/WAVE-PLAN.md` for Wave 3 trace and enforcement work.

## Assurance ladder MVP

The five-rung ladder (`declared → coherent → verified → enforced → observed`) is documented in `docs/ASSURANCE-LADDER.md`. Per ADR-0001, only `declared` is implementable today (JSON Schema validation via `chassis-core`). The other four rungs require additional infrastructure and are deferred.

## What was deliberately dropped (do not re-import)

- `chassis-runtime` crate — admitted scaffolding; exemption registry returned `Denied` unconditionally.
- `chassis-capability-derive` proc macro — emitted only a doc-comment HTML marker.
- Agent rule-file emission (`chassis emit agent-surface`) — replaced by an MCP server in the new design.
- Python / Go / C# codegen targets — out of scope (ADR-0001).
- 19 governance gates → keep only schema-validate + contract-diff + attestation-integrity.
- ~75 of 95 CLI subcommands — the planned CLI surface is roughly: `validate`, `attest`, `trace`, `diff`, `exempt`, `doctor`.

Per ADR-0001, the project does not use the word "runtime" in user-facing copy until an enforcement point actually exists.

## Distribution intent

- `chassis-core` → crates.io
- `@chassis/core-types` (renamed from the original `@chassis/types`) → npm
- A Spec Kit extension package on day one (Spec Kit's catalog has 70+ extensions)

## Immediate next work

1. **Wave 3 — see `docs/WAVE-PLAN.md`:** trace graph (spec ↔ code ↔ test), drift detection over git history, attestation emitter conforming to `reference/artifacts/release-gate.example.json`.
2. Stand up a TypeScript CLI scaffold under `packages/chassis-cli/` that wraps `chassis-core` (via NAPI or WASM) and exposes the planned subcommands.
3. Implement the MCP server in TypeScript, using `reference/python-cli/mcp_server.py` as the semantic spec.
4. Publish `chassis-core` to crates.io and `@chassis/core-types` to npm.
5. Ship the Spec Kit extension package.

## History

`reference/docs-original/HISTORY.md` is the narrative of how the tree got into its current shape (salvage extraction + compile-blocker fixup). Describes what was done; not a backlog.
