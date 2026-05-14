# @chassis/types

TypeScript type definitions for the chassis JSON schemas.

This is the **out-of-process wire-contract** path for TypeScript consumers of
chassis. For this Standalone source checkout RC it is **locally packable but
unpublished**: use a `file:` dependency or a tarball produced by `npm pack`.
Do not document `npm install @chassis/types` from a registry until the
standalone gate reports `distribution.npm.published: true`.

The in-process, native-binding path (napi-rs) is still **experimental**: it
ships as `@chassis/runtime-experimental` from
`crates/_experimental/chassis-runtime-napi`, is `"private": true`, and is not
published. See [ADR-0020](../../docs/adr/ADR-0020-ts-distribution-paths.md)
and `crates/_experimental/chassis-runtime-napi/README.md` § "Posture" for the
graduation checklist. Use this package when you talk to a chassis-fronted
service over HTTP/WebSocket/SSE/MCP and need compile-time guarantees on the
wire shape.

## Install From This Checkout

```bash
npm install --save-dev file:/path/to/your/chassis/checkout/codegen/ts-types
```

Use the absolute or workspace-relative path to **`codegen/ts-types/` inside your vendored Chassis distribution** (often `./vendor/chassis/codegen/ts-types` in consumer repos).

Types are the only artifact. There is no runtime surface: `require("@chassis/types")`
returns `{}`.

## Install From A Packed Tarball

```bash
cd /path/to/chassis/codegen/ts-types
npm pack --dry-run
npm pack
cd /path/to/consumer
npm install --save-dev /path/to/chassis/codegen/ts-types/chassis-types-0.1.0.tgz
```

### Consumption via a local `file:` dependency

Downstream repositories typically vendor Chassis and then point `file:` at the
`codegen/ts-types/` directory inside that checkout. Example `package.json`:

```json
{
  "dependencies": {
    "@chassis/types": "file:./vendor/chassis/codegen/ts-types"
  }
}
```

pnpm/npm resolve `main: dist/index.js` and `types: dist/index.d.ts` from this
package's tree at install time. There is no `prepare` or `postinstall` build
step, so `dist/` **must be present on disk** when the consumer runs `pnpm install`.

### dist/ is committed (not CI-built)

`codegen/ts-types/dist/` and `codegen/ts-types/fingerprint.sha256` are checked
into this repository. A fresh clone is immediately usable by consumers via
`file:` without first running `npm install && npm run build` inside this package. This
is deliberate — it keeps the consumer install path synchronous and offline-safe.

A chassis-side CI gate (`codegen-ts-types` in `.github/workflows/ci.yml`)
prevents drift: on every PR and push to main, CI regenerates `dist/` and
`fingerprint.sha256` from the source schemas and fails if the committed output
diverges from the fresh build. The same rebuild + diff is enforced by
`./scripts/chassis/chassis codegen --check` (see [`guides/codegen.md`](../../docs/chassis/guides/codegen.md)).
If your PR touches `schemas/**/*.schema.json`,
you are responsible for regenerating and committing the updated output — see
[Build locally](#build-locally) below.

## Usage

```ts
// Bare top-level names for schemas with globally unique type names.
import type { AgentAction, ApiResponse } from '@chassis/types';

// Namespaced access for every schema (collision-safe).
import type { Agent_AgentAction, Api_ApiResponse } from '@chassis/types';
```

Every schema under `schemas/**/*.schema.json` in the chassis source tree
produces one exported `.d.ts` under `dist/<domain>/<name>.d.ts`. Collisions
(multiple `Policy` schemas, multiple `Capability` schemas) are resolved by
preferring the namespaced form; only schema types with a globally unique name
get a bare-name re-export.

**TypeScript `strict`:** Importing from the package entry point loads the full
barrel; declaration emit from `json-schema-to-typescript` can surface
`skipLibCheck`-level noise in strict workspaces. Prefer `skipLibCheck: true` (or
import from specific `dist/...` modules) if your compiler reports benign index-signature
clashes in generated files — the wire shapes remain authoritative vs JSON Schema.

## Drift protection

Every packed tarball ships a `fingerprint.sha256` alongside `dist/`. The
fingerprint is the SHA-256 of an RFC 8785-canonicalized manifest of the
chassis schemas tree; it is identity-equivalent to the output of

```bash
python3 scripts/chassis/scripts/fingerprint_schemas.py
```

in the chassis repo. The JS implementation under `scripts/fingerprint-schemas.mjs`
is a byte-for-byte port of the Python reference; the hashes match or the port
is broken.

Consumer CI should run

```bash
node node_modules/@chassis/types/scripts/verify-fingerprint.mjs
```

as a drift gate, or compare `fingerprint.sha256` against the consumer's own
`.chassis-schema.sha256` pin (see
[ADR-0015](../../docs/adr/ADR-0015-schema-fingerprint-identity.md)). A
mismatch means the types you installed were generated from a different schema
set than the chassis release you're pinned to — treat as a breaking-change
signal.

## Tests (package maintainer)

From `codegen/ts-types/`:

```bash
npm install
npm test                 # pack manifest includes dist/ + consumer fixture typecheck
npm run typecheck        # only the consumer fixture (`tsc --noEmit`)
```

These are also covered by `chassis codegen --check` (rebuilds this package in a temp directory and diffs `dist/` + `fingerprint.sha256`).

## Build locally

```bash
cd codegen/ts-types
npm install --no-save --prefer-offline
npm run build
```

This walks `../../schemas/**/*.schema.json` (or the tree at `CHASSIS_REPO_ROOT`
when set, e.g. a temp copy of this package for drift checks), runs
[`json-schema-to-typescript`](https://github.com/bcherny/json-schema-to-typescript),
emits per-schema `.d.ts` modules and a barrel at `dist/index.d.ts`, then
writes `fingerprint.sha256`.

After regenerating, commit the resulting changes under
`codegen/ts-types/dist/` and `codegen/ts-types/fingerprint.sha256`. The
`codegen-ts-types` CI job on `main` runs the same two commands and
`git diff --exit-code` against those paths; drift fails the gate with the
regenerate-then-commit instructions inline in the log.

## Governance

- Owner: chassis team.
- Related blueprint decision: Q3 binding strategy, second TS consumer path.
- Related ADRs: [ADR-0013](../../docs/adr/ADR-0013-runtime-library-split.md)
  (runtime surface), [ADR-0015](../../docs/adr/ADR-0015-schema-fingerprint-identity.md)
  (schema fingerprint identity), [ADR-0020](../../docs/adr/ADR-0020-ts-distribution-paths.md)
  (TS distribution paths).
- License: MIT OR Apache-2.0.
