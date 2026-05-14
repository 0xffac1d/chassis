# CONTRACT.schema looseness survey (Wave 1 inventory)

Source: `schemas/metadata/contract.schema.json` **v1.0.2**.

| Area | Previously loose | Replacement (now or planned) |
|------|-------------------|-------------------------------|
| Component slices | `states`, `accessibility`, `theme`, `props`, `events`, `slots`, `responsive` | `$ref` into **`schemas/component/component.schema.json`** fragments |
| Endpoint slices | `auth`, `request`, `response`, `rateLimit`, `cache`, `idempotency`, `pagination` | **`schemas/api/endpoint.schema.json`** |
| Entity slices | `fields`, `relationships`, `indexes`, `timestamps`, `versioning` | **`schemas/data/entity.schema.json`** + **`schemas/common/field-definition.schema.json`** |
| Service slices | `resilience`, `healthCheck`, `sla` | **`schemas/service/service.schema.json`** |
| Event slices | `payload`, `delivery` | **`schemas/event/event.schema.json`** |
| Store slices | `shape`, `actions`, `selectors`, `persistence`, `sync` | **`schemas/state/store.schema.json`** |
| Feature flag | `type`, `defaultValue`, `targeting`, `metrics`, `expiration` | **`schemas/config/feature-flag.schema.json`** |
| Inputs / outputs | `additionalProperties: true` bags | Candidate: **`schemas/domain/data-contract.schema.json`** (future tightening) |
| Inference evidence rows | Mixed `additionalProperties: true` | Future **`schemas/metadata`** `$defs` or dedicated provenance schema |

See **[schema-versioning.md](schema-versioning.md)** for migration expectations.
