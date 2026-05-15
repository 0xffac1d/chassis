# Multi-wave delivery plan

Staging map. Wave numbers advance when foundational contracts land; later waves may subdivide into multiple PRs without renaming the wave.

## Wave 1 — Foundations ✅

- Kind-discriminated `schemas/contract.schema.json` with semver metadata per ADR-0008.
- ADRs 0002–0016 accepted.
- Regenerated Rust + `@chassis/core-types` artifacts + schema fingerprint.
- Repository-root `CONTRACT.yaml` validating against the tightened schema.

## Wave 2 — Contract tooling ✅

- ADR-0017 (Rust fingerprint port) — **`crates/chassis-core/src/fingerprint.rs`** (parity-tested against `packages/chassis-types/scripts/fingerprint-schemas.mjs`).
- `crates/chassis-core/src/diff/` — contract-diff engine emitting `CH-DIFF-*` diagnostics (ADR-0019).
- `crates/chassis-core/src/exempt/` — exemption registry + sweeper + CODEOWNERS resolver (ADR-0020).
- 8 per-kind subschemas under `schemas/contract-kinds/`; `contract.schema.json` bumped to `3.0.0` (ADR-0021).
- 9 happy-path fixtures (one per kind + extra library), 17 diff fixtures, 14 exempt fixtures.

## Wave 3 — Trace + drift + attestation (active)

- **Static claim scanner** (ADR-0005): walks Rust + TypeScript sources, extracts `@claim <id>` annotations, builds a graph of `claim_id → implementing_files → covering_tests → governing_adrs → active_exemptions`. Module: `crates/chassis-core/src/trace/`. Output JSON + Mermaid. Rule IDs: `CH-TRACE-*`.
- **Drift detector**: per-claim drift score from git history (last claim edit vs last impl edit + churn between). Module: `crates/chassis-core/src/drift/`. Rule IDs: `CH-DRIFT-*`. Depends on the trace graph.
- **Attestation emitter**: assembles trace + drift + diff + exempt state into a signed release artifact conforming to `reference/artifacts/release-gate.example.json`. Module: `crates/chassis-core/src/attest/`. Signing: cosign or in-toto (pick in Wave 3 prep ADR). Rule IDs: `CH-ATTEST-*`.

## Wave 4 — Operator interfaces

- Consolidated TypeScript CLI under `packages/chassis-cli/` wrapping `chassis-core` (NAPI or WASM per Wave 4 prep ADR). Subcommands: `validate`, `diff`, `exempt`, `trace`, `drift`, `attest`, `doctor`.
- MCP server (TypeScript) in `packages/chassis-mcp/` exposing `what_governs`, `what_breaks_if_i_change`, `is_exempt`, `validate_contract`. Reference semantics in `reference/python-cli/mcp_server.py`.

## Wave 5 — Self-application

- The chassis repo's own `CONTRACT.yaml` annotated with `@claim` markers in source.
- Trace graph runs against this repo; drift detector runs against this repo; release attestation is generated and signed for chassis itself.
- Exemption registry has at least one real entry covering a known gap.
- Finish-line bar (per `docs/STABLE-IDS.md` and `docs/ASSURANCE-LADDER.md`): chassis describes and verifies itself using its own machinery.
