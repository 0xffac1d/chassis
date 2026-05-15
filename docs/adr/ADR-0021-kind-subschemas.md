---
id: ADR-0021
title: "Per-kind contract subschemas — payload constraints under schemas/contract-kinds/"
status: accepted
date: "2026-05-14"
supersedes: []
enforces:
  - rule: KIND-SUBSCHEMA-PER-KIND-FILE
    description: "Each supported kind has exactly one canonical subschema file at schemas/contract-kinds/<kind>.schema.json (dash-separated filename matching kind token)."
  - rule: KIND-SUBSCHEMA-REF-FROM-PARENT
    description: "Parent schemas/contract.schema.json discriminates on kind and composes each branch via $ref into the matching per-kind subschema."
  - rule: CH-RUST-METADATA-CONTRACT
    description: "Error. `CanonicalMetadataContractValidator` rejected a CONTRACT JSON instance against kind-discriminated `schemas/contract.schema.json` + per-kind refs."
applies_to:
  - "schemas/contract.schema.json"
  - "schemas/contract-kinds/*.schema.json"
  - "crates/chassis-core/src/contract.rs"
tags:
  - schemas
  - wave-2
---

## Context

Wave 1 tightened `schemas/contract.schema.json` so every `kind` carries a small required-field set, but those fields were shallow (`request: {}` validated). The schema file ballooned to thousands of lines of duplicated branch bodies. Wave 2 deepens each kind's payload while making the parent schema discriminator-only and maintaining semver discipline under ADR-0008.

The root self-governance claims `chassis.contract-schema-kind-discriminated` and `chassis.adversarial-fixture-rejected` are direct checks on this decision: each supported kind must select an explicit subschema, and adversarial contracts must fail canonical validation instead of slipping through the parent schema.

## Decision

### Kinds that received deeper subschemas

All eight kinds have a per-kind subschema file under `schemas/contract-kinds/`:

| Kind            | New required (in addition to base) | Notes |
| --------------- | ---------------------------------- | ----- |
| `library`       | `exports`                          | Each export is `{path, kind ∈ function|type|module|macro|trait|constant, description?}` |
| `cli`           | `entrypoint`, `argsSummary`        | Optional structured `subcommands[]` |
| `component`     | `props`, `events`, `slots`, `states` | Structured arrays; `ui_taxonomy`/`accessibility`/`dependencies` relaxed to optional |
| `endpoint`      | `method`, `path`, `auth`, `request`, `response` | `request`/`response` carry `{content_type, schema_ref?, status_code?}`; examples optional |
| `entity`        | `fields`, `relationships`          | `indexes` and `timestamps` optional |
| `service`       | `protocol`, `endpoints`, `consumes`, `produces` | `resilience` optional open object |
| `event-stream`  | `source`, `payload`, `delivery`, `consumers` | `payload` includes format enum; `delivery` flat enum |
| `feature-flag`  | `type`, `defaultValue`, `targeting`, `metrics` | Typed flag with targeting rules |

### `additionalProperties` at the subschema layer

Each per-kind subschema enumerates the full property set (base + kind-specific), sets `additionalProperties: false`, and uses a `patternProperties` carve-out for `^x-` vendor extensions. The parent `contract.schema.json` does **not** impose `additionalProperties: false` on branches that `$ref` subschemas — avoiding the Draft-07 composition trap between `additionalProperties` and `allOf`/`$ref`.

### Version bump

`schemas/contract.schema.json` moved **2.0.0 → 3.0.0** per ADR-0008: previously valid instances can become invalid (new required fields and structured library exports). No parallel `contract.v2.schema.json` file ships because the salvage-era 2.x line had no external consumers.

### Resource registration in chassis-core

Validation uses `jsonschema` resource registration (`with_resource`) so `./contract-kinds/<kind>.schema.json` resolves against embedded `include_str!` payloads — no filesystem or network I/O at validate time.

### Deferred deepening

`service.resilience` stays an open object. Richer circuit-breaker / retry shapes may arrive in a future wave once enforcement semantics exist.

## Consequences

- Typed `@chassis/core-types` generated contracts reflect structured per-kind payloads.
- Repo-root `CONTRACT.yaml` and minimal fixtures were migrated (e.g. library `exports` as structured rows).
- Seven additional happy-path fixtures cover non-library kinds.
- Validator tests include targeted adversarial cases for CLI and entity kinds.

## Relationship to predecessor

None. New ADR replacing the Wave 2 stub; normative linkage is ADR-0008 for semver and schema versioning.

## Followups

- Future ADR may deepen `service.resilience` once retry/timeout enforcement lands.
