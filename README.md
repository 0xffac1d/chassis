# chassis

Typed metadata vocabulary and verifiable-adherence layer for spec-driven AI development. Provides stable identifier conventions (rule IDs, claim IDs, ADR IDs), a five-rung assurance ladder (`declared → coherent → verified → enforced → observed`), and signed attestation artifacts for releases.

Intended as a complement to [GitHub Spec Kit](https://github.com/github/spec-kit): Spec Kit captures intent, `chassis` proves the code still honors it. Trace graph (spec ↔ code ↔ test), drift detection, breaking-change diff, exemption registry with hard expiry, and an MCP server for direct agent integration.

**Status: pre-alpha extraction.** Reference implementation in `crates/chassis-core/` (Rust) and `packages/chassis-types/` (TypeScript). Not yet on crates.io or npm. See `CLAUDE.md` for the current state of the rebuild.
