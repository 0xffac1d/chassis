# chassis

Typed metadata vocabulary and verifiable-adherence layer for spec-driven AI development. Provides stable identifier conventions (rule IDs, claim IDs, ADR IDs), a five-rung assurance ladder (`declared → coherent → verified → enforced → observed`), and the shape of a signed attestation artifact for releases.

Designed as a complement to [GitHub Spec Kit](https://github.com/github/spec-kit), not a competitor: Spec Kit captures intent, `chassis` proves the code still honors it. Planned surface — trace graph (spec ↔ code ↔ test), drift detection, breaking-change diff, exemption registry with hard expiry, and an MCP server for direct agent integration. Scope: **Rust + TypeScript only** (see `docs/adr/ADR-0001-project-scope-and-positioning.md`).

**Status: pre-alpha kernel.**

- `crates/chassis-core/` — Rust types + JSON Schema validators. `cargo check` and `cargo test` pass (Rust ≥ 1.86).
- `packages/chassis-types/` (**npm `@chassis/core-types`**) — TypeScript types generated from **16 canonical JSON Schema modules** (8 root schemas plus 8 kind subschemas under `schemas/contract-kinds/`).
- All canonical schemas under `schemas/` compile; happy-path/adversarial `CONTRACT.yaml` fixtures validate as documented in `fixtures/`.
- No CLI binary yet; no MCP server yet; not yet published to crates.io or npm.

See `CLAUDE.md` for the current state of the tree and the immediate next work.
