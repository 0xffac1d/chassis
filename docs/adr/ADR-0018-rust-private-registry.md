---
id: ADR-0018
title: Rust private registry — Kellnr self-hosted for production, bare-git for v0.1
status: accepted
date: "2026-04-20"
enforces:
  - rule: RUST-REGISTRY-KIND-UNKNOWN
    description: "A module pin references a Rust source outside {git, registry}; the resolver surface is typed."
  - rule: RUST-REGISTRY-GIT-V01-UNMARKED
    description: "A module pins `source.git = file://…legacy-registry.git` without app.yaml metadata declaring v0.1 placeholder mode."
  - rule: RUST-REGISTRY-REGISTRY-ENTRY-MISSING
    description: "A module declares `source.registry = <name>` without a matching `[registries.<name>]` entry in `.cargo/config.toml`."
  - rule: RUST-REGISTRY-GOLD-WITHOUT-ATTESTATION
    description: "Gold app (ADR-0017) resolves a module from a registry that does not publish detached in-toto/cosign bundles alongside the `.crate` tarball."
applies_to:
  - "docs/chassis/guides/module-registry.md"
  - "schemas/app/app.schema.json"
  - "scripts/chassis/generate_app_lock.py"
tags:
  - chassis
  - registry
  - rust
  - supply-chain
  - infrastructure
---

# ADR-0018: Rust private registry

## Context

