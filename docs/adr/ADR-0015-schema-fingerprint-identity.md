---
id: ADR-0015
title: "Schema fingerprint identity — reproducible SHA-256 over canonical schema subjects"
status: accepted
date: "2026-05-14"
enforces:
  - rule: SCHEMA-FINGERPRINT-CANONICAL-IMPL
    description: "Fingerprint computation MUST match packages/chassis-types/scripts/fingerprint-schemas.mjs (canonical subject + JCS + manifest)."
  - rule: SCHEMA-FINGERPRINT-CI-VERIFIED
    description: "Committed fingerprint.sha256 must equal fresh recomputation or CI fails."
applies_to:
  - "schemas/**/*.schema.json"
  - "packages/chassis-types/fingerprint.sha256"
  - "packages/chassis-types/scripts/fingerprint-schemas.mjs"
tags:
  - foundation
  - supply-chain
---

## Context

Consumers need a deterministic identity for “which chassis vocabulary did we compile against?” distinct from Cargo semver alone. `reference/adrs-original/ADR-0015-schema-fingerprint-identity.md` described Python + release manifest artifacts; this repository ships a Node reference implementation co-located with TypeScript typings.

## Decision

### Identity definition

The **schema fingerprint** is the lowercase hex SHA-256 digest of the RFC 8785-style canonical JSON encoding (via `canonicalize.mjs`) of a manifest object:

```json
{
  "version": 1,
  "kind": "chassis-schemas-manifest",
  "count": <N>,
  "entries": [
    {"path": "schemas/adr.schema.json", "sha256": "<digest>"},
    ...
  ]
}
```

Each entry digest hashes the canonical encoding of the **schema subject** extracted per the keep-list in `fingerprint-schemas.mjs` (`$id`, `type`, `required`, `properties`, … — exactly the keys enumerated there, no additions).

### Normalization rules

Two implementations are equivalent iff:

1. They enumerate `schemas/**/*.schema.json` using identical sorted path order (POSIX `/` separators).
2. They parse JSON with UTF-8 decoding **without** BOM tolerance differences (files MUST be UTF-8 without BOM).
3. They apply the same subject extraction filter before canonicalization.
4. They use identical canonical serialization (`canonicalize.mjs`, JCS-inspired deterministic ordering).

### CI gate

`packages/chassis-types/fingerprint.sha256` MUST match `node packages/chassis-types/scripts/fingerprint-schemas.mjs` output on every PR touching `schemas/**` or fingerprint logic.

### When fingerprints change

Any byte change to validation-relevant schema content (post subject extraction) yields a new per-file digest and therefore new manifest digest — **even if** `version` semver did not bump (failure should instead trigger SCHEMA-VERSION-BUMP-ON-CHANGE once CI wiring lands).

## Consequences

- Symlink/path dependency drift becomes auditable: differing bytes surface as fingerprint mismatch.
- Typed consumers (@chassis/types) share an objective pin alongside semver.

## Relationship to predecessor

Replaces Python-centric manifest filenames with the in-repo Node reference while preserving the “schema wins over opaque semver” philosophy for consumers.
