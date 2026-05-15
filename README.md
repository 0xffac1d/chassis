# chassis

Typed metadata vocabulary and verifiable-adherence layer for spec-driven AI development. Provides stable identifier conventions (rule IDs, claim IDs, ADR IDs), a five-rung assurance ladder (`declared → coherent → verified → enforced → observed`), and the shape of a signed attestation artifact for releases.

Designed as a complement to [GitHub Spec Kit](https://github.com/github/spec-kit), not a competitor: Spec Kit captures intent, `chassis` proves the code still honors it. The supported surface today is a trace graph (spec ↔ code ↔ test), drift scoring against git history, breaking-change contract diff, DSSE-signed release attestation, and an exemption registry with hard expiry. Scope: **Rust + TypeScript only** (see `docs/adr/ADR-0001-project-scope-and-positioning.md`).

**Status: pre-alpha kernel.** Not yet published to crates.io or npm.

## Workspace

| Crate / package | Status | What it ships |
|---|---|---|
| `crates/chassis-core/` | supported | Rust kernel: validators, contract diff, exemption registry, fingerprint, trace graph, drift report, DSSE attestation. `cargo test` clean on Rust ≥ 1.86. |
| `crates/chassis-cli/` (binary: `chassis`) | supported | Subcommands `validate`, `diff`, `exempt verify`, `trace`, `drift`, `export`, `attest sign`, `attest verify`. |
| `crates/chassis-jsonrpc/` (binary: `chassis-jsonrpc`) | **experimental** | Newline-delimited JSON-RPC 2.0 server over stdio exposing six methods (`validate_contract`, `diff_contracts`, `trace_claim`, `drift_report`, `release_gate`, `list_exemptions`). **Not** an MCP implementation; a real MCP surface is future work. |
| `packages/chassis-types/` (npm `@chassis/core-types`) | supported | 20 generated `.d.ts` modules (12 root schemas + 8 kind subschemas), plus committed `dist/`, `fingerprint.sha256`, `manifest.json`. |

All canonical schemas under `schemas/` resolve locally; happy-path and adversarial `CONTRACT.yaml` fixtures validate as documented in `fixtures/`.

## Golden path

Self-attestation against this repository (run by `scripts/self-attest.sh` and CI):

```bash
chassis validate CONTRACT.yaml
chassis trace --json > trace.json
chassis drift --json > drift.json
chassis attest sign --private-key .chassis/keys/release.priv --out release-gate.dsse
chassis attest verify --public-key .chassis/keys/release.pub release-gate.dsse
```

## Governance Exports

`chassis export` emits JSON facts for systems that already own policy decisions. It is an export surface, not a policy language or enforcement engine.

```bash
chassis export --format chassis      # repo facts, contracts, claims, diagnostics, exemptions, drift summary
chassis export --format opa          # wraps the Chassis facts as OPA input JSON
chassis export --format cedar        # Cedar-style entity/action/resource facts
chassis export --format eventcatalog # service/message metadata from service and event-stream contracts
```

The EventCatalog-style adapter only uses data Chassis already has in `service` and `event-stream` contracts. Chassis does not emit OpenLineage run events because the current model has metadata, not runtime job/run telemetry.

`./scripts/verify-foundation.sh` is the local pre-push gate (Rust fmt/clippy/check/test, Node build + fingerprint parity + tests).

See `CLAUDE.md` for what each tree path holds and `docs/WAVE-PLAN.md` for current work.
