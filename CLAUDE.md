# Project context

This project was extracted from a prior codebase ("Chassis"). Audit determined ~85% of the original was scope-bloat or competed unfavorably with GitHub Spec Kit; this directory contains only the salvageable kernel. Scope: **Rust + TypeScript only**. The rewrite positions this as the *verifiable-adherence layer* for spec-driven AI development — a complement to Spec Kit, not a competitor to it.

## What's here

- `schemas/` — 8 canonical JSON Schemas. The contract schema is currently loose (74 fields, 6 required); the rewrite will tighten it via a kind-discriminated `oneOf`.
- `crates/chassis-core/` — Rust types + JSON Schema validators. Compiles on Rust ≥ 1.85.
- `packages/chassis-types/` — TypeScript types extracted from the prior `@chassis/types` codegen output.
- `reference/python-cli/` — original Python implementations for semantic reference only. **`mcp_server.py` is the highest-priority study target** — the rewrite's primary integration is an MCP server.
- `reference/schemas-extended/` — schemas for component / api / data / service / event / state. Reference for designing kind-discriminated subschemas.
- `reference/artifacts/release-gate.example.json` — shape of the attestation artifact the rewrite should produce.
- `fixtures/` — happy-path (rust + ts), adversarial (invalid-schema + illegal-layout), and brownfield-messy.
- `docs/STABLE-IDS.md`, `docs/ASSURANCE-LADDER.md` — the two highest-value vocabulary documents.
- `docs/REFS-TO-FIX.md` — broken `$ref` paths logged during extraction, deferred to the rewrite.
- `docs/EXTRACTION-NOTES.md` — anomalies logged during this extraction.

## What was deliberately dropped (do not re-import)

- `chassis-runtime` crate — admitted scaffolding; exemption registry returned `Denied` unconditionally.
- `chassis-capability-derive` proc macro — emitted only a doc-comment HTML marker.
- Agent rule-file emission (`chassis emit agent-surface`) — replaced by an MCP server in the rewrite.
- Python / Go / C# codegen targets — out of scope.
- 19 governance gates → keep only schema-validate + contract-diff + attestation-integrity in the rewrite.
- ~75 of 95 CLI subcommands — the new CLI surface is roughly: `validate`, `attest`, `trace`, `diff`, `exempt`, `doctor`.

## Strategic position

Spec Kit owns *intent capture*. This project owns *verifiable adherence* — the trace graph (spec ↔ code ↔ test), drift detection, signed attestation, breaking-change diff, exemption registry with hard expiry, MCP-based agent integration. Distribute as a Spec Kit extension (their catalog has 70+) on day one.

## Build status as of extraction

- Rust: `cargo check` result logged in `docs/EXTRACTION-NOTES.md`.
- TypeScript: build scripts stubbed; `npm install` not run.
- Python reference: not expected to run; imports unrewired.

## Immediate next work

1. Tighten `schemas/contract.schema.json` to ~10 required base fields + `oneOf` discriminator on `kind`.
2. Resolve the broken `$ref`s logged in `docs/REFS-TO-FIX.md`.
3. Stand up the TypeScript CLI scaffold (`packages/chassis-cli/`) wrapping `chassis-core` via NAPI or WASM.
4. Reimplement the MCP server in TypeScript using `reference/python-cli/mcp_server.py` as the semantic spec.
5. Publish `chassis-core` to crates.io and `@chassis/core-types` to npm.
6. Ship the Spec Kit extension package.
