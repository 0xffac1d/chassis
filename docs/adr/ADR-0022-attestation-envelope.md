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
  - rule: CH-ATTEST-ENVELOPE-SCHEMA
    description: "DSSE envelope does not conform to schemas/dsse-envelope.schema.json (sign or verify side)."
  - rule: CH-GATE-REPO-UNREADABLE
    description: "Release-gate input root is missing or not a directory; no trace graph can be built."
  - rule: CH-GATE-GIT-METADATA-REQUIRED
    description: "`release-gate` was run on a tree without Git checkout metadata (e.g. an extracted source archive): drift and `git_commit` require `.git` + `HEAD`."
  - rule: CH-GATE-CONTRACT-INVALID
    description: "Release-gate preflight rejected at least one CONTRACT.yaml that did not satisfy schemas/contract.schema.json. Surfaced identically by the CLI envelope and the JSON-RPC `release_gate` error so both surfaces fail closed before trace/drift/exempt run. Not exemption-applicable."
  - rule: CH-GATE-CONTRACT-MALFORMED
    description: "A CONTRACT.yaml file could not be read or YAML-parsed during release-gate preflight (filesystem error or malformed YAML before schema validation)."
  - rule: CH-GATE-SUBSYSTEM-FAILURE
    description: "Trace, drift, or git inspection failed below the release gate."
  - rule: CH-GATE-REGISTRY-MALFORMED
    description: ".chassis/exemptions.yaml existed but failed to parse or did not match the registry shape."
  - rule: CH-GATE-SCHEMA-INVALID
    description: "Release-gate predicate, trace graph, or drift report did not satisfy its canonical JSON Schema."
  - rule: CH-ATTEST-KEY-MISSING
    description: "Caller asked for --attest / attest sign without an explicit --private-key, .chassis/keys/release.priv, or an explicit --ephemeral-key opt-in. The signer fails closed rather than fabricating a throwaway key."
---

# ADR-0022 — Attestation envelope

## Context

Wave 5 needs a signed, verifiable release-gate artifact that binds schema fingerprint, git head, trace/drift summaries, and the commands that produced them. Fulcio/Sigstore is deferred; local ed25519 keys suffice for the first implementation.

## Decision

- **Outer envelope:** [DSSE](https://github.com/secure-systems-lab/dsse) with `payloadType: application/vnd.in-toto+json`.
- **Payload:** [in-toto Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1.0/statement.md) whose `subject` names the schema bundle via the digest from `chassis_core::fingerprint::compute` on the repository root.
- **Predicate type:** `https://chassis.dev/attestation/release-gate/v1` (JSON object conforming to `schemas/release-gate.schema.json` once present).
- **Signing:** Ed25519 via `ed25519-dalek` v2. Development keys live under `.chassis/keys/` and are gitignored.
- **Key policy (fail closed).** `chassis attest sign` and `chassis release-gate --attest` refuse to sign with an implicitly fabricated keypair:
  1. If `--private-key <path>` is given, use it. The attestation is **release-grade**.
  2. Otherwise, require `.chassis/keys/release.priv`. If present, use it; release-grade.
  3. Otherwise, the caller may pass `--ephemeral-key` to opt into a freshly generated keypair. The CLI writes the matching public key as `<envelope>.ephemeral.pub`, stamps the DSSE signature with `keyid: ephemeral:<hex-pub>`, surfaces `release_grade: false` + the public-key path + fingerprint in the JSON summary, and prints a `WARNING` on stderr. **Such envelopes are not release-grade** and must not be promoted to a release verifier.
  4. With none of the above, the command exits non-zero with rule `CH-ATTEST-KEY-MISSING`.
  `--ephemeral-key` and `--private-key` are mutually exclusive.
- **Out of scope here:** Sigstore/Fulcio, OIDC, keyless signing — a future ADR may layer these on without changing the predicate type.

## Consequences

Tooling must emit DSSE + in-toto compatible with this ADR before `chassis attest verify` is considered complete. The example subprocess log under `reference/artifacts/` is **not** normative for the predicate shape. Anything that looks like a release attestation is, at minimum, signed by a key the operator named — `release_grade: true` in the CLI summary, no `ephemeral:` prefix on the DSSE `keyid`, and a verifiable public key available offline.

This ADR binds the root-contract invariant `chassis.attest-key-policy-fail-closed`: a verifier reading the root `CONTRACT.yaml` can trace that invariant back here for its rationale and authoritative rule list.
