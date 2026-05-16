---
id: ADR-0032
title: "Pin GitHub Actions to immutable commit SHAs"
status: accepted
date: "2026-05-15"
enforces: []
applies_to:
  - .github/workflows/
  - .github/scripts/check-action-pins.sh
tags:
  - supply-chain
  - ci
---

# ADR-0032 — Immutable Action pins

## Decision

- Every non-GitHub-owned `uses:` reference in `.github/workflows/*.yml` MUST resolve to a **40-character commit SHA**, optionally followed by `# comment` naming the semver tag.
- **Exceptions** are listed in `.github/action-pin-allowlist.txt`; the list is currently empty because even the SLSA reusable workflow is pinned to the commit behind `v2.1.0`.
- `github/codeql-action/*` pins continue to use upstream-published full SHAs.
- `bash .github/scripts/check-action-pins.sh` enforces this invariant locally and in `foundation.yml` / `verify-foundation.sh`.
- Renovate remains configured with `pinDigests: true` so digest bumps arrive as normal dependency PRs.

## Consequences

- CI supply chain no longer follows floating `@v4` / `@master` pointers for critical build actions (`actions/checkout`, `dtolnay/rust-toolchain`, caches, OPA, Cosign, etc.).
- Drift is caught before merge; allowlist changes are explicit and reviewable.
