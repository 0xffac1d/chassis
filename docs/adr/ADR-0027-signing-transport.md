---
id: ADR-0027
title: "Wave-3-close: signing transport for release-gate attestations"
status: proposed
date: "2026-05-15"
supersedes: []
enforces: []
---

# ADR-0027 — Signing transport for release-gate attestations

## Context

ADR-0022 picked **DSSE wrapping an in-toto Statement v1, signed with Ed25519**
for the first release-gate attestation. It deliberately deferred Sigstore /
cosign / SLSA, with the open question: do we layer one of them on before
calling the kernel "done", or close Wave 3 on the existing transport?

The audit closing Wave 3 found the existing transport **adequate for the kernel
release**. Concretely:

- `chassis attest sign` and `chassis release-gate --attest` produce DSSE
  envelopes that match `schemas/dsse-envelope.schema.json` and an in-toto
  Statement v1 that matches `schemas/in-toto-statement-v1.schema.json`.
- The predicate type
  (`https://chassis.dev/attestation/release-gate/v1`) and predicate schema
  (`schemas/release-gate.schema.json`) are versioned, so a future Sigstore /
  cosign / SLSA layer can wrap (or be wrapped by) the same payload without
  changing predicate type, predicate schema, or the verification rule
  vocabulary (`CH-ATTEST-*`).
- ADR-0022's fail-closed key policy already prevents implicit throwaway
  signing; ephemeral signing is opt-in and surfaces `release_grade=false`.

The remaining work — issuer pinning, OIDC keyless workflows, in-toto-attest
upload, and SLSA provenance generation in CI — is a meaningful **separate
fleet**, not a kernel hygiene item. Trying to absorb it into Wave 3 close
would either ship a half-built keyless path or hold the kernel release
indefinitely.

## Decision

1. **Stay on DSSE + in-toto Statement v1 + Ed25519 for Wave 3 close.** No
   change to the predicate type, predicate schema, DSSE envelope schema, or
   `CH-ATTEST-*` rule set. Operators retain the keyed-trust model documented
   in ADR-0022.
2. **Future provenance fleet is explicitly out of scope here.** Sigstore /
   cosign / OIDC keyless signing, GitHub artifact attestations, and SLSA
   provenance generators belong to a follow-up wave with its own ADR. That
   ADR may add new wrapping (e.g. cosign signature over the same DSSE), but
   must not change the inner predicate shape; verifiers reading only the
   Chassis predicate must keep reaching the same verdict.
3. **Documentation is normative for the current state.** The CLI long help,
   `README.md` golden path, and `docs/WAVE-PLAN.md` describe DSSE+Ed25519 as
   the supported signing transport. Anything else (cosign, SLSA, GitHub
   artifact attestations) is labeled "follow-up / out of scope" until a
   later ADR supersedes this decision.

## Consequences

- The kernel release ships with the operator-keyed Ed25519 path. Release
  managers must continue to keep their `release.priv` material out of the
  repo (`.chassis/keys/*.priv` is gitignored) and rotate keys per their
  organization's policy.
- A future Sigstore / SLSA ADR will be additive: it can introduce new rule
  IDs (e.g. `CH-PROVENANCE-*`) and a wrapping envelope, but the existing
  `CH-ATTEST-*` set, predicate type, and predicate schema remain stable.
- Until the follow-up ships, downstream verifiers cannot rely on Sigstore
  identity claims or SLSA build provenance for Chassis releases. That
  trade-off is documented and conscious.
- **Trace id:** `chassis.no-private-keys-tracked` documents the fail-closed CI guard
  against committing `.priv` material while keeping `.pub` verifier keys addressable.

## Status

Proposed at Wave-3-close. Will move to `accepted` once the README and
WAVE-PLAN reference the decision and `cargo test --workspace` passes. Will
later be superseded (not retired) by the follow-up provenance ADR.
