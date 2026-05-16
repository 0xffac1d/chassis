# chassis

Typed metadata vocabulary and verifiable-adherence layer for spec-driven AI development. Provides stable identifier conventions (rule IDs, claim IDs, ADR IDs), a five-rung assurance ladder (`declared → coherent → verified → enforced → observed`), and the shape of a signed attestation artifact for releases.

Designed as a complement to [GitHub Spec Kit](https://github.com/github/spec-kit), not a competitor: Spec Kit captures intent, `chassis` proves the code still honors it. Chassis consumes the documented **`yaml-meta` Markdown preset** (ADR-0029), not arbitrary Spec Kit Markdown. The supported surface today is a trace graph (spec ↔ code ↔ test), drift scoring against git history, breaking-change contract diff, DSSE-signed release attestation, and an exemption registry with hard expiry. Scope: **Rust + TypeScript only** (see `docs/adr/ADR-0001-project-scope-and-positioning.md`).

**Status: pre-alpha kernel.** Not yet published to crates.io or npm.

## Workspace

| Crate / package | Status | What it ships |
|---|---|---|
| `crates/chassis-core/` | supported | Rust kernel: validators, contract diff, exemption registry, fingerprint, Spec Kit index + linker (`spec_index`; Markdown bridge accepts **only** the `yaml-meta` preset per ADR-0029), trace graph, drift report, DSSE attestation, SARIF scanner evidence. `cargo test` clean on Rust ≥ 1.86. |
| `crates/chassis-cli/` (binary: `chassis`) | supported | Subcommands `validate`, `diff`, `exempt verify`, `trace` (regex or tree-sitter extractor), `drift`, `export`, `spec-index export|validate|link|from-spec-kit`, `scanner ingest|verify-evidence`, `release-gate`, `attest sign`, `attest verify`. |
| `crates/chassis-jsonrpc/` (binary: `chassis-jsonrpc`) | **experimental** | Newline-delimited JSON-RPC 2.0 server over stdio exposing six methods (`validate_contract`, `diff_contracts`, `trace_claim`, `drift_report`, `release_gate`, `list_exemptions`). Every emitted diagnostic validates against `schemas/diagnostic.schema.json`; `release_gate` returns the same predicate shape (`schemas/release-gate.schema.json`) the CLI writes. **Not** an MCP implementation — a real MCP surface (lifecycle + `tools/list` + `tools/call`) is future work, see `docs/future-mcp.md`. |
| `packages/chassis-types/` (npm `@chassis/core-types`) | supported | 26 generated `.d.ts` modules (18 root schemas + 8 kind subschemas), plus committed `dist/`, `fingerprint.sha256`, `manifest.json`. |

All canonical schemas under `schemas/` resolve locally; happy-path and adversarial `CONTRACT.yaml` fixtures validate as documented in `fixtures/`.

## Golden path

The end-to-end bundled verifier is `chassis release-gate`. It runs the
canonical-schema preflight on every `CONTRACT.yaml`, builds the trace graph,
computes drift, applies the exemption registry across trace/spec/drift, links
`artifacts/spec-index.json` if present, and (optionally) signs a DSSE-wrapped
in-toto release-gate predicate. Both the CLI and the JSON-RPC `release_gate`
method return the same predicate shape (`schemas/release-gate.schema.json`) and
the same blocking-axis fields, so an agent and a human see identical verdicts.

```bash
# release-grade signing key (gitignored)
mkdir -p .chassis/keys
# bundled verifier — writes dist/release-gate.json and dist/release-gate.dsse
chassis release-gate \
    --fail-on-drift \
    --attest \
    --private-key .chassis/keys/release.priv

# verify the signed envelope
chassis attest verify dist/release-gate.dsse \
    --public-key .chassis/keys/release.pub
```

The granular primitive form — useful for CI or for embedding into another
pipeline — is what `scripts/self-attest.sh` runs to confirm each layer
in isolation before re-running the bundled command:

```bash
mkdir -p self-attest-artifacts
chassis trace --json > self-attest-artifacts/trace-graph.json
chassis drift --json > self-attest-artifacts/drift-report.json
chassis attest sign \
    --private-key .chassis/keys/release.priv \
    --out self-attest-artifacts/release-gate.dsse
chassis attest verify self-attest-artifacts/release-gate.dsse \
    --public-key .chassis/keys/release.pub
```

Both paths produce a `schemas/release-gate.schema.json`-conformant predicate
that records `schema_fingerprint`, `git_commit`, `verdict`, per-axis blocking
flags (`trace_failed`, `drift_failed`, `exemption_failed`, `attestation_failed`,
`spec_failed`), `unsuppressed_blocking`, `suppressed`, `severity_overridden`,
`final_exit_code`, and the `commands_run` log.

**Git checkout required for `release-gate`.** The command expects a Git working tree at the repo root (a `.git` directory and readable `HEAD`): drift compares claim edits to file history, and the predicate includes `git_commit`. Extracted source archives (`git archive` tarballs without `.git`) are **not** `release-gate` runnable — use `git clone` or otherwise preserve checkout metadata. The stable failure id is `CH-GATE-GIT-METADATA-REQUIRED`. Those same archives **are** expected to run `./scripts/verify-foundation.sh` (docs-lint, Rust, Node gates); CI proves this via `source-archive.yml` / `source-archive-pr.yml` extract-and-smoke.

**Default artifact paths.** Without `--out` / `--attest-out`, the CLI writes
predicates and DSSE envelopes under `<repo>/dist/` (gitignored). The root-level
filenames `release-gate.json` and `release-gate.dsse` are reserved for explicit
overrides and remain in `.gitignore` so generated outputs cannot dirty the
working tree.

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

**OPA (Rego)** — Release policy lives in `policy/chassis_release.rego` and is evaluated over `chassis export --format opa` by `./scripts/policy-gate.sh` (also run in CI). When `artifacts/spec-index.json` is present, exports include `spec_kit.spec_index_digest` and merged spec-to-contract linker diagnostics. Chassis stays an evidence exporter; OPA evaluates `allow` and emits `policy-result.json`.

`./scripts/verify-foundation.sh` is the local pre-push gate (Rust fmt/clippy/check/test, diagnostic governance, Node build + schema/fingerprint checks + tests). It is intentionally runnable from a clean `git archive` extract so tarball consumers see the same gate as developers. Optional local hooks live in `.pre-commit-config.yaml`; drift vs `verify-foundation.sh` is enforced by `scripts/verify-pre-commit-parity.sh` (also run inside `foundation.yml`).

### What CI proves

Foundation gates are split by workflow name:

| Workflow | Role |
| --- | --- |
| `foundation.yml` | `verify-foundation` (Ubuntu + macOS) + Node/Rust fingerprint parity; blocks tracked `*.priv`; pre-commit parity. |
| `supply-chain.yml` | `cargo audit`, `cargo deny`, banned deps, PR `dependency-review`. |
| `policy-gate.yml` | OPA over `chassis export --format opa` (`policy/chassis_release.rego`). |
| `self-attest.yml` | `scripts/self-attest.sh` + Cosign keyless sign/verify on `release-gate.dsse`. |
| `source-archive.yml` | On `main`/`master` push: `git archive` + hygiene + GitHub attest-build-provenance + extract-smoke of `verify-foundation.sh`. |
| `source-archive-pr.yml` | On pull requests: tarball + extract-smoke only (reusable SLSA jobs cannot be `if:`-skipped on the caller job). |
| `semgrep.yml` / `codeql.yml` / `renovate-config-validator.yml` | Static analysis + Renovate config schema. |
| `release-evidence.yml` | After the above succeed for a commit, downloads artifacts, re-validates with `validate_artifact`, checks `CH-EVIDENCE-DIGEST-MISMATCH` round-trips, and uploads one `release-evidence-<sha>.tar.gz` bundle. |

See `CLAUDE.md` for what each tree path holds and `docs/WAVE-PLAN.md` for current work.
