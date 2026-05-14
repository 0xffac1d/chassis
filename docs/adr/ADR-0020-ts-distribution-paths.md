---
id: ADR-0020
title: TypeScript distribution paths — napi-rs binding vs generated schema types
status: accepted
date: "2026-04-20"
enforces:
  - rule: TS-DIST-PATH-COLLAPSE
    description: "A single TS package is asked to cover both in-process and out-of-process consumers; the two surfaces have different versioning, licensing, and governance needs and must not collapse into one."
  - rule: TS-TYPES-UNPINNED
    description: "`@chassis/types` ships without a `fingerprint.sha256` sidecar; consumers have no identity to diff against and schema drift is silent."
  - rule: TS-TYPES-HANDWRITTEN-DRIFT
    description: "A chassis consumer hand-writes `.d.ts` copies of schema types; the handwritten copies drift from the schemas and chassis has no build-time signal."
  - rule: TS-NAPI-ARCH-MISMATCH
    description: "The napi binding (currently `@chassis/runtime-experimental`, not published) is shipped for a target matrix that does not cover a supported deployment (e.g. aarch64 Linux for DGX Spark); the in-process path silently falls through to `throw`. Applies only if/when the experimental binding graduates; until graduation the constraint is vacuous because the package is private."
applies_to:
  - "codegen/ts-types/**"
  - "crates/_experimental/chassis-runtime-napi/**"
  - "docs/chassis/guides/napi-binding.md"
  - "docs/adr/ADR-0013-runtime-library-split.md"
  - "docs/adr/ADR-0015-schema-fingerprint-identity.md"
tags:
  - chassis
  - governance
  - distribution
  - typescript
  - binding
  - schema
---

# ADR-0020: TypeScript distribution paths

## Status note (2026-04-23 update)

The in-process napi-rs binding described below is **experimental and
private.** It ships as `@chassis/runtime-experimental`, remains `"private": true`, and is not
part of the standalone Chassis plug-and-play surface. The
`emit_diagnostic` and `coherence_report` entry points are fenced:
one returns an error naming its limitation, the other returns a
wire-visible `experimental: true` sentinel. See
`crates/_experimental/chassis-runtime-napi/README.md` § "Posture" and § "Re-promoting
this crate" for the graduation checklist. The Decision section below
still captures the intended shape; publishing is deferred until the
checklist is closed.

## Status note (2026-04-23 update — `@chassis/types` distribution)

Decision #2 below (`@chassis/types` is generated, `dist/` is
`.gitignore`d, npm tarball is the only persisted artifact) is
**superseded in part**: `codegen/ts-types/dist/` and
`codegen/ts-types/fingerprint.sha256` are now **checked in** so
consumers that depend via `file:./vendor/chassis/codegen/ts-types`
can install without a chassis build
step. The generator is still deterministic; committed output is
drift-gated by the `ts-types-build` job in `.github/workflows/ci.yml`
(rebuild + `git diff --exit-code` + `verify-fingerprint.mjs` +
`npm pack` artifact-inclusion check). The npm-publish pipeline
described in #3 remains the mechanism for external consumers once
the package ships to a registry; local `file:` consumption by consumer repositories
is the additive in-tree case. `chassis/.gitignore` carries an
explicit negation (`!/codegen/ts-types/dist/`) so the top-level
`dist/` rule does not swallow it.

## Context

Blueprint Q3 asks which TypeScript distribution paths chassis should
support, and the question does not have a single answer. Two kinds of
consumer exist, and they have different operating assumptions:

1. **In-process consumer.** A Node-hosted runtime (server, CLI,
   background worker) that wants to *execute* chassis logic — emit
   diagnostics, attest assurance, read a coherence snapshot — without
   a network round-trip. For this consumer the correct shape is a
   native binding that wraps the Rust `chassis-runtime-api` surface
   directly. napi-rs v3 is the mature option: Rspack, Rolldown, and
   SWC all ship this way, the build tooling (`@napi-rs/cli`) is
   production-proven, and the v3 platform loader handles the
   Linux/macOS/Windows/WASI target matrix that DGX Spark and operator
   laptops both need. *As of this ADR's 2026-04-23 update that
   binding ships only as the experimental scaffold described in the
   status note above.*

2. **Out-of-process consumer.** A dashboard, an SDK, a service in
   another language's ecosystem, or a monitoring tool that speaks to
   a chassis-fronted API over HTTP / WebSocket / SSE / MCP. This
   consumer does not execute chassis code; it needs compile-time
   guarantees on the *wire shape* — the JSON objects chassis emits
   and accepts. The authoritative source for that shape is the JSON
   schemas under `schemas/`, not the Rust crate. Generated TypeScript
   types from the schemas are the right surface.

Collapsing these into one package was considered and rejected. An
in-process binding that also vends wire-contract types forces the
out-of-process consumer to install a native binary it will never
load, and forces the in-process consumer to re-verify schema
fingerprints on every release it never touched. A types-only package
that secretly loads a native binding hides the architectural split
the consumer should see in its dependency graph.

