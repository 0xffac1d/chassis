# Project context

`chassis` is a typed metadata vocabulary plus JSON-Schema validators for spec-driven AI development. It is the **verifiable-adherence layer**: given a spec/intent artifact (typically captured by [GitHub Spec Kit](https://github.com/github/spec-kit)), `chassis` answers whether the code still honors it — via a trace graph (spec ↔ code ↔ test), drift detection, breaking-change diff, signed attestation, and an exemption registry with hard expiry. See `docs/adr/ADR-0001-project-scope-and-positioning.md` for the scope decision and `README.md` for the elevator pitch.

The repo started as a salvage of a prior, much larger codebase (also called Chassis). The audit kept ~15% — the load-bearing vocabulary and validator kernel — and discarded the rest. Salvage + fixup are complete; the kernel compiles and its tests pass. Build-out from here is forward-only.

## Scope

**Rust + TypeScript only.** Python/Go/C# codegen targets from the original are out of scope. See ADR-0001.

## What's in the tree

| Path | Status | Role |
|------|--------|------|
| `schemas/` | canonical | 8 JSON Schemas — contract, ADR, exemption-registry, coherence-report, diagnostic, authority-index, tag-ontology, field-definition |
| `crates/chassis-core/` | builds, tested | Rust types + `StaticValidator` + `CanonicalMetadataContractValidator`. `cargo check` + `cargo test` both clean on Rust ≥ 1.85 (verified on 1.95). 4 unit tests pass. |
| `packages/chassis-types/` | builds | TypeScript `.d.ts` generated from the 8 canonical schemas via `json-schema-to-typescript`. `dist/` is committed; rebuild with `npm run build`. |
| `fixtures/happy-path/` | valid | `rust-minimal` and `typescript-vite`. The `rust-minimal` CONTRACT.yaml is exercised by `chassis-core`'s integration test. |
| `fixtures/adversarial/` | reference | `invalid-schema` — intentionally fails validation (exercised by `chassis-core`'s negative-fixture test) |
| `docs/adr/` | active | New ADRs for this project. Currently: ADR-0001 (scope + positioning). |
| `docs/STABLE-IDS.md`, `docs/ASSURANCE-LADDER.md` | active | Load-bearing vocabulary docs |
| `reference/python-cli/` | reference only | Original Python implementations; semantic spec for the Rust/TS implementations to come. **`mcp_server.py` is the highest-priority study target** — the primary integration path forward is an MCP server. |
| `reference/schemas-extended/` | reference | Original schemas for component / api / data / service / event / state — design input for kind-discriminated subschemas |
| `reference/adrs-original/` | reference | 32 historical ADRs. Reference only; re-author as new ADRs in `docs/adr/` if still binding. |
| `reference/artifacts/release-gate.example.json` | reference | Shape of the attestation artifact this project should produce |
| `reference/docs-original/` | reference | Historical and process docs (AGENTS, DECISIONS, PROTOCOL, OBJECTIVES-REGISTRY, ROADMAP from the prior project; HISTORY and CONTRACT-SCHEMA-LOOSENESS-SURVEY from this project's setup). Do not rely on any command, path, or decision here without verifying against the current tree. |
| `reference/fixtures-deferred/` | reference | `illegal-layout` and `brownfield-messy` — fixtures awaiting machinery that doesn't exist yet (layout validator, `chassis bootstrap`). Move back to `fixtures/` when their enforcers land. |

## Contract schema status

`schemas/contract.schema.json` is intentionally loose right now: 7 required fields, 74 total properties. The plan is to tighten it via a `kind`-discriminated `oneOf` against the subschemas in `reference/schemas-extended/`. See `reference/docs-original/CONTRACT-SCHEMA-LOOSENESS-SURVEY.md`. This is the largest single piece of upcoming schema work.

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

1. Tighten `schemas/contract.schema.json` to ~10 required base fields + `oneOf` discriminator on `kind`. Subschemas in `reference/schemas-extended/` are the design input.
2. Stand up a TypeScript CLI scaffold under `packages/chassis-cli/` that wraps `chassis-core` (via NAPI or WASM) and exposes the planned subcommands.
3. Implement the MCP server in TypeScript, using `reference/python-cli/mcp_server.py` as the semantic spec.
4. Publish `chassis-core` to crates.io and `@chassis/core-types` to npm.
5. Ship the Spec Kit extension package.

## History

`reference/docs-original/HISTORY.md` is the narrative of how the tree got into its current shape (salvage extraction + compile-blocker fixup). Describes what was done; not a backlog.
