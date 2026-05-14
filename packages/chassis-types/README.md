# @chassis/types

TypeScript type definitions generated from the canonical chassis JSON Schemas. Types are the only artifact — the package has no runtime surface (`require("@chassis/types")` returns `{}`).

**Status: pre-alpha.** Not yet published to npm. Until publication, consume via a local `file:` dependency from a vendored chassis checkout.

## Install (local checkout)

```json
{
  "dependencies": {
    "@chassis/types": "file:./vendor/chassis/packages/chassis-types"
  }
}
```

`dist/` and `fingerprint.sha256` are committed, so a fresh clone is installable immediately without running `npm install && npm run build` here first.

## Install (packed tarball)

```bash
cd packages/chassis-types
npm pack
# in the consumer:
npm install --save-dev /path/to/chassis-types-0.1.0.tgz
```

## Usage

```ts
import type { Contract, Adr, ExemptionRegistry } from '@chassis/types';
```

Every schema under `schemas/*.schema.json` in the chassis source tree produces one `.d.ts` under `dist/`. The barrel `dist/index.d.ts` re-exports all generated types.

The 8 current schemas: `contract`, `adr`, `exemption-registry`, `coherence-report`, `diagnostic`, `authority-index`, `tag-ontology`, `field-definition`.

## Build locally

```bash
cd packages/chassis-types
npm install --no-save --prefer-offline
npm run build
```

The build:
1. `scripts/gen-types.mjs` walks `<repo>/schemas/**/*.schema.json`, runs [`json-schema-to-typescript`](https://github.com/bcherny/json-schema-to-typescript) on each, and writes `dist/<name>.d.ts` plus a barrel `dist/index.d.ts`.
2. `scripts/fingerprint-schemas.mjs` writes `fingerprint.sha256` — the SHA-256 of an RFC 8785-canonicalized manifest of the schemas tree.

`CHASSIS_REPO_ROOT` overrides the repo-root resolution (used by the drift check, which copies this package into a temp tree).

After regenerating, commit `dist/` and `fingerprint.sha256`.

## Tests

From `packages/chassis-types/`:

```bash
npm install
npm test           # pack-manifest test + consumer-fixture typecheck
npm run typecheck  # consumer fixture only (tsc --noEmit)
```

## Drift protection

`fingerprint.sha256` lets a consumer detect schema drift between the types they installed and the chassis release they pinned to. To verify in consumer CI:

```bash
node node_modules/@chassis/types/scripts/verify-fingerprint.mjs
```

A mismatch means the installed types were generated from a different schema set than the chassis release in your pin — treat as a breaking-change signal.

## License

MIT OR Apache-2.0.
