# chassis

Typed metadata vocabulary and verifiable-adherence layer for spec-driven AI development. Provides stable identifier conventions (rule IDs, claim IDs, ADR IDs), a five-rung assurance ladder (`declared → coherent → verified → enforced → observed`), and the shape of a signed attestation artifact for releases.

Designed as a complement to [GitHub Spec Kit](https://github.com/github/spec-kit), not a competitor: Spec Kit captures intent, `chassis` proves the code still honors it. Planned surface — trace graph (spec ↔ code ↔ test), drift detection, breaking-change diff, exemption registry with hard expiry, and an MCP server for direct agent integration. Scope: **Rust + TypeScript only** (see `docs/adr/ADR-0001-project-scope-and-positioning.md`).

**Status: pre-alpha kernel.**

- `crates/chassis-core/` — Rust types + JSON Schema validators. `cargo check` and `cargo test` pass (Rust ≥ 1.85).
- `packages/chassis-types/` — TypeScript types generated from the 8 canonical JSON Schemas.
- 8 canonical schemas under `schemas/` validate; `fixtures/happy-path/` validates against them.
- No CLI binary yet; no MCP server yet; not yet published to crates.io or npm.

See `CLAUDE.md` for the current state of the tree and the immediate next work.
