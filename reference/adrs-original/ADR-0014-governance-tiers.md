---
id: ADR-0014
title: Governance tiers — ungoverned, tier-3, tier-1 selected by capability
status: accepted
date: "2026-04-20"
enforces:
  - rule: GOVERNANCE-TIER-UNKNOWN
    description: "CHASSIS_GOVERNANCE is set to a value outside {ungoverned, tier-3, tier-1}; the selector is typed."
  - rule: GOVERNANCE-TIER1-ATTESTATION-MISSING
    description: "CHASSIS_GOVERNANCE=tier-1 but no in-toto Statement or sigstore bundle is present at the consumer-configured path."
  - rule: GOVERNANCE-TIER3-FINGERPRINT-MISSING
    description: "CHASSIS_GOVERNANCE=tier-3 but .chassis-schema.sha256 is absent from the consumer repo."
  - rule: GOVERNANCE-TIER3-FINGERPRINT-DRIFT
    description: "The committed schema fingerprint does not match the chassis currently being consumed; drift gate fires."
  - rule: CHASSIS-QUARTERLY-REVIEW-OVERDUE
    description: "A scheduled re-review entry in config/chassis/quarterly-review.yaml is past next_review + grace_period_days."
  - rule: CHASSIS-QUARTERLY-REVIEW-GRACE
    description: "A scheduled re-review entry is past next_review but still within the grace window; emitted as warning."
  - rule: CHASSIS-QUARTERLY-REVIEW-INVALID
    description: "config/chassis/quarterly-review.yaml is missing, malformed, or contains an unparseable entry."
applies_to:
  - "scripts/chassis-extraction/verify-schema-fingerprint.sh"
  - "docs/chassis/guides/governance-tiers.md"
  - "schemas/chassis/in-toto-statement.schema.json"
tags:
  - chassis
  - governance
  - tiers
  - supply-chain
---

# ADR-0014: Governance tiers

## Context

Blueprint blind-spot #4: *"ungoverned consumption"* has been
conflated with *"Tier-3"*. They are different failure modes.

- A consumer pulling one chassis-governed module as a plain cargo
  dependency, with no chassis symbols reachable from their binary,
  is **ungoverned in use**. Imposing CI friction on them is
  user-hostile and drives them to fork.
- A fork that wants to ignore chassis gates in production is
  **ungoverned in production**. Imposing CI friction here is the
  whole point.

The distinction is *capability*, not *presence*. Governance must key
off what the consumer does with chassis, not whether chassis files
are on disk. A single fork flag collapses both cases; a tier ladder
handles them cleanly.

The symlink drift incident (see ADR-0015) also showed that the
weakest tier must do *something* — a zero-cost drift check is
dramatically better than nothing, because the failure mode
(unnoticed schema drift across a sibling symlink) is silent.

## Decision

1. **Three tiers, selected by `CHASSIS_GOVERNANCE` env var.** Not by
   fork, not by feature flag, not by presence of a config file.
2. **`ungoverned`** — library import, no chassis symbols reachable,
   zero CI friction. The drift gate is a no-op. Appropriate for
   personal repos pulling one module as a cargo dep.
3. **`tier-3`** — one-line SHA-256 check against committed
   `.chassis-schema.sha256`. Implemented as
   `scripts/chassis-extraction/verify-schema-fingerprint.sh` in the
   consumer repo. ~10 ms, no network, no cosign. Appropriate for
   small teams composing one or two chassis modules.
4. **`tier-1`** — full gate suite. In-toto v1 signed Statement
   (`https://in-toto.io/Statement/v1`) covering the Rust `.crate`
   tarball, npm tarball, canonicalized schema SHA, and git commit.
   Consumer CI runs
   `cosign verify-blob-attestation --bundle chassis.sigstore.json`
   at ~10–30 s cost. SLSA v1 provenance attached. Appropriate for
   enterprise consumers, air-gapped deployments, and anything
   claiming chain-of-custody.
5. **Capability, not presence.** The env var selects the tier; the
   gate implementation keys off it. Forks that set
   `CHASSIS_GOVERNANCE=ungoverned` opt themselves out visibly — the
   choice is on the record, not hidden in a deleted directory.
6. **Default is tier-3.** The consumer gate script defaults to
   `tier-3` when `CHASSIS_GOVERNANCE` is unset. Opting *up* to
   tier-1 requires configuration; opting *down* to ungoverned
   requires an explicit env var.

## Consequences

- Consumers pick the cost/assurance tier appropriate to their use
  case; we stop trying to force one shape on everyone.
- Forks and external consumers are distinguishable from legitimate
  library users; the CI gate is keyed on the distinguishing signal.
- Tier-1 supply-chain assurance is available without requiring it
  of every consumer — SLSA v1 + in-toto + sigstore is expensive, so
  it's opt-in.
- New gates and rule IDs bind to this ADR for diagnostic
  resolution.
- The legacy product integration landed tier-3 today; the Rust crate binding lands next.

## Alternatives considered

- **Symlink the sibling chassis repo.** The original incident. No
  identity, no drift detection, silent breakage. ADR-0015 explains
  why this class of fix is structurally inadequate.
- **Git submodule.** Cargo issues #10278, #10727, #15775, #4247
  document five distinct resolver bugs with submodules-as-deps.
  Operators hate the workflow.
- **Vendoring.** Silent forks; Shopify's vendoring retrospective
  rejected this pattern for exactly the drift case we're solving.
- **SBOM as enforcement.** cargo-auditable's own docs explicitly
  refute this: an SBOM records what was built, it does not attest
  to what *should* have been built.
- **Single "chassis-governed" boolean.** Collapses the ungoverned
  vs tier-3 distinction and drives legitimate library consumers to
  fork.

## References

- ADR-0015 (schema fingerprint identity): the primitive tier-3 and
  tier-1 both depend on.
- ADR-0007 (repo boundary): what standalone vs consumer scope
  means.
- `docs/chassis/guides/governance-tiers.md`: operator walkthrough.
- Downstream tier-3 gate documentation:
  tier-3 adoption notes.

## Status

Accepted. Tier-3 gate landed in the legacy product integration on 2026-04-20. Tier-1 sigstore
wiring in progress.
