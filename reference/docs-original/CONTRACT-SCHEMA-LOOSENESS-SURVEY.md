# `contract.schema.json` looseness survey

Source: [`schemas/contract.schema.json`](../schemas/contract.schema.json) (currently v1.0.1 — 7 required fields, 74 total properties).

Reference subschemas for tightening live in [`reference/schemas-extended/`](../reference/schemas-extended/). They are unreviewed input, not canonical — the tightening pass will redesign them into a tight kind-discriminated `oneOf` rather than promote them as-is.

| Area | Currently loose on contract | Candidate subschema to inform tightening |
|------|-----------------------------|-------------------------------------------|
| Component slices | `states`, `accessibility`, `theme`, `props`, `events`, `slots`, `responsive` | `reference/schemas-extended/component/component.schema.json` |
| Endpoint slices | `auth`, `request`, `response`, `rateLimit`, `cache`, `idempotency`, `pagination` | `reference/schemas-extended/api/endpoint.schema.json` |
| Entity slices | `fields`, `relationships`, `indexes`, `timestamps`, `versioning` | `reference/schemas-extended/data/entity.schema.json` (+ `schemas/field-definition.schema.json`) |
| Service slices | `resilience`, `healthCheck`, `sla` | `reference/schemas-extended/service/service.schema.json` |
| Event slices | `payload`, `delivery` | `reference/schemas-extended/event/event.schema.json` |
| Store slices | `shape`, `actions`, `selectors`, `persistence`, `sync` | `reference/schemas-extended/state/store.schema.json` |
| Feature flag | `type`, `defaultValue`, `targeting`, `metrics`, `expiration` | `reference/schemas-extended/feature-flag.schema.json` |
| Inputs / outputs | `additionalProperties: true` bags | No reference subschema; new design needed |
| Inference evidence rows | Mixed `additionalProperties: true` | No reference subschema; new design needed |

## Tightening plan

Reduce required fields to roughly 10 base properties (`name`, `kind`, `purpose`, `status`, `since`, `invariants`, `edge_cases`, plus a small set of identity fields) and switch the rest of the schema to a `oneOf` discriminated on `kind`. Each branch references one of the subschemas above (after redesign), with `additionalProperties: false` enforced inside each branch.

This is the largest single piece of upcoming schema work — see "Immediate next work" in `CLAUDE.md`.
