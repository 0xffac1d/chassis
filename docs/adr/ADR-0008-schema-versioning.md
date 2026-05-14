---
id: ADR-0008
title: "Schema versioning — semver on every canonical schema, breaking-change policy"
status: accepted
date: "2026-05-14"
enforces:
  - rule: SCHEMA-VERSION-FIELD-REQUIRED
    description: "Every canonical *.schema.json includes top-level `version` semver."
  - rule: SCHEMA-VERSION-BUMP-ON-CHANGE
    description: "Any drift-relevant edit to a schema without the correct semver bump fails CI."
  - rule: SCHEMA-BREAKING-CHANGE-CO-EXISTS
    description: "Breaking contract schema changes ship as additive files (e.g., contract.v2.schema.json) retaining prior major families."
applies_to:
  - "schemas/**/*.schema.json"
tags:
  - foundation
  - schemas
---

## Context

JSON Schemas underpin Rust + TypeScript codegen and validator kernels. Silent breakage erodes consumer trust. `reference/adrs-original/ADR-0008-schema-versioning.md` bundled unrelated gates; this ADR narrows to schema semantics for the salvaged tree.

## Decision

### Required metadata

Every canonical schema file under `schemas/**.schema.json` MUST include a top-level `"version": "MAJOR.MINOR.PATCH"` field (`^\\d+\\.\\d+\\.\\d+$`).

### Semver semantics (schemas)

- **MAJOR++** when a previously valid instance becomes invalid — e.g., adding a `required` field, removing / narrowing `enum` values, tightening `additionalProperties` from permissive to forbidden without carve-outs, removing properties when `additionalProperties` is false, or reshaping a `$ref` target such that transitive validation tightens.
- **MINOR++** when validation relaxes or adds optional capacity — new optional property, new enum case, widening patterns.
- **PATCH++** when prose-only changes occur (`description`, `title`, examples) with **zero** validation impact.

Transitive breakage via `$ref` follows the same rule: if the composed validation tightens consumers, bump the referring schema’s major version **or** introduce a compatibility shim file.

### Shipping breaking changes (parallel majors)

**Parallel majors coexist:** `schemas/contract.schema.json` retains the prior consumer-facing major lineage (e.g., `version: 1.x`) until intentionally retired, while breaking redesigns land as explicitly versioned filenames such as `schemas/contract.v2.schema.json` with `version: 2.0.0`. Tools select via manifest / fingerprint coupling rather than silent overwrite.

Hard cutover is **disallowed** without a major filename + migration notes committed alongside.

### CI gate (spec)

CI compares pull-request trees against the merge base:

1. Enumerate changed `schemas/**/*.schema.json` blobs.
2. For each changed file, ensure the embedded `version` field increments appropriately versus the base commit **or** the change set is classified as patch-only via an allowed hash allowlist mechanism (future automation).
3. Fail if bytes under validation-relevant keys change per fingerprint extractor (`packages/chassis-types/scripts/fingerprint-schemas.mjs` keep-list) without a semver bump.

Exact scripting lands in Wave 2+; the rule ID stabilizes the obligation now.

## Consequences

- Consumers can pin both Cargo/npm semver **and** schema fingerprints (ADR-0015).
- reviewers see explicit major filenames instead of ambiguous drift.

## Relationship to predecessor

Dropped bundled completeness/process gates; clarified parallel-major delivery specific to this repo’s salvage posture.