Blueprint Q1 ranked distribution mechanisms for extracted Rust
modules against five axes: tier-3 ergonomics (`cargo add` /
`[workspace.dependencies]` friendliness), in-toto attestation
compatibility, operational cost, drift detection (ADR-0015
fingerprint flow), and air-gapped viability. The top row of that
ranking was `[workspace.dependencies]` + private registry +
`release-plz`; the rejected rows were git submodules (cargo issues
#10278, #10727, #15775, #4247 — five resolver bugs documented) and
vendoring (silent forks within one release cycle; Shopify's
vendoring retrospective is the standing precedent).

The Day-0 blueprint required the *production registry choice* be
"chosen and standing" even if not yet populated. Day-90 today ships
one extracted module (`aux-provider-error`, ADR-0016) via a
private git remote pinned by full
SHA. That pattern works for a single-developer monorepo but does
not scale past "a handful of modules" (the git plumbing's clone
time grows with branch count) and cannot carry in-toto attestation
bundles alongside its artifacts the way a real cargo registry can.

The candidate set at Day-90:

- `cargo-sparse-registry` (file:// or https:// index) — the native
  Cargo protocol since 1.68; index is static files on any HTTPS host
- **Kellnr** — self-hosted, open-source, cargo-native registry
  written in Rust (per project homepage at kellnr.io)
- **Cloudsmith** — commercial hosted registry with native cargo
  support
- **Artifactory** — JFrog's enterprise artifact manager; native
  cargo support since 7.x (per module-registry guide)
- **AWS CodeArtifact** — AWS-managed; cargo support was added post
  its original launch and reports of flakiness persist through 2025
- **GitHub Packages** — native Rust support is still listed as
  limited / preview as of this writing (verify with GitHub docs
  at time of adoption; caveat stands)
- **branch-based git pins in a monorepo-of-crates** — the current
  v0.1 pattern

## Decision

1. **Production target: Kellnr (self-hosted).** Kellnr is the
   primary production registry target for extracted
   modules. It is cargo-native (speaks the sparse-registry
   protocol), open-source (no license ceiling), and per its project
   documentation ships as a single binary / container suitable for
   a single-VM deployment — appropriate for a small-team air-gapped
   or internal-network deployment. The cost profile is
   self-hosting time rather than per-seat or per-request licensing.

   Caveat: "single-VM-friendly" is per Kellnr's own project
   documentation. Operators should size the VM against their real
   artifact volume and retention policy before deployment; we do
   not claim a specific memory/disk floor here.

2. **Hosted fallback: Cloudsmith.** For teams that do not want to
   operate registry infrastructure, Cloudsmith is the accepted
   hosted fallback. It is cargo-native, ships SBOMs and detached
   signatures (compatible with ADR-0014 tier-1 attestation flows),
   and has a published pricing model. Teams adopting Cloudsmith do
   not interact with chassis infrastructure differently — the
   consumer surface (`<name> = { workspace = true }` in crate
   manifests; `[workspace.dependencies]` with `registry = "…"`) is
   identical under either registry.

3. **v0.1 placeholder retained.**
   The configured private git URL
   remains the **v0.1** mechanism and is kept as an **emergency fallback** even after
   Kellnr stands up. `app.yaml` MUST mark any module sourced from
   that URL with `metadata.registry_kind=v0.1-placeholder` so the
   composition gate can emit `RUST-REGISTRY-GIT-V01-UNMARKED` if
   the marker is missing.

4. **Target timeline.** Kellnr target stand-up: Day-60 from this
   ADR (end of Q2 2026, contingent on extraction cadence hitting
   a second module). Migration of `aux-provider-error` to Kellnr:
   together with the Day-60 cutover. No hard migration deadline —
   the v0.1 git pattern is safe to leave running until the
   Kellnr instance is health-checked end-to-end.

5. **`release-plz` integration deferred.** Manual
   `cargo publish --registry <name>` is acceptable for v0.1. A
   second ADR (out of scope here) lands `release-plz` once the
   extraction cadence justifies automation — expected Day-90 from
   this ADR.

6. **`app.yaml` source kinds.** The schema already accepts both
   `source.git = <url>, rev = <sha>` and `source.registry = <name>,
   version = <exact>`. No schema change is required — this ADR
   pins the **choice of registry implementation** behind the
   `source.registry` kind for production adoption.

### Decision matrix

Axes mirror the blueprint Q1 table: mechanism × one-time cost ×
tier-3 ergonomics (ADR-0014) × in-toto compatibility (ADR-0014
tier-1) × operational cost × drift detection (ADR-0015 fingerprint
flow).

| Mechanism | Up-front cost | Tier-3 ergonomics (`cargo add`) | In-toto bundle distribution | Operational cost | Drift detection (ADR-0015) | Verdict |
|---|---|---|---|---|---|---|
| **Kellnr (self-hosted)** | Medium (stand up VM + TLS + backup) | Native — cargo sparse protocol | Native — arbitrary sidecar files per crate version | Low–medium, one VM per project docs | Works (consumer commits `.chassis-schema.sha256` unchanged) | **Selected — production** |
| **Cloudsmith (hosted)** | Low (signup + token) | Native cargo | Native sidecar support | Subscription | Works | **Selected — hosted fallback** |
| Bare git + `file://` (v0.1) | Zero | Works via `git = …, rev = …` | Awkward — bundles must be committed into the registry tree | Zero | Works | **Retained — v0.1 placeholder and emergency fallback** |
| `cargo-sparse-registry` (roll-your-own on static HTTPS) | Low | Native | Works (sidecar files on same origin) | Low but no admin UI, no auth layer | Works | Considered — narrower than Kellnr for no functional gain |
| Artifactory | Medium–high (licensing + operator time) | Native (7.x+) | Native | High — enterprise licensing | Works | Rejected for small-team profile — overkill |
| AWS CodeArtifact | Medium (IAM + pay-per-request) | Native but reports of cargo flakiness through 2025 | Native (sidecar via S3 backing) | Per-request + egress | Works | Rejected — AWS-only lock-in + reliability concerns |
| GitHub Packages (Rust) | Low | Rust support still listed as limited at time of writing | Uneven (reported) | Zero incremental | Works | Rejected for Rust — see ADR-0019 for GitHub Packages npm use |
| crates.io (public) | Zero | Native | Native | Zero | Works | Rejected — defeats "private" requirement |
| git submodules | Zero | Broken (five cargo resolver bugs) | N/A | Zero | Works via fingerprint but submodule workflow hostile | Rejected — blueprint Q1 |
| Vendoring | Zero | Works | N/A | Zero | Silent forks (Shopify precedent) | Rejected — blueprint Q1 |
| Cross-repo `path =` | Zero | Broken (cargo #14946 still open per blueprint Q1) | N/A | Zero | No identity | Rejected — blueprint Q1 |

The top two rows are both selected: Kellnr for teams that self-host,
Cloudsmith for teams that don't. The consumer surface is identical
under either — the choice is operational, not architectural.

## Consequences

- **Chassis runtime crates now ship through a second v0.1 bare-git
  registry.** As of 2026-04-20, `chassis-runtime` and
  `chassis-runtime-api` are consumed by a downstream provider crate
  from a private chassis registry rev
  `6a5a3eeb66c04e12d9f5be5ec438af3feb7f7a9a` (a snapshot of the
  chassis working tree), pinned via the downstream workspace's
  `[workspace.dependencies]`. This is the same v0.1 placeholder
  pattern as the legacy registry URL above; the production path
  (Kellnr self-hosted, Cloudsmith hosted fallback) is unchanged by
  this addition. `chassis-runtime-napi` is intentionally **not**
  mirrored into the chassis registry yet — no downstream crate
  consumes it today, and blast radius stays small. Adding it is a
  trivial re-push when the first Node consumer lands.
- **Day-60 deliverable:** Kellnr instance standing at an internal
  address (working name `registry.internal`), TLS-terminated,
  backed up, seeded with `aux-provider-error` and any module the
  extraction cadence promotes between now and then. Migration
  procedure documented in an update to
  `docs/chassis/guides/module-registry.md`.
- **v0.1 path survives as fallback.** The bare-git pattern is not
  deprecated; it is demoted. Emergency recovery (Kellnr outage)
  falls back to repointing `source.registry` entries to
  `source.git = file://…legacy-registry.git`.
- **Rule IDs bind to enforcement.** `RUST-REGISTRY-KIND-UNKNOWN`
  and `RUST-REGISTRY-GIT-V01-UNMARKED` are added to the
  composition gate's closed-set validator. `RUST-REGISTRY-
  REGISTRY-ENTRY-MISSING` fires at `cargo fetch` time via a
  wrapper script; the raw cargo error is preserved.
- **Gold assurance ties to tier-1 attestation (ADR-0017).** Any
  registry chosen for a gold app must publish detached
  in-toto/cosign bundles. `RUST-REGISTRY-GOLD-WITHOUT-
  ATTESTATION` is the hard-fail signal; Kellnr and Cloudsmith
  both support this; the bare-git v0.1 path is **not gold-eligible**.
- **Ungoverned consumers unaffected.** Per ADR-0014, a consumer
  pulling one module with no chassis symbols reachable runs with
  `CHASSIS_GOVERNANCE=ungoverned`; they see only the surface
  `<name> = { version = "…", registry = "…" }` (or `git = …, rev =
  …`) and none of the gate machinery.
- **Schema is stable across this decision.**
  `schemas/app/app.schema.json` already accepts both source kinds;
  no schema bump is triggered by this ADR. The v0.1-placeholder
  marker is `metadata.registry_kind`, which is a free-form
  metadata field in the existing schema.
- **Release-plz deferred, not abandoned.** A follow-up ADR at
  Day-90 lands the automation layer; until then manual
  `cargo publish` is the supported workflow and is documented in
  the extraction playbook.

## Alternatives considered

- **Git submodules.** Cargo issues #10278, #10727, #15775, #4247
  — five distinct resolver bugs. Blueprint Q1 rejected; ADR-0014
  echoes the rejection. Operators hate the workflow, CI hates it
  more.
- **Vendoring.** Silent forks appear within one release cycle;
  Shopify's vendoring retrospective is the standing precedent
  cited in the blueprint.
- **crates.io (public).** Directly defeats the "private"
  requirement. Any module with embedded customer-specific
  behavior cannot publish there regardless of policy.
- **Cross-repo `path =` deps.** Cargo #14946 is still open per
  the blueprint; using path across repo boundaries breaks the
  resolver in subtle ways and erases the identity primitive
  ADR-0015 depends on.
- **AWS CodeArtifact.** AWS-only lock-in is the primary
  disqualifier; secondary concerns about cargo support
  reliability through 2025 (per blueprint Q1) push it below the
  line. Teams already on AWS can still run Kellnr on EC2 and
  get cargo-native without the CodeArtifact integration risk.
- **GitHub Packages for Rust.** Rust support is still listed as
  limited at the time of this ADR. See ADR-0019 for why GitHub
  Packages *is* our npm choice — the ecosystem posture differs
  by language.
- **Artifactory.** Functionally correct and widely deployed at
  enterprise scale. Priced and operated for orgs an order of
  magnitude larger than ours; no feature gap justifies the cost
  difference.

## References

- ADR-0014 (governance tiers): ungoverned/tier-3/tier-1 consumer
  cost model; the registry must support tier-1 attestation
  sidecars.
- ADR-0015 (schema fingerprint identity): the drift primitive
  every registry must preserve on the consumer side.
- ADR-0016 (deferred extractions): the list of modules this
  registry is expected to carry.
- ADR-0017 (pin-policy assurance mapping): gold apps require
  tier-1 attestation; the registry choice must support it.
- ADR-0019 (npm private registry): sibling decision for the
  TypeScript scope.
- ADR-0020 (TS distribution paths): forward reference; the
  cross-language consumption story.
- `docs/chassis/guides/module-registry.md`: operator walkthrough;
  updated to point at this ADR.
- Blueprint Q1 (distribution-mechanism scorecard).
- Kellnr project homepage (kellnr.io): cargo-native, self-host
  or cloud, open-source.
- Cloudsmith public pricing page (at time of adoption).

## Status

Accepted. Kellnr stand-up targeted Day-60 (2026-06-19). v0.1
bare-git path continues to carry `aux-provider-error` in the
interim; no module migration happens until the Kellnr instance
is health-checked end-to-end.
