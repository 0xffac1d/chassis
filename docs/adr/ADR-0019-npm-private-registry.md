---
id: ADR-0019
title: npm private registry — GitHub Packages with mandatory Verdaccio CI proxy
status: accepted
date: "2026-04-20"
enforces:
  - rule: NPM-REGISTRY-SCOPE-UNCONFIGURED
    description: "A consumer repo publishes or installs from an internal scope (`@chassis/*`) without the matching `@scope:registry=…` line in `.npmrc`."
  - rule: NPM-REGISTRY-CI-WITHOUT-VERDACCIO
    description: "A CI job installs from the internal scopes without routing through the Verdaccio proxy; GitHub Packages rate-limit exposure is unmitigated."
  - rule: NPM-REGISTRY-PLATFORM-SPLIT-MISSING
    description: "The napi binding (currently `@chassis/runtime-experimental`; subject to rename on graduation) is published without the per-triple subpackages expected by the napi-rs v3 optionalDependencies loader. Vacuous while the package remains `private:true`."
  - rule: NPM-REGISTRY-FINGERPRINT-GATE-MISSING
    description: "`@chassis/types` is published without the schema-fingerprint prepublish check (ADR-0015) having run."
applies_to:
  - "docs/chassis/guides/napi-binding.md"
  - "docs/chassis/guides/module-registry.md"
  - "crates/_experimental/chassis-runtime-napi/package.json"
tags:
  - chassis
  - registry
  - npm
  - typescript
  - napi-rs
  - supply-chain
  - infrastructure
---

# ADR-0019: npm private registry

## Status note (2026-04-23 update)

The napi-rs binding referenced below is **experimental and private.**
It ships as `@chassis/runtime-experimental` with `"private": true`. The registry
decisions stand as forward-looking design; none of them are active
until the graduation checklist in
`crates/_experimental/chassis-runtime-napi/README.md` § "Re-promoting this crate"
is closed and the `-experimental` suffix is dropped. On graduation the binding publishes
under `@chassis/*`.

## Context

Blueprint Q3 (Rust↔TS bridge, 2026) selected **napi-rs v3** for the
binding (see `docs/chassis/guides/napi-binding.md`) and followed
with a decision matrix for where the resulting packages are
published. Two npm scopes were originally in play:

- `@chassis/*` — pure TypeScript packages (e.g. `@chassis/types`,
  generated from the canonical JSON Schemas). No native code. The
  graduated napi binding also lands here (see status note above).

The napi-rs v3 cross-compile matrix declared in
`crates/_experimental/chassis-runtime-napi/package.json` covers eight triples;
each produces its own `.node` (or `.wasm`) artifact that must be
hosted as a subpackage on the registry. Any npm solution therefore
has to host **native binaries**, not just ESM source. That one
constraint closes several doors (JSR in particular).

The blueprint Q3 npm decision surface:

- **GitHub Packages** — free for internal use, tied to the repo's
  GitHub org; rate limits apply and have caused CI incidents in
  practice.
- **Verdaccio** — zero-cost self-hosted cache + registry; supports
  arbitrary scopes; hosts native binaries without restriction.
- **JSR** — ESM source registry; cannot host native binaries.
  Wrong primitive for the napi binding (which ships per-triple
  native `.node` artifacts); technically viable for `@chassis/*`
  pure-TS packages but fragmenting across two registries for one
  scope family is not worth it.
- **Sonatype Nexus** — enterprise feature set, licensing and
  operator cost mismatched to a <20-dev team.
- **AWS CodeArtifact** — pay-per-use, IAM-first; viable but
  AWS-only, same lock-in concern as ADR-0018.
- **Cloudflare Packages** — does not exist as a GA product as of
  April 2026.
- **npmjs.com (public)** — defeats the "private" requirement for
  internal scopes.

The blueprint's recommended combination was explicit: **GitHub
Packages for the internal scopes + Verdaccio as a mandatory CI
proxy / cache**. This ADR accepts that recommendation.

## Decision

1. **Primary registry: GitHub Packages.** Internal scopes publish to
   GitHub Packages. Scope-to-registry mapping is published via
   `.npmrc` in each consuming repo:

   ```
   @chassis:registry=https://npm.pkg.github.com
   //npm.pkg.github.com/:_authToken=${GH_PKG_TOKEN}
   ```

   Legacy private-scope mappings belong in downstream repositories, not
   in Chassis. The binding itself is currently
   `@chassis/runtime-experimental`, `"private":true`, and not
   published; see status note above.