Prisma's 6.16 retrospective is relevant nuance: they rewrote their
Rust query engine to TypeScript because the Rust layer was a thin
passthrough over database drivers and the napi boundary cost more
than the Rust work saved. That is **not** the chassis position today.
Chassis runtime work (diagnostic dispatch, coherence analysis, future
runtime validation) is real Rust work, not passthrough. The Prisma
retrospective motivates a yearly review checkpoint, not a pre-emptive
rewrite.

## Decision

1. **Ship both paths as separately-versioned npm packages.**
   - `@chassis/runtime-experimental` (napi-rs) is the in-process surface. It
     exposes the five-function chassis-runtime-api stable surface
     documented in ADR-0013. **It is `private` in the workspace and
     will remain private** until the graduation checklist in
     `crates/_experimental/chassis-runtime-napi/README.md` § "Re-promoting this
     crate" is closed. When (if) published, it pins to the Rust
     `chassis-runtime-api` semver and drops the `-experimental`
     suffix.
   - `@chassis/types` is the out-of-process wire-contract surface. It
     is a types-only npm package. Its `main` resolves to a trivial
     `module.exports = {}` runtime; its value is the generated
     `dist/**/*.d.ts` set plus the barrel `dist/index.d.ts`.
2. **Generated and committed (class-1 canonical artifact).** `@chassis/types`
   is generated by `codegen/ts-types/scripts/gen-types.mjs` from
   `schemas/**/*.schema.json`. The generator is deterministic (sorted
   schema walk, no timestamps in output). *Superseded clause: the
   original 2026-04-20 decision said the `dist/` directory was
   `.gitignore`d and only the npm tarball was persisted. That clause
   is replaced by the 2026-04-23 amendment below.* Today,
   `codegen/ts-types/dist/` and `codegen/ts-types/fingerprint.sha256`
   are checked in as a class-1 committed canonical generated artifact
   so consumer repositories that depend via `file:./vendor/chassis/codegen/ts-types`
   install without running a build step. Drift is gated by the
   `ts-types-build` job in `.github/workflows/ci.yml` and by the
   `generated-artifacts` group of `chassis release-standalone-gate`.
   See the amendment section and `REPO_BOUNDARY.md` § Generated
   artifacts for the authoritative three-class definition.
3. **Fingerprint-based drift protection.** Every published
   `@chassis/types` tarball ships `fingerprint.sha256` alongside
   `dist/`. The fingerprint is byte-identical to the output of
   `scripts/chassis/scripts/fingerprint_schemas.py` (ADR-0015). The
   JS implementation at `codegen/ts-types/scripts/fingerprint-schemas.mjs`
   is a byte-for-byte port of the Python reference; the
   `prepublishOnly` hook verifies the port has not drifted. A
   consumer CI job running `verify-fingerprint.mjs` gets drift
   detection for free.
4. **Two surfaces, two consumer expectations.** The dashboard and
   external SDK prototypes consume `@chassis/types` only. A Node
   server hosting chassis in-process would consume the graduated
   napi binding for the runtime surface and *may also* pull
   `@chassis/types` for wire-contract types at its network edge.
   They are complementary, not competing. *Today only `@chassis/types`
   ships; the napi binding is the experimental scaffold described in
   the status note above.*
5. **Yearly Rust-justification review.** Once a year the chassis
   team re-runs the Prisma 6.16 question: is the Rust work behind
   the napi binding still doing something a TypeScript rewrite could
   not? If the answer ever becomes "no," the ADR is re-opened. A
   `DECISIONS.md` reminder is the only enforcement; the outcome is
   an engineering call, not a gate.

## Amendment (2026-04-23) — three-class generated-artifact policy

