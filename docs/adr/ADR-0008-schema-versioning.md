---
id: ADR-0008
title: Schema versioning policy — every schema gets `version`, semver, deprecation flow
status: accepted
date: "2026-04-19"
enforces:
  - rule: SCHEMA-VERSION-MISSING
    description: "A schemas/**/*.schema.json file lacks a top-level `version` field."
  - rule: SCHEMA-VERSION-MALFORMED
    description: "A schema's `version` field does not match ^\\d+\\.\\d+\\.\\d+$."
  - rule: SCHEMA-DEPRECATION-WITHOUT-MIGRATION
    description: "A schema marked deprecated lacks a docs/chassis/guides/schema-migrations/<schema-id>-vN-vM.md guide."
  - rule: SCHEMA-BREAKING-CHANGE-WITHOUT-MAJOR-BUMP
    description: "A schema's content changed in a backwards-incompatible way (additionalProperties tightening, required-field addition, enum removal) without a major version bump."
  - rule: COMPLETENESS-INVARIANT-PARSE-ERR
    description: "completeness gate could not parse a CONTRACT.yaml's invariants block (malformed YAML or schema-incompatible shape)."
  - rule: COMPLETENESS-INVARIANT-MISSING-LINKAGE
    description: "An invariant or edge_case has no test_linkage entry referencing its claim_id."
  - rule: COMPLETENESS-API-STALE
    description: "A public API symbol is documented in CONTRACT.exports but no longer exists in source."
  - rule: COMPLETENESS-API-COVERAGE
    description: "A public API symbol exists in source but is not declared in CONTRACT.exports (and the manifest does not opt out via drift.skip_exports)."
  - rule: PROCESS-RATIO-SNAPSHOT-RECORDED
    description: "A new baseline snapshot was recorded; informational only."
  - rule: PROCESS-RATIO-NO-BASELINE
    description: "No baseline snapshot exists for the meta/product LoC ratio; baseline is recorded on this run."
  - rule: PROCESS-RATIO-META-GROWTH-EXCESSIVE
    description: "Meta/process code (scripts/, docs/, gates/) grew significantly faster than product code since the baseline; risks process-over-product imbalance."
applies_to:
  - "schemas/**/*.schema.json"
  - "scripts/chassis/gates/schema_version.py"
  - "scripts/chassis/gates/completeness.py"
  - "scripts/chassis/gates/process_ratio.py"
tags:
  - chassis
  - governance
  - schemas
  - versioning
---

# ADR-0008: Schema versioning policy

## Context

The 73 JSON Schemas under `schemas/` carry no internal version field.
Breaking changes can land silently — a tightened `additionalProperties`,
a new required field, a removed enum value — and downstream consumers
have no signal that they need to migrate. This ADR pins the versioning
strategy and registers the rule IDs for the planned
`schema_version.py` gate.

This ADR also registers the rule IDs for the `completeness` and
`process_ratio` gates, because both speak to the same concern: keeping
the contract surface in sync with the code surface.

## Decision

1. **Every schema has a `version` field at the top level.** Format:
   semver (`^\d+\.\d+\.\d+$`). Initial value: `1.0.0` for shipped
   schemas; `0.x.y` for internal-only or experimental schemas.
2. **Semver semantics for schema changes.**
   - **Major bump:** breaking changes — a previously-valid instance
     becomes invalid. Examples: tightening `additionalProperties` from
     `true` to `false`, adding a new `required` field, removing an
     enum value, narrowing a `pattern`.
   - **Minor bump:** backwards-compatible additions — new optional
     property, new enum value, new `examples` entry, new `oneOf`
     branch.
   - **Patch bump:** description / docstring changes only; no
     validation impact.
3. **Catalog.** `schemas/CATALOG.yaml` (planned, C1) lists every
   schema's id, current version, and supersedes chain.
4. **Migration guides.** Every major version bump requires a
   migration guide at `docs/chassis/guides/schema-migrations/<schema-id>-v<N>-v<M>.md`.
   Template: `docs/chassis/guides/schema-migrations/_template.md`.
5. **Deprecation.** A schema marked deprecated stays parseable for
   one major version of the consuming codegen tree; after that,
   removal is allowed.
6. **Enforcement.** The `schema_version.py` gate (planned, C1) checks
   git diff: any modification to a `schemas/**/*.schema.json` file
   without a corresponding `version` bump fails CI.

## Consequences

- Consumers can pin to a major version and trust that breaking
  changes are visible in their dependency-update PR.
- Authors are forced to think about backwards compatibility before
  every schema edit. This is intentional.
- The `version` field adds 1 line per schema; trivial cost.

## Status

Accepted. Versioning gate scheduled in C1 of the chassis maturity
plan.
