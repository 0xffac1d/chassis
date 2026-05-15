# @chassis/core-types

TypeScript type definitions generated from the canonical chassis JSON Schema tree (`schemas/**/*.schema.json`). The package ships **types only** (`require("@chassis/core-types")` resolves to `{}`; NodeNext users import types from the package specifier).

**Status: pre-alpha.** Not published to npm yet. Until publication, consume via a local `file:` dependency against this directory.

## Install (local checkout)

```json
{
  "dependencies": {
    "@chassis/core-types": "file:./vendor/chassis/packages/chassis-types"
  }
}
```

`dist/`, bundled `manifest.json`, and `fingerprint.sha256` are committed — a checkout is installable even before rebuilding (though developers should still run `npm run build` when schemas change).

## Install (packed tarball)

```bash
cd packages/chassis-types
npm pack
# consumer:
npm install --save-dev /path/to/chassis-core-types-0.1.0.tgz
```

## Usage

```ts
import type { Contract, Adr, ExemptionRegistry } from '@chassis/core-types';
```

## What gets generated

- **26 `.d.ts` modules** mapped one-to-one from `schemas/**/*.schema.json`: 18 root schemas (`adr`, `authority-index`, `cedar-facts`, `coherence-report`, `contract`, `diagnostic`, `drift-report`, `dsse-envelope`, `eventcatalog-metadata`, `exemption-registry`, `field-definition`, `in-toto-statement-v1`, `opa-input`, `policy-input`, `release-gate`, `spec-index`, `tag-ontology`, `trace-graph`) plus 8 kind subschemas under `dist/contract-kinds/` (`cli`, `component`, `endpoint`, `entity`, `event-stream`, `feature-flag`, `library`, `service`). The committed `manifest.json` lists all 26 schema sources with their SHA-256 digests.
- A barrel [`dist/index.d.ts`](dist/index.d.ts) re-exports namespaces for every leaf module and collision-free top-level aliases where names are globally unique.

## Build locally

```bash
cd packages/chassis-types
npm ci --prefer-offline
npm run build
```

Steps:

1. `scripts/gen-types.mjs` — walks every `*.schema.json`, runs [`json-schema-to-typescript`](https://github.com/bcherny/json-schema-to-typescript).
2. `scripts/fingerprint-schemas.mjs` — writes **`fingerprint.sha256`** plus a canonical **`manifest.json`** used by downstream consumers (`verify-fingerprint.mjs`) when no repo `schemas/` tree is present.

`CHASSIS_REPO_ROOT` overrides repo-root inference for fingerprint + codegen.

After schema edits, rebuild and recommit `dist/`, `fingerprint.sha256`, and **`manifest.json`**.

## Tests

From `packages/chassis-types/`:

```bash
npm ci
npm test           # tarball manifest assertions + consumer typecheck + installed-package verifier
npm run typecheck  # consumer fixture only (tsc --noEmit)
```

## Drift protection

Consumers can pin types to a schema fingerprint (ADR-0015):

```bash
node node_modules/@chassis/core-types/scripts/verify-fingerprint.mjs
```

The script prefers a live `<repo>/schemas/**` tree (dev checkouts); published installs hash the bundled `manifest.json`.

## License

MIT OR Apache-2.0.
