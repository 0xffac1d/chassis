---
id: ADR-0017
title: "Schema fingerprint in Rust — canonical port + CI parity with Node reference"
status: accepted
date: "2026-05-14"
enforces:
  - rule: FINGERPRINT-RUST-PORT-CANONICAL
    description: "chassis-core exposes the schema fingerprint algorithm; Rust output is authoritative for in-process diff/trace consumers."
  - rule: FINGERPRINT-IMPLEMENTATIONS-AGREE
    description: "CI runs Node fingerprint-schemas.mjs and the Rust implementation on every push and asserts identical hex digest."
applies_to:
  - "crates/chassis-core/**"
  - "packages/chassis-types/scripts/fingerprint-schemas.mjs"
  - "packages/chassis-types/scripts/canonicalize.mjs"
  - "packages/chassis-types/fingerprint.sha256"
tags:
  - foundation
  - supply-chain
  - rust
---

## Context

ADR-0015 defines the schema fingerprint as a SHA-256 over a canonical manifest of per-schema subjects. The committed reference implementation lives in `packages/chassis-types/scripts/fingerprint-schemas.mjs` (with `canonicalize.mjs`). Wave 2 adds contract-diff and related tooling that runs inside `chassis-core`; those callers must detect schema vocabulary drift without assuming the TypeScript package directory is present at runtime.

Two viable approaches:

- **Option A — Rust port:** Implement the same algorithm in `chassis-core` (e.g. `chassis_core::fingerprint`), byte-identical to the Node reference.
- **Option B — Node-only:** Read `packages/chassis-types/fingerprint.sha256` from disk; no second implementation.

## Decision

Chassis adopts **Option A**.

### Canonical placement

The Rust implementation MUST live under `chassis-core` (recommended module path: `crates/chassis-core/src/fingerprint.rs`, exported as `chassis_core::fingerprint`). It MUST reproduce ADR-0015’s manifest construction and canonical serialization semantics:

1. Enumerate `schemas/**/*.schema.json` in the same sorted order as `fingerprint-schemas.mjs`.
2. Extract each file’s subject using the same keep-list as `KEEP_KEYS` in `fingerprint-schemas.mjs`.
3. Canonicalize subjects and the manifest with behavior equivalent to `canonicalize.mjs` (JCS-shaped deterministic UTF-8 JSON).
4. Emit the same lowercase hex SHA-256 digest as the Node script’s stdout / `fingerprint.sha256` file content.

### Reference vs authoritative

`fingerprint-schemas.mjs` remains the **reference implementation** for readability and for npm workflows (`npm run build`). The Rust port is **canonical for programmatic consumers inside the Rust kernel** (diff, future gates). Neither implementation may drift from the other.

### CI gate (`FINGERPRINT-IMPLEMENTATIONS-AGREE`)

On every push, CI MUST:

1. Run `node packages/chassis-types/scripts/fingerprint-schemas.mjs` (or equivalent) from the repo root and capture the fingerprint line.
2. Run the Rust parity check (e.g. `cargo test -p chassis-core fingerprint` or a small `cargo run` harness) that computes the same digest from the same tree.
3. Fail if the two digests differ.

The committed `packages/chassis-types/fingerprint.sha256` MUST still match fresh Node recomputation when schemas or fingerprint logic changes (ADR-0015 `SCHEMA-FINGERPRINT-CI-VERIFIED`).

## Consequences

- `chassis-core` binaries and libraries do not depend on `packages/chassis-types/` being co-installed on disk for fingerprint identity.
- Two implementations must be maintained; CI absorbs the cost and prevents silent divergence.

## Relationship to ADR-0015

ADR-0015’s identity definition and normalization rules are unchanged. This ADR chooses **where** the algorithm runs for Rust tooling and adds an explicit cross-language parity obligation.
