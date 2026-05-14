# Multi-wave delivery plan

Staging map from Wave 1 onward. Wave numbers advance when foundational contracts land; later waves may subdivide into multiple PRs without renaming the wave.

## Wave 1 — Foundations (complete target: schema + ADRs)

- Kind-discriminated `schemas/contract.schema.json` with semver metadata per ADR-0008.
- ADRs 0002, 0003, 0004, 0005, 0008, 0011, 0015, 0016 accepted.
- Regenerated Rust + `@chassis/types` artifacts + schema fingerprint.
- Repository-root `CONTRACT.yaml` validating against the tightened schema.

## Wave 2 — Contract tooling expansion

- Contract diff engine + CLI surface (`chassis diff`) scoped to semantic compares on stable IDs.
- Exemption registry CLI (`chassis exempt`) implementing ADR-0004 quotas.
- Deeper per-kind subschemas (props/events payloads, endpoint auth matrices) once trace inputs stabilize.

## Wave 3 — Trace graph + attestation

- Static claim scanners (ADR-0005) feeding a trace graph (spec ↔ code ↔ test).
- Release attestation emitter validating against `reference/artifacts/release-gate.example.json` lineage.

## Wave 4 — Operator interfaces

- Consolidated CLI wrapper (`validate`, `trace`, `attest`, `doctor`).
- MCP server (TypeScript) following `reference/python-cli/mcp_server.py` semantics without importing Python.

## Wave 5 — Self-application

- Run chassis gates against this repository as dogfood (coverage + coherence once verifiers exist).