After this ADR shipped, consumer repositories
began depending on `@chassis/types` via a relative `file:` specifier rather
than a registry install. A `file:./vendor/chassis/codegen/ts-types` dependency
resolves `main: dist/index.js` and `types: dist/index.d.ts` **at install
time**, with no `prepare`/`postinstall` build step. The original Decision §2
clause ("`dist/` is `.gitignore`d, the tarball is the only persisted
artifact") therefore broke local `file:` installs on a fresh clone: the resolver
would fail because `dist/` did not exist on disk.

The amendment, effective 2026-04-23, restructures the generated-artifact
policy into three non-interchangeable classes:

1. **Committed canonical generated** — checked in, drift-gated in CI:
   - `crates/chassis-schemas/` (Rust schema bindings; drift gate: `chassis codegen --check --lang rust`).
   - `codegen/ts-types/dist/` and `codegen/ts-types/fingerprint.sha256`
     (`@chassis/types` npm package output; drift gate: the
     `ts-types-build` job in `.github/workflows/ci.yml`, plus the
     `generated-artifacts` group of `chassis release-standalone-gate`,
     plus `verify-fingerprint.mjs` on every `npm publish` via
     `prepublishOnly`). The top-level `dist/` rule in `.gitignore` is
     explicitly negated for this path so the class-1 output is not
     accidentally excluded.

2. **Local transient generated** — `.gitignore`d, never committed:
   - `generated/{typescript,python,csharp,go}/` — on-demand output of
     `chassis codegen --lang <lang>` for non-Rust targets.

3. **Package-consumer generated** — lives in the consumer repo, not here:
   - Files produced by `chassis bootstrap` / `chassis adopt` in a target
     repo. These belong to the consumer and must not be backported.

**Do-not-mix rules (enforced by `release-standalone-gate` §
docs-consistency):**

- Do NOT delete or `.gitignore` `codegen/ts-types/dist/`. It is class 1.
- Do NOT commit files under `generated/`. They are class 2.
- Do NOT document "only the Rust crate is committed" in standalone prose;
  such claims conflict with class 1 and will trip the docs-consistency gate.

This amendment supersedes Decision §2 above. All other decisions
(separately-versioned packages, fingerprint-based drift protection, yearly
Rust-justification review) remain in force.

## Consequences

- Consumers pick the path that matches their deployment; neither
  consumer drags in artifacts they do not use.
- Schema drift in chassis surfaces as a one-line diff in
  `@chassis/types@fingerprint.sha256` on publish, and as a
  pre-`prepublishOnly` hard-fail if the JS port of the
  canonicalizer has drifted from the Python reference.
- The napi publishing pipeline remains under `crates/_experimental/chassis-runtime-napi`
  with its own target matrix (see `napi-binding.md`). Release
  engineering documents the two pipelines separately.
- The yearly Rust-justification review keeps the Prisma-shaped
  failure mode on the radar without forcing a pre-emptive rewrite.

## Alternatives considered

- **WASM component model as the primary TS surface.** `wit-bindgen`
  is still explicitly "NOT stable" per its own README; `jco componentize`
  is experimental. Adopting component-model today would ship an
  unstable toolchain as the chassis contract — exactly the class of
  risk chassis exists to refuse. Re-evaluate when component-model
  reaches a stable marker.
- **JSR (JavaScript Registry) for both packages.** JSR is ESM-only
  and explicitly does not host native binaries; the napi binding
  cannot be published there. Splitting the registries (JSR for types,
  npm for the binding) gives consumers two install surfaces for one
  split — net cost exceeds benefit.
- **Manual C FFI + hand-written `.d.ts`.** Zero tooling dependency,
  but hand-written declarations drift from the Rust surface on every
  change and chassis has no build-time signal when they do. Rejected
  on the same principle as
  [TS-TYPES-HANDWRITTEN-DRIFT](#ts-types-handwritten-drift).
- **Single-package Prisma-style rewrite to TypeScript.** Defensible
  only when the Rust work is thin passthrough. Chassis runtime work
  is not; re-evaluating is a yearly decision, not a today decision
  (see #5 above).
- **Collapse into one package with conditional exports.** Appears to
  be one install, but it forces every consumer to carry every
  artifact and blurs the versioning story. Rejected: the split is
  architectural, not cosmetic.

## References

- Blueprint Q3 binding strategy.
- Prisma 6.16 retrospective (Rust query engine removal, 2024):
  justifies yearly review, not immediate rewrite.
- napi-rs v3 adopters: Rspack, Rolldown, SWC, NestJS native core.
- ADR-0013: runtime library split — the napi binding pins to
  `chassis-runtime-api`, not `chassis-runtime`.
- ADR-0014: governance tiers — `@chassis/types` is tier-3 consumer
  fingerprint-only; the napi binding would be tier-2 once graduated
  (currently not applicable — it is experimental and unpublished).
- ADR-0015: schema fingerprint identity — the fingerprint that
  `@chassis/types` embeds and consumers diff.
- `codegen/ts-types/README.md`, `codegen/ts-types/scripts/fingerprint-schemas.mjs`.

## Status

Accepted (decision on the two-path split) with one path partially
active:

- `@chassis/types` 0.1.0 scaffolded at `codegen/ts-types/` with a
  byte-identical JS port of the Python fingerprint. Publishing
  pipeline tracked under release engineering.
- The in-process napi binding ships as `@chassis/runtime-experimental`
  (private, unpublished, stubbed). Not part of the standalone Chassis
  plug-and-play surface. Graduation gated on the checklist in
  `crates/_experimental/chassis-runtime-napi/README.md` § "Re-promoting this crate".

The smoke consumer at `crates/_experimental/chassis-runtime-napi/examples/smoke.ts`
exercises both paths at compile time; it is not a production
entry-point and currently fails at runtime on the napi side by
design (because `emit_diagnostic` deliberately errors out — see
`src/lib.rs`).

### Status update — 2026-04

`chassis-runtime-napi` has been moved to `crates/_experimental/` and
removed from the standalone workspace. The standalone Chassis distribution
no longer depends on or builds it. The crate remains in-tree for the team
continuing the napi-rs workstream. Re-promotion follows the checklist in
`crates/_experimental/chassis-runtime-napi/README.md` § "Re-graduation
checklist". A split-out `chassis-experimental/` repository remains a future
direction if the workstream should leave this source tree entirely.