2. **Mandatory CI proxy: Verdaccio.** Every CI job that installs
   from either internal scope routes through a locally-run
   Verdaccio instance. Reasons: (a) GitHub Packages rate-limits
   have caused CI incidents in practice and persist as a published
   risk on GitHub's own status history; (b) Verdaccio caches
   upstream tarballs, keeping CI warm when GitHub Packages is
   degraded; (c) Verdaccio natively supports the scope mapping
   pattern so the CI `.npmrc` points at `http://localhost:4873`
   and Verdaccio transparently upstreams to GitHub Packages. The
   "run Verdaccio in CI" pattern is exactly the blueprint Q3
   recommendation.

3. **Rejected: JSR.** JSR hosts ESM source only. The napi-rs
   binding publishes native `.node` artifacts and a
   `wasm32-wasip1-threads` `.wasm`; JSR cannot carry those. JSR
   also cannot carry the platform-split optionalDependencies
   pattern. The `@chassis/*` pure-TS scope *could* live on JSR
   but splitting the two scopes across registries is not worth
   the operational tax.

4. **Platform-split publishing is non-optional (on graduation).**
   The graduated napi binding publishes one subpackage per triple
   declared in `crates/_experimental/chassis-runtime-napi/package.json`'s
   `napi.targets`. Until graduation the package is
   `@chassis/runtime-experimental`, `"private":true`, and does not
   publish at all — the platform-split requirement is vacuous in
   that state.
   The root package declares all subpackages via
   `optionalDependencies`; the loader in `index.js` (generated by
   `napi build`) picks the matching platform at install time.
   Missing platform subpackages trigger
   `NPM-REGISTRY-PLATFORM-SPLIT-MISSING`. The matrix at time of
   this ADR is the six triples listed in
   `docs/chassis/guides/napi-binding.md`:
   `aarch64-apple-darwin`, `x86_64-apple-darwin`,
   `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`,
   `aarch64-unknown-linux-gnu` (primary: NVIDIA DGX Spark),
   `aarch64-unknown-linux-musl`, `x86_64-pc-windows-msvc`, and
   `wasm32-wasip1-threads`. (Eight total; "6 triples" as sometimes
   described in prose elides the two wasm / musl-arm entries —
   count the full `napi.targets` array as the canonical source.)

5. **Schema-fingerprint gate is a mandatory prepublish step for
   `@chassis/types`.** `@chassis/types` is generated from the
   canonical JSON Schemas whose identity is the fingerprint from
   ADR-0015. The package's `prepublishOnly` script MUST verify
   `release/schemas-manifest.sha256` matches the schemas tree the
   codegen ran against. Missing the check fires
   `NPM-REGISTRY-FINGERPRINT-GATE-MISSING` at gate time.

6. **`.npmrc` scope-registry lines are load-bearing.** A repo
   that publishes or installs from internal scopes without the
   matching `@scope:registry=…` line fires
   `NPM-REGISTRY-SCOPE-UNCONFIGURED`. The check is a regex against
   `.npmrc` at CI start; it catches the recurring "works on dev
   laptop, silently resolves from public npmjs.com in CI" drift
   case.

### Decision matrix

Mirrors the blueprint Q3 npm table: registry × cost × native-binary
support × notes × verdict.

| Registry | Cost | Native / platform-split binaries | Scope support | Notes | Verdict |
|---|---|---|---|---|---|
| **GitHub Packages** (primary) | Free for org-internal | Yes — arbitrary tarballs; optionalDependencies pattern supported | Native scope mapping | Rate limits on install are the known soft spot; Verdaccio proxy mitigates | **Selected — primary** |
| **Verdaccio** (CI proxy + cache) | Zero self-host | Yes | Native scope mapping | Single binary or Docker image; runs local-to-CI | **Selected — mandatory proxy** |
| JSR | Free | **No — ESM source only** | Native | Wrong primitive for napi-rs binaries | Rejected |
| Sonatype Nexus | Enterprise licensing | Yes | Native | Operator cost far exceeds <20-dev scale | Rejected — overkill |
| AWS CodeArtifact | Pay-per-use (requests + egress) | Yes | Native | AWS-only lock-in; IAM-first auth adds friction to external contributors | Rejected — lock-in |
| Cloudflare Packages | — | — | — | Does not exist as a GA product as of 2026-04 | Not applicable |
| npmjs.com (public) | Free | Yes | Native | Public — defeats "private" requirement for internal scopes | Rejected |

