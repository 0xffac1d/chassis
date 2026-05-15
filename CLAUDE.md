# Project context

`chassis` is a typed metadata vocabulary plus JSON-Schema validators for spec-driven AI development. It is the **verifiable-adherence layer**: given a spec/intent artifact (typically captured by [GitHub Spec Kit](https://github.com/github/spec-kit)), `chassis` answers whether the code still honors it — via a trace graph (spec ↔ code ↔ test), drift detection, breaking-change diff, signed attestation, and an exemption registry with hard expiry. See `docs/adr/ADR-0001-project-scope-and-positioning.md` for the scope decision and `README.md` for the elevator pitch.

The repo started as a salvage of a prior, much larger codebase (also called Chassis). The audit kept ~15% — the load-bearing vocabulary and validator kernel — and discarded the rest. The kernel, CLI, JSON-RPC kernel surface, contract diff, trace graph, drift score, and DSSE-signed attestation are all in tree and tested.

## Scope

**Rust + TypeScript only.** Python/Go/C# codegen targets from the original are out of scope. See ADR-0001.

## What's in the tree

| Path | Status | Role |
|------|--------|------|
| `schemas/` | canonical | 18 root metadata schemas plus 8 per-kind contract branches under `schemas/contract-kinds/` (contract parent at `schemas/contract.schema.json` v3.x). 26 schema files total. |
| `crates/chassis-core/` | supported | Rust types + validators + `diff/` (contract-diff) + `exempt/` (registry verifier) + `fingerprint.rs` + `spec_index/` (Spec Kit index + linker) + `trace/` + `drift/` + `exports.rs` (export-only governance facts) + `attest/` (DSSE). `cargo check` + `cargo test` clean on Rust ≥ 1.86 (`rust-toolchain.toml`). |
| `crates/chassis-cli/` | supported | Binary `chassis`. Subcommands: `validate`, `diff`, `exempt verify`, `trace [--mermaid]`, `drift`, `export`, `spec-index export|validate|link`, `release-gate`, `attest sign`, `attest verify`. `release-gate` / `attest sign` use `chassis_core::gate::compute` so the JSON-RPC `release_gate` method, CLI JSON, on-disk `release-gate.json`, and DSSE predicates stay aligned (including optional `artifacts/spec-index.json` linkage fields on the predicate). (`doctor` is planned, not implemented.) |
| `crates/chassis-jsonrpc/` | **experimental** | Binary `chassis-jsonrpc`. Newline-delimited JSON-RPC 2.0 over stdio, six methods (`validate_contract`, `diff_contracts`, `trace_claim`, `drift_report`, `release_gate`, `list_exemptions`). **Not** a Model Context Protocol implementation — a real MCP surface is future work and is intentionally out of scope for the kernel surface. |
| `policy/` | supported | OPA/Rego release policy (`package chassis.release`) evaluated over `chassis export --format opa`. Wired through `scripts/policy-gate.sh` and the `policy-opa` CI job; Chassis remains an evidence exporter. |
| `packages/chassis-types/` | supported | TypeScript `.d.ts` generated from canonical schemas via `json-schema-to-typescript` (26 leaf modules: 18 root + 8 contract-kinds). `dist/`, `fingerprint.sha256`, and `manifest.json` are committed; rebuild with `npm run build`. |
| `fixtures/happy-path/` | valid | One minimal contract per kind plus `rust-minimal` and `typescript-vite`; exercised by `chassis-core` integration tests. |
| `fixtures/adversarial/` | reference | Surgical invalid contracts per kind + `invalid-schema`; exercised by `chassis-core` validator tests. |
| `fixtures/diff/`, `fixtures/exempt/` | reference | Cases driving `chassis-core::diff` and `chassis-core::exempt` test suites. |
| `fixtures/drift-repo/`, `fixtures/trace-render/` | reference | Bare git fixture for drift tests; expected Mermaid output for trace renderer. |
| `docs/adr/` | active | Foundation ADRs through ADR-0026 (scope, ladder, claims, exemptions, schema semver, fingerprint, diff/exempt/kind rules, attestation envelope, claim annotations, drift scoring, supply-chain policy, Spec Kit index). |
| `docs/STABLE-IDS.md`, `docs/ASSURANCE-LADDER.md` | active | Load-bearing vocabulary docs. |
| `docs/WAVE-PLAN.md` | active | Current delivery plan. |
| `reference/python-cli/` | reference only | Original Python implementations; semantic spec for the Rust implementations now in tree. |
| `reference/schemas-extended/` | reference | Original schemas for component / api / data / service / event / state — design input for the kind-discriminated subschemas now in `schemas/contract-kinds/`. |
| `reference/adrs-original/` | reference | 32 historical ADRs. Reference only; re-author as new ADRs in `docs/adr/` if still binding. |
| `reference/artifacts/release-gate.example.json` | reference | Shape of the attestation artifact the `attest` pipeline produces. |
| `reference/docs-original/` | reference | Historical and process docs from the prior project and from this project's setup. Verify against the current tree before trusting. |
| `reference/fixtures-deferred/` | reference | `illegal-layout` and `brownfield-messy` — fixtures awaiting machinery that doesn't exist yet. |
| `reference/historical/` | reference | Active-doc claims that have aged out of currency. See `reference/historical/README.md`. |

## Contract schema status

`schemas/contract.schema.json` is **tightened**: ten universal required fields (`name`, `kind`, `purpose`, `status`, `since`, `version`, `assurance_level`, `invariants`, `edge_cases`, `owner`) plus kind-specific requirements via `oneOf` across eight kinds (`library`, `cli`, `component`, `endpoint`, `entity`, `service`, `event-stream`, `feature-flag`). Legacy kinds migrate forward by picking the closest supported kind. Claims are structured `{id, text}` only. Schema semver is `3.x` (parent `contract.schema.json` v3.0.0; kind branches refactored to `$ref` per-kind subschemas in `schemas/contract-kinds/`). Export schemas (`policy-input`, `opa-input`, `cedar-facts`, `eventcatalog-metadata`) describe JSON facts for external governance systems; they are not Chassis policy or enforcement engines.

## Assurance ladder MVP

The five-rung ladder (`declared → coherent → verified → enforced → observed`) is documented in `docs/ASSURANCE-LADDER.md`. Per ADR-0001, only `declared` is implementable today (`chassis validate` → `chassis-core::validators::CanonicalMetadataContractValidator`). The other four rungs require infrastructure (coherence walker, test-runner integration, enforcement point, telemetry pipeline) that does not ship from this repo.

## What was deliberately dropped (do not re-import)

- `chassis-runtime` crate — admitted scaffolding; exemption registry returned `Denied` unconditionally.
- `chassis-capability-derive` proc macro — emitted only a doc-comment HTML marker.
- Agent rule-file emission (`chassis emit agent-surface`) — replaced by the JSON-RPC kernel surface (`chassis-jsonrpc`); a real MCP shim is future work (see `docs/future-mcp.md` for the requirements).
- Python / Go / C# codegen targets — out of scope (ADR-0001).
- 19 governance gates → keep only schema-validate + contract-diff + attestation-integrity (plus `release-gate` as the bundled run).
- ~75 of 95 CLI subcommands — the shipped CLI surface is `validate`, `diff`, `exempt verify`, `trace`, `drift`, `export`, `release-gate`, `attest sign`, `attest verify`; `doctor` remains planned only.

Per ADR-0001, the project does not use the word "runtime" in user-facing copy until an enforcement point actually exists.

## Distribution intent

- `chassis-core` → crates.io
- `@chassis/core-types` (renamed from the original `@chassis/types`) → npm
- A Spec Kit extension package on day one (Spec Kit's catalog has 70+ extensions)

## Immediate next work

See `docs/WAVE-PLAN.md`. Highlights:

1. Promote `chassis-jsonrpc` from experimental: either add a real MCP-protocol shim (per `docs/future-mcp.md`) or formalize the custom JSON-RPC surface and version it.
2. Decide on signing transport beyond raw Ed25519 (cosign or in-toto envelope); capture in a Wave-3-close ADR.
3. Add the planned `doctor` subcommand to `chassis-cli`.
4. Stand up a TypeScript CLI alternative (NAPI or WASM) for `chassis-core`, if a Node-only consumer surfaces.
5. Publish `chassis-core` to crates.io and `@chassis/core-types` to npm.
6. Ship the Spec Kit extension package.

## History

`reference/docs-original/HISTORY.md` is the narrative of how the tree got into its current shape (salvage extraction + compile-blocker fixup). Describes what was done; not a backlog. `reference/historical/` is the destination for active-doc claims that have aged out.
