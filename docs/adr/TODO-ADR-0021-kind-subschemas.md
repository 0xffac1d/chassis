---
id: TODO-ADR-0021
title: "Per-kind subschemas — defer payload constraints to schemas/contract-kinds/"
status: stub
date: "2026-05-14"
supersedes: []
applies_to:
  - "schemas/contract.schema.json"
  - "schemas/contract-kinds/*.schema.json"
tags:
  - schemas
  - wave-2
---

> **Stub.** Promote to `ADR-0021-kind-subschemas.md` in the Wave 2 close-out pass.

## Context

Wave 1 tightened `schemas/contract.schema.json` so every `kind` carries a small required-field set, but those fields themselves were shallow (`request: {}` validated). The schema file ballooned to ~3,700 lines of duplicated branch bodies. Wave 2 deepens each kind's payload while making the parent schema small and discriminator-only.

## Decision

### Kinds that got deeper subschemas

All eight kinds picked up a per-kind subschema file under `schemas/contract-kinds/`:

| Kind            | New required (in addition to base) | Notes |
| --------------- | ---------------------------------- | ----- |
| `library`       | `exports`                          | each export is `{path, kind ∈ function|type|module|macro|trait|constant, description?}` |
| `cli`           | `entrypoint`, `argsSummary`        | optional structured `subcommands[]` |
| `component`     | `props`, `events`, `slots`, `states` | structured arrays; `ui_taxonomy`/`accessibility`/`dependencies` relaxed to optional |
| `endpoint`      | `method`, `path`, `auth`, `request`, `response` | `request`/`response` carry `{content_type, schema_ref?, status_code?}`; `request_examples`/`response_examples` optional |
| `entity`        | `fields`, `relationships`          | `indexes` and `timestamps` are now optional |
| `service`       | `protocol`, `endpoints`, `consumes`, `produces` | `resilience` optional/open object |
| `event-stream`  | `source`, `payload`, `delivery`, `consumers` | `payload` carries `{format ∈ json|avro|protobuf|raw, schema_ref?}`; `delivery` is a flat enum (`at-least-once`/`at-most-once`/`exactly-once`/`unknown`) |
| `feature-flag`  | `type`, `defaultValue`, `targeting`, `metrics` | `type ∈ bool|string|number|json`; `targeting` is `{rules[], default_variation}` |

### `additionalProperties: false` decision

**Kept**, but **at the per-kind subschema layer** rather than the parent. Each per-kind subschema enumerates the full property set (base + kind-specific) and sets `additionalProperties: false` plus a `patternProperties` carve-out for `^x-` vendor extensions. The parent `contract.schema.json` no longer sets `additionalProperties` on the per-kind branches — it just discriminates by `kind` const and `allOf`-references the subschema.

This avoids the Draft-7 trap where `additionalProperties` and `$ref`/`allOf` don't compose: each `additionalProperties` only sees its own schema's `properties`, so a parent `additionalProperties: false` would reject properties added by the `$ref`'d subschema.

### Version bump

`schemas/contract.schema.json`: **2.0.0 → 3.0.0** per ADR-0008. Every non-library kind grew at least one new required field, and library now requires structured export objects (not strings) — both are breaking changes per ADR-0008's MAJOR rule (previously valid instance becomes invalid). No companion `contract.v2.schema.json` is shipped because the salvage-era 2.x line had no public consumers; the `chassis.contract-schema-kind-discriminated` self-invariant remains satisfied.

### Resource registration in chassis-core

`crates/chassis-core/src/contract.rs` switches from `jsonschema::validator_for(&schema)` to `jsonschema::options().with_resource(uri, …)` (one call per per-kind subschema) so the in-memory validator resolves `./contract-kinds/<kind>.schema.json` $refs against embedded `include_str!`'d content. No filesystem or network access at validation time.

### Kinds not deepened beyond the brief

`service.resilience` is kept as an open object (`additionalProperties: true`) — `reference/schemas-extended/service/service.schema.json` has a much richer shape (circuit breaker, retry policy, bulkhead) but Wave 2's calibration says "minus the most controversial / domain-specific bits". Deepening can land in Wave 3 once enforcement points exist.

## Consequences

- `@chassis/types` `Contract` union now type-checks against structured payloads (consumer fixture exercises a structured `LibraryExport`).
- `CONTRACT.yaml` at the repo root and `fixtures/happy-path/{rust-minimal,typescript-vite}/CONTRACT.yaml` migrated their `exports` from `string[]` to `{path, kind}[]`.
- Seven new happy-path fixtures cover the previously library-only kind matrix.
- Two new inline adversarial tests cover (a) a CLI missing the kind-specific `entrypoint`, and (b) an entity using a non-enum `relationships[].kind`.

## Followups

- Wave 3 / TODO-ADR-0022 (or similar): deepen `service.resilience` once retry/timeout enforcement lands.
- Promote this stub to `ADR-0021` and link from `MEMORY`/wave plan once the Wave 2 close-out pass runs.