## Consequences

- **CI must run Verdaccio.** Every CI workflow that installs from
  the internal scopes starts a Verdaccio container (or runs the
  binary) and points `.npmrc` at it. The `docs/chassis/guides/
  napi-binding.md` guide is updated with the pointer to this
  ADR; the concrete CI recipe lives there.
- **Publishing `.npmrc` in every consumer repo.** Either checked
  in (scope lines only, no tokens) or templated from a shared
  config. The scope-registry mapping is not optional.
- **`@chassis/types` prepublish is now gated.** The package's
  `prepublishOnly` runs the fingerprint check before any tarball
  leaves a developer machine or CI runner. This is the hard
  binding from ADR-0015 into the npm publishing path.
- **napi-rs platform-split shape is stable and documented.** The
  publishing matrix is whatever `napi.targets` says in
  `crates/_experimental/chassis-runtime-napi/package.json`; adding or removing
  a triple is a one-line edit plus the CI matrix update, nothing
  more. Verdaccio caches each subpackage independently, so a
  single dropped tarball doesn't invalidate the full set.
- **Forward compatibility.** If GitHub Packages' Rust support
  matures to usable parity (see ADR-0018 — currently rejected
  for Rust), Kellnr remains the Rust choice; this ADR does not
  cross-bind npm and cargo registry choices. They are independent.
- **JSR migration is possible for `@chassis/*` only.** If at some
  future checkpoint JSR is the better home for pure-TS packages,
  we can split the scope family — at the operational cost of two
  registries. No current reason to; flagged as a lever, not a plan.
- **Rate-limit incidents become non-fatal.** Verdaccio's cache
  carries the last known-good tarballs. GitHub Packages going
  degraded during a release window no longer blocks the release;
  it only blocks upstream *pulls* of new versions until recovery.

## Alternatives considered

- **JSR.** ESM-only. Cannot host native binaries. The blueprint
  Q3 table rejects it as the wrong primitive for the napi-rs
  binding. Not a viable primary; not worth fragmenting the scope
  family over.
- **Cloudflare Packages.** Advertised but not GA at time of
  writing; not a selectable option.
- **neon on public npmjs.com.** Blueprint Q3 rejection: no
  automated `.d.ts` generation (drifts from Rust), no
  platform-split publishing story. napi-rs v3's
  platform-subpackage pattern is exactly what we need;
  neon-on-npmjs.com forfeits both.
- **Postinstall binary downloads (node-pre-gyp / node-gyp
  lifecycle).** Prisma's own retrospective on their
  Rust-to-TypeScript migration cites the postinstall-download
  model as a primary pain driver — failures behind corporate
  proxies, stale caches, CI flake. The napi-rs v3
  platform-subpackage model is explicitly designed to avoid this
  failure class; we do not reintroduce it.
- **Sonatype Nexus.** Feature-complete and widely deployed at
  enterprise scale; priced and operated for orgs an order of
  magnitude larger than ours.
- **AWS CodeArtifact (npm side).** Same AWS lock-in concern as
  ADR-0018's rejection on the Rust side; additionally, IAM-first
  auth adds friction for any external contributor path.

## References

- ADR-0014 (governance tiers): the consumer-cost ladder the
  registry choice feeds.
- ADR-0015 (schema fingerprint identity): the prepublish gate on
  `@chassis/types` is this primitive in the npm pipeline.
- ADR-0017 (pin-policy assurance mapping): gold apps require
  tier-1 attestation on both Rust and npm sides.
- ADR-0018 (Rust private registry): sibling decision for the
  cargo scope; independent choice.
- ADR-0020 (TS distribution paths): forward reference; the
  per-consumer shape (dashboard, CLI, dev tools) that this
  registry decision carries.
- Blueprint Q3 (npm registry decision matrix).
- `docs/chassis/guides/napi-binding.md`: platform-split matrix and
  publishing recipe.
- `docs/chassis/guides/module-registry.md`: Rust-side companion.

## Status

Accepted. GitHub Packages stand-up is same-day (org permissions
already in place). Verdaccio CI proxy lands in the next CI pipeline
update. `@chassis/types` prepublish gate wiring and platform-split
publish matrix land alongside the first napi-rs v3 release beyond
the Day-60 scaffold.
