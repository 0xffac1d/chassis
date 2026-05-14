---
id: ADR-0015
title: Schema fingerprint as identity — canonicalized SHA-256 over the schemas tree
status: accepted
date: "2026-04-20"
enforces:
  - rule: SCHEMA-FINGERPRINT-MISSING
    description: "A chassis release does not ship schemas-manifest.json + schemas-manifest.sha256; consumers have nothing to pin against."
  - rule: SCHEMA-FINGERPRINT-NONCANONICAL
    description: "schemas-manifest.json is not RFC 8785 JCS canonicalized; the hash is not reproducible across platforms."
  - rule: SCHEMA-COMPOSITION-RULE-VIOLATION
    description: "A resolver picked a chassis version where Rust semver said minor but the schema fingerprint changed; schema must win."
applies_to:
  - "scripts/chassis/scripts/fingerprint_schemas.py"
  - "release/schemas-manifest.json"
  - "release/schemas-manifest.sha256"
tags:
  - chassis
  - governance
  - schema
  - identity
  - supply-chain
---

# ADR-0015: Schema fingerprint as identity

## Context

The symlink drift incident: a consumer pulled chassis via a sibling
symlink. Schema files changed upstream. Nothing in the consumer
noticed — not cargo, not CI, not the test suite. The schemas were
consumed as paths, and paths have no version opinion. Cargo's path
resolver by design has no concept of *what* is at the path — only
that something is. No identity, no comparison, no drift signal.

Every structural fix for this class of failure has the same shape:
introduce an **identity** for the depended-on artifact and diff
against it. Symlinks, submodules, vendored trees, and path deps all
fail for the same reason: no identity.

The fix is cheap. A canonicalized hash of the schemas tree is ~10 ms
to compute, ~64 bytes to commit, and gives the consumer a one-line
diff whenever the chassis they consume changes.

## Decision

1. **Every chassis release emits a canonicalized manifest.**
   `release/schemas-manifest.json` is a JSON document listing every
   file under `schemas/`, in sorted order, with per-file SHA-256.
   The document itself is RFC 8785 JCS canonicalized so the hash is
   reproducible across platforms, locales, and Python versions.
2. **The manifest SHA-256 is the identity.**
   `release/schemas-manifest.sha256` is the single-line fingerprint.
   This is what consumers pin against.
3. **Consumers commit the fingerprint.** Conventional location:
   `.chassis-schema.sha256` at the consumer repo root. One line,
   one hash, one file.
4. **Consumer CI diffs at 10 ms cost.** The consumer runs the
   chassis fingerprint CLI against the chassis they're resolving
   and compares against the committed fingerprint. Mismatch is a
   drift signal.
5. **Reference generator:**
   `scripts/chassis/scripts/fingerprint_schemas.py` (already
   landed). Invoked without args, prints the hash; invoked with
   `--write <path>`, writes the consumer-style fingerprint file.
6. **Composition rule: schema wins.** When a resolver would pick a
   chassis version where Rust semver says *minor* (no breaking Rust
   change) but the schema fingerprint differs, the schema is
   treated as breaking. The app gate refuses to compose across a
   schema fingerprint boundary without an explicit app-manifest
   bump. Pseudocode:

   ```text
   if rust_semver(prev, next) == "minor" and
      schema_fingerprint(prev) != schema_fingerprint(next):
       treat_as_breaking()
   ```

## Consequences

- The drift failure mode that motivated this ADR becomes
  impossible to miss: either the fingerprint matches or the gate
  fails.
- Symlinks, submodules, and path deps all become viable again — the
  identity lives in the committed fingerprint, not in the transport
  mechanism.
- Chassis upgrades surface as a one-line diff in every consumer
  repo (`.chassis-schema.sha256`), reviewable in PRs.
- The composition rule eliminates the "Rust minor, schema breaking"
  footgun that every monorepo-to-polyrepo migration hits.
- The fingerprint is the primitive that ADR-0014 tiers and ADR-0012
  app composition both build on.

## Alternatives considered

- **Per-file SHAs published separately.** Consumers would have to
  collect and compare file-by-file; the single-line summary is
  dramatically easier to commit and diff.
- **Git tree hash of `schemas/`.** Captures file content but
  depends on git's hashing rules (mode bits, etc.) which are not
  portable to non-git distribution (tarballs, OCI images).
- **Rust-semver-only versioning.** Doesn't express schema breakage
  when the Rust surface is unchanged — the exact failure the
  composition rule prevents.
- **Signed release artifact without a fingerprint file.** Signatures
  without an identity still require consumers to download the
  artifact to compare; the fingerprint is the cheap identity.

## References

- ADR-0014 (governance tiers): tier-3 and tier-1 both consume this
  fingerprint.
- ADR-0012 (app composition): app.lock records the schema
  fingerprint per resolution.
- RFC 8785 (JSON Canonicalization Scheme).
- `scripts/chassis/scripts/fingerprint_schemas.py`.

## Status

Accepted. Fingerprint CLI shipped; consumer adoption in progress
(legacy tier-3 integration landed 2026-04-20).
