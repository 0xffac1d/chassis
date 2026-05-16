---
id: ADR-0031
title: "GitHub build attestations for source archives"
status: accepted
date: "2026-05-15"
enforces: []
tags:
  - supply-chain
  - ci
---

# ADR-0031 — Provenance layers around source archives

## Decision

- **`source-archive.yml`** runs pinned **`actions/attest-build-provenance`** over `dist/chassis-source-ci.tar.gz`. The former external SLSA reusable workflow is intentionally not required because GitHub rejects its nested mutable action refs under SHA-pin policy.
- **`release-evidence.yml`** recomputes the schema-manifest digest from the checkout and compares it to `self-attest-artifacts/in-toto-statement.json` (see `.github/scripts/verify-in-toto-subject-digest.sh`).
- Additional artifact digests MAY be checked the same way as the evidence bundle grows; this ADR records the CI split only.

## Consequences

- Consumers can fetch GitHub-hosted provenance + a single `release-evidence` tarball per commit.
- **Trace ids:** `chassis.provenance-cosign-slsa` (Cosign + GitHub attestations around release artifacts) and `chassis.evidence-digest-roundtrip` (post-download digest checks in `release-evidence.yml`).
