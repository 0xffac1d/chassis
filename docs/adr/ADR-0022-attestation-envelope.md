---
id: ADR-0022
title: "Release-gate attestation envelope (in-toto + DSSE, ed25519)"
status: accepted
date: "2026-05-14"
supersedes: []
enforces:
  - rule: CH-ATTEST-SIGN-FAILED
    description: "Signing a release-gate statement failed (key I/O, serialization, or crypto)."
  - rule: CH-ATTEST-VERIFY-FAILED
    description: "DSSE signature over the in-toto Statement did not verify against the provided public key."
  - rule: CH-ATTEST-SUBJECT-MISMATCH
    description: "Statement subject digest does not match chassis-core::fingerprint::compute for the repo root."
  - rule: CH-ATTEST-PREDICATE-INVALID
    description: "Predicate JSON does not conform to the release-gate predicate schema."
  - rule: CH-ATTEST-NOT-FOUND
    description: "A cached attestation artifact (e.g. release_gate) was requested but the file is absent."
---

# ADR-0022 — Attestation envelope

## Context

Wave 5 needs a signed, verifiable release-gate artifact that binds schema fingerprint, git head, trace/drift summaries, and the commands that produced them. Fulcio/Sigstore is deferred; local ed25519 keys suffice for the first implementation.

## Decision

- **Outer envelope:** [DSSE](https://github.com/secure-systems-lab/dsse) with `payloadType: application/vnd.in-toto+json`.
- **Payload:** [in-toto Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1.0/statement.md) whose `subject` names the schema bundle via the digest from `chassis_core::fingerprint::compute` on the repository root.
- **Predicate type:** `https://chassis.dev/attestation/release-gate/v1` (JSON object conforming to `schemas/release-gate.schema.json` once present).
- **Signing:** Ed25519 via `ed25519-dalek` v2. Development keys live under `.chassis/keys/` and are gitignored; CI may use an ephemeral keypair or a checked-in public key with a secret private key.
- **Out of scope here:** Sigstore/Fulcio, OIDC, keyless signing — a future ADR may layer these on without changing the predicate type.

## Consequences

Tooling must emit DSSE + in-toto compatible with this ADR before `chassis attest verify` is considered complete. The example subprocess log under `reference/artifacts/` is **not** normative for the predicate shape.
