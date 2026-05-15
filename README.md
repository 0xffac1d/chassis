# chassis

Typed metadata vocabulary and verifiable-adherence layer for spec-driven AI development. Provides stable identifier conventions (rule IDs, claim IDs, ADR IDs), a five-rung assurance ladder (`declared → coherent → verified → enforced → observed`), and the shape of a signed attestation artifact for releases.

Designed as a complement to [GitHub Spec Kit](https://github.com/github/spec-kit), not a competitor: Spec Kit captures intent, `chassis` proves the code still honors it. The supported surface today is a trace graph (spec ↔ code ↔ test), drift scoring against git history, breaking-change contract diff, DSSE-signed release attestation, and an exemption registry with hard expiry. Scope: **Rust + TypeScript only** (see `docs/adr/ADR-0001-project-scope-and-positioning.md`).

**Status: pre-alpha kernel.** Not yet published to crates.io or npm.

## Workspace

| Crate / package | Status | What it ships |
|---|---|---|
| `crates/chassis-core/` | supported | Rust kernel: validators, contract diff, exemption registry, fingerprint, trace graph, drift report, DSSE attestation. `cargo test` clean on Rust ≥ 1.86. |
| `crates/chassis-cli/` (binary: `chassis`) | supported | Subcommands `validate`, `diff`, `exempt verify`, `trace`, `drift`, `export`, `release-gate`, `attest sign`, `attest verify`. |
| `crates/chassis-jsonrpc/` (binary: `chassis-jsonrpc`) | **experimental** | Newline-delimited JSON-RPC 2.0 server over stdio exposing six methods (`validate_contract`, `diff_contracts`, `trace_claim`, `drift_report`, `release_gate`, `list_exemptions`). Every emitted diagnostic validates against `schemas/diagnostic.schema.json`; `release_gate` returns the same predicate shape (`schemas/release-gate.schema.json`) the CLI writes. **Not** an MCP implementation — a real MCP surface (lifecycle + `tools/list` + `tools/call`) is future work, see `docs/future-mcp.md`. |
| `packages/chassis-types/` (npm `@chassis/core-types`) | supported | 25 generated `.d.ts` modules (17 root schemas + 8 kind subschemas), plus committed `dist/`, `fingerprint.sha256`, `manifest.json`. |

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

`chassis release-gate` bundles validate + trace + drift + exemption + (optional) attestation into one
invocation when you want a single pass/fail verdict; the steps above are the granular form.

### Release-grade attestation key policy

`chassis attest sign` and `chassis release-gate --attest` **fail closed** when no signing key is present — release-grade attestations are never signed by implicit throwaway keys.

- **Default (release-grade).** Pass `--private-key <path>`, or place an Ed25519 secret key (64 hex chars) at `.chassis/keys/release.priv` (gitignored). If neither is found the command exits non-zero with rule `CH-ATTEST-KEY-MISSING`.
- **Ephemeral (demos/tests only).** Pass `--ephemeral-key` to generate a fresh keypair on the fly. The CLI writes the matching public key next to the envelope (`<envelope>.ephemeral.pub`), marks the result `release_grade: false`, emits a `WARNING` on stderr, and stamps the DSSE signature with `keyid: ephemeral:<fingerprint>`. **Never ship this artifact as a release-grade attestation.**
- `--ephemeral-key` and `--private-key` are mutually exclusive.

Verification is symmetric: `chassis attest verify` needs the public key it expects to see (`--public-key <path>` or `.chassis/keys/release.pub`). A correct key passes; the wrong key or a tampered payload exits with `CH-ATTEST-VERIFY-FAILED` (exit code 6).

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
