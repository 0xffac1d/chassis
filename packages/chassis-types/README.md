# @chassis/types

TypeScript type definitions for the chassis canonical JSON schemas. Types-only — `require('@chassis/types')` returns `{}`.

## Use

```ts
import type { Contract, Adr, Diagnostic, ExemptionRegistry } from '@chassis/types';
import type { Contract_Contract } from '@chassis/types';   // namespaced
```

Generated from `<repo-root>/schemas/*.schema.json`. Current set (8): `contract`, `adr`, `exemption-registry`, `coherence-report`, `authority-index`, `diagnostic`, `field-definition`, `tag-ontology`.

## Install

Not yet published. Use a file dependency:

```json
{ "dependencies": { "@chassis/types": "file:./path/to/chassis/packages/chassis-types" } }
```

## Build & test

```bash
npm install
npm run build    # regenerates dist/ + fingerprint.sha256
npm test         # pack-includes-dist + consumer-typecheck
```

`dist/` and `fingerprint.sha256` are committed so `file:` consumers don't need to build.

## License

MIT OR Apache-2.0
