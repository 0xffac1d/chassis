---
id: ADR-0008
title: "Schema versioning — semver on every canonical schema, breaking-change policy"
status: accepted
date: "2026-05-14"
enforces:
  - rule: SCHEMA-VERSION-FIELD-REQUIRED
    description: "Every canonical *.schema.json includes top-level `version` semver (MAJOR.MINOR.PATCH)."
  - rule: SCHEMA-IDENTITY-FIELDS-REQUIRED
    description: "Every canonical *.schema.json declares `$schema`, `$id`, and `title` as non-empty strings."
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

Every canonical schema file under `schemas/**.schema.json` MUST declare the following top-level fields, each as a non-empty string:

| Field | Purpose |
|-------|---------|
| `$schema` | Meta-schema dialect (e.g. `http://json-schema.org/draft-07/schema#` or `https://json-schema.org/draft/2020-12/schema`). Required so validators select the correct dialect. |
| `$id` | Stable canonical URI for the schema (e.g. `https://chassis.dev/schemas/<name>.schema.json`). Required so consumers can pin identity independently of file path and so `$ref` resolution is unambiguous. |
| `version` | Semver `MAJOR.MINOR.PATCH` matching `^\\d+\\.\\d+\\.\\d+$`. Required so consumers can pin schema semantics. |
| `title` | Human-readable label. Required so generated documentation and diagnostics have a stable display name. |

`$id` and `version` are included in the canonical-subject keep-list consumed by `packages/chassis-types/scripts/fingerprint-schemas.mjs` (ADR-0015), so changes to either propagate into the schema fingerprint. `title` is prose-only and does not affect the fingerprint.

The root self-governance claim `chassis.schemas-self-contained` is part of this schema-versioning contract: canonical schemas must resolve locally so a release cannot silently depend on a mutable external schema.

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

`packages/chassis-types/scripts/verify-schema-metadata.mjs` implements rules SCHEMA-VERSION-FIELD-REQUIRED and SCHEMA-IDENTITY-FIELDS-REQUIRED: every canonical schema is walked and the gate fails if any of `$schema`, `$id`, `version`, `title` is missing or empty, or if `version` does not match the semver pattern. It runs inside `scripts/verify-foundation.sh` and therefore on every CI invocation. SCHEMA-VERSION-BUMP-ON-CHANGE remains a separate Wave 2+ gate.

## Consequences

- Consumers can pin both Cargo/npm semver **and** schema fingerprints (ADR-0015).
- reviewers see explicit major filenames instead of ambiguous drift.

## Relationship to predecessor

Dropped bundled completeness/process gates; clarified parallel-major delivery specific to this repo’s salvage posture.
