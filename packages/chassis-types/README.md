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

- **16 `.d.ts` modules** mapped from matching schema paths (examples: root `schemas/contract.schema.json`, kinds under `schemas/contract-kinds/*.schema.json`, plus metadata schemas such as `diagnostic`, `adr`, etc.).
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
