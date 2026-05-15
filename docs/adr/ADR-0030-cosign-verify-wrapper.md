---
id: ADR-0030
title: "Cosign keyless verification for DSSE blobs (optional CLI path)"
status: accepted
date: "2026-05-15"
enforces:
  - rule: CH-PROVENANCE-COSIGN-VERIFY-FAILED
    description: "Cosign `verify-blob` rejected the signature/certificate/OIDC binding when optional --cosign-* flags are passed to `chassis attest verify`."
tags:
  - attestation
  - provenance
---

# ADR-0030 — Cosign wrapper on `attest verify`

## Decision

- DSSE integrity remains **Ed25519** per ADR-0022.
- When `--cosign-signature` and `--cosign-certificate` are passed, `chassis attest verify` shells out to `cosign verify-blob` after successful Ed25519 verification, using the caller’s identity/OIDC issuer regexes.
- Failure uses `CH-PROVENANCE-COSIGN-VERIFY-FAILED` (exit code 6, same family as attest verify).

## Consequences

- CI may record Sigstore metadata alongside the existing kernel attestation without changing the predicate type.
- **Trace id:** `chassis.provenance-cosign-slsa` names the optional Cosign keyless wrapper layered on DSSE blobs.
