# Multi-wave delivery plan

Staging map. Wave numbers advance when foundational contracts land; later waves may subdivide into multiple PRs without renaming the wave.

## Wave 1 — Foundations ✅

- Kind-discriminated `schemas/contract.schema.json` with semver metadata per ADR-0008.
- ADRs 0001–0016 accepted (Wave 1 set; later waves bring the count to ADR-0026).
- Regenerated Rust + `@chassis/core-types` artifacts + schema fingerprint.
- Repository-root `CONTRACT.yaml` validating against the tightened schema.

## Wave 2 — Contract tooling ✅

- ADR-0017 (Rust fingerprint port) — `crates/chassis-core/src/fingerprint.rs`, parity-tested against `packages/chassis-types/scripts/fingerprint-schemas.mjs`.
- `crates/chassis-core/src/diff/` — contract-diff engine emitting `CH-DIFF-*` diagnostics (ADR-0019).
- `crates/chassis-core/src/exempt/` — exemption registry + sweeper + CODEOWNERS resolver (ADR-0020).
- 8 per-kind subschemas under `schemas/contract-kinds/`; `contract.schema.json` bumped to `3.0.0` (ADR-0021).
- 9 happy-path fixtures (one per kind plus `rust-minimal` and `typescript-vite`), 20 diff fixtures, 14 exempt fixtures.

## Wave 3 — Trace + drift + attestation ✅ (in tree; polish ongoing)

Shipped:

- `crates/chassis-core/src/trace/` — static claim scanner (ADR-0023, supersedes ADR-0005). Walks Rust + TypeScript sources, extracts the canonical `// @claim <id>` line-comment annotation (identical grammar for both languages), builds a graph of `claim_id → contract → implementing_files → covering_tests`. The pre-ADR-0023 TypeScript JSDoc form (`/** @claim ... */`) is rejected with `CH-TRACE-MALFORMED-CLAIM` so it cannot fail silently. JSON + Mermaid output. Rule IDs: `CH-TRACE-*`.
- `crates/chassis-core/src/drift/` — per-claim drift score from git history via `git2` (ADR-0024). Rule IDs: `CH-DRIFT-*`.
- `crates/chassis-core/src/attest/` — DSSE-style attestation envelope: assemble → sign (Ed25519) → verify (ADR-0022). Subject is the schema-manifest digest plus repo metadata. Rule IDs: `CH-ATTEST-*`.
- `crates/chassis-cli/` — single binary `chassis` exposing `validate`, `diff`, `exempt verify`, `trace [--mermaid]`, `drift`, `export`, `spec-index …`, `release-gate`, `attest sign`, `attest verify`.
- `crates/chassis-jsonrpc/` — newline-delimited JSON-RPC 2.0 sidecar exposing six chassis methods (`validate_contract`, `diff_contracts`, `trace_claim`, `drift_report`, `release_gate`, `list_exemptions`). Explicitly **not** the Model Context Protocol; see `docs/future-mcp.md` for the requirements a real MCP surface would have to meet.
- CI job `self-attest` runs `scripts/self-attest.sh` (trace → drift → attest sign → attest verify) and uploads the DSSE artifact.
- OPA policy gate: `policy/chassis_release.rego` plus `scripts/policy-gate.sh` and CI job `policy-opa` evaluate Rego over `chassis export --format opa`. Chassis exports evidence; OPA returns `allow` and `policy-result.json`.

Open polish items:

- Decide on signing transport beyond raw Ed25519 (cosign or in-toto) and capture in a Wave-3-close ADR.
- Add the planned `doctor` subcommand to the CLI surface.

## Wave 3.5 — Spec Kit bridge ✅ (in tree)

- `schemas/spec-index.schema.json` — canonical Spec Kit index wire format (ADR-0008 metadata).
- `docs/spec-kit-chassis-preset.md` + `.chassis/spec-index-source.yaml` — human workflow; `chassis spec-index export` writes deterministic `artifacts/spec-index.json`.
- `crates/chassis-core/src/spec_index.rs` — YAML→JSON export validation, SHA-256 digest, spec-to-contract linker (`CH-SPEC-*`, ADR-0026).
- Policy input (`schemas/policy-input.schema.json`) optional `spec_kit.spec_index_digest`; `chassis export` and `release-gate` consume `artifacts/spec-index.json` when present. Release-gate predicate v1.2 (`schemas/release-gate.schema.json`) records `spec_index_digest`, `spec_failed`, and `spec_error_count` so JSON-RPC, CLI, and DSSE artifacts agree.

## Wave 4 — Operator interfaces

Reality-aligned scope:

- `chassis-cli` is the supported operator surface (shipped Wave 3). Wave 4 work here is feature polish: `doctor`, schema-manifest digest printing, optional `--repo` discovery beyond cwd.
- `chassis-jsonrpc` is the **experimental** machine surface (shipped Wave 3). Wave 4 decision: either add a real Model Context Protocol shim on top of the existing JSON-RPC methods (per the requirements in `docs/future-mcp.md`), or formalize and version the JSON-RPC surface as a chassis-specific protocol. The shim option requires an ADR specifying the MCP method/capability mapping.
- TypeScript CLI is **not** currently planned. If a Node-only consumer needs `chassis-core` capability without the Rust toolchain, the path is a thin wrapper (NAPI or WASM) — captured in a future Wave 4 prep ADR if/when a consumer surfaces.

## Wave 5 — Self-application

In tree:

- `CONTRACT.yaml` at repo root with structured invariants and edge cases, all `declared`.
- `scripts/self-attest.sh` and the `self-attest` CI job exercise the full pipeline against this repo.
- Trace graph + drift report + DSSE envelope are produced for every push.

Remaining:

- Annotate every CONTRACT claim with at least one `@claim` marker in Rust or TypeScript source so the trace graph has no orphans for chassis itself.
- Add at least one real exemption to `.chassis/exemptions.yaml` exercising the registry.
- Promote claims past `declared` once the per-rung verifiers (per `docs/ASSURANCE-LADDER.md`) ship.

## Wave 6 — Supply-chain hygiene ✅ (in tree; ongoing)

- ADR-0025 (`docs/adr/ADR-0025-supply-chain-policy.md`) accepted. Rule IDs: `CH-SUPPLY-LICENSE-ALLOW`, `CH-SUPPLY-ADVISORY-CLEAN`, `CH-SUPPLY-NO-NETWORK-CRATES`, `CH-SUPPLY-ARCHIVE-HYGIENE`.
- `deny.toml` enforces the SPDX license allowlist and the network-crate denylist via cargo-deny.
- `scripts/check-archive-hygiene.sh` rejects build/cache artifacts and stale developer-machine paths from any candidate source archive (driven by `scripts/build-source-archive.sh`).
- `scripts/docs-lint.sh` enforces a forbidden-phrase set on active documentation so stale wording (old monolith paths, the wrong "MCP server" framing, an incorrect schema-module count) cannot land silently. <!-- chassis-lint-allow:mcp-server -->
