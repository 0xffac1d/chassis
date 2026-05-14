---
id: ADR-0013
title: Runtime library split — chassis-runtime-api (stable) vs chassis-runtime (impl)
status: accepted
date: "2026-04-20"
enforces:
  - rule: RUNTIME-API-BREAKING-CHANGE
    description: "A change to chassis-runtime-api breaks source compatibility within a 0.x.y line; the stable-surface contract forbids it."
  - rule: RUNTIME-IMPL-LEAKED
    description: "chassis-runtime-api publicly re-exports a type, trait, or function owned by chassis-runtime; the API/impl boundary has leaked."
  - rule: RUNTIME-DEFAULT-FEATURE-BLOAT
    description: "chassis-runtime default features pull a dep outside the baseline set; the <50 KB / WASM-clean default budget is violated."
  - rule: RUNTIME-SURFACE-UNDOCUMENTED
    description: "A function added to chassis-runtime-api lacks a doc comment identifying it as part of the five-function stable surface."
  - rule: INVARIANT-DENSITY-TEXT-EXCESS
    description: "A single invariant text exceeds the per-item character budget (250 chars)."
  - rule: INVARIANT-DENSITY-EDGE-CASE-EXCESS
    description: "A single edge_case text exceeds the per-item character budget (400 chars)."
  - rule: INVARIANT-DENSITY-COUNT-EXCESS
    description: "A CONTRACT.yaml file contains more than 15 invariants or edge_cases combined."
  - rule: INVARIANT-DENSITY-TOTAL-CHARS-EXCESS
    description: "Total invariant and edge_case text in a single CONTRACT.yaml file exceeds the 4000-char budget."
  - rule: INVARIANT-DENSITY-SUMMARY
    description: "Summary finding emitted by the invariant-density gate after scanning all CONTRACT.yaml files."
applies_to:
  - "crates/chassis-runtime-api/**"
  - "crates/chassis-runtime/**"
  - "scripts/chassis/gates/runtime_surface.py"
tags:
  - chassis
  - governance
  - runtime
  - versioning
---

# ADR-0013: Runtime library split

## Context

Blueprint blind-spot #1: *"the chassis-runtime version-skew problem is
worse than the schema-drift problem."* When an app composes modules
from different release trains, each module depends transitively on
some version of the chassis runtime crate. Cargo's resolver will pick
one — and if the runtime carries heavy deps or churns its surface,
minor runtime bumps cascade into compile failures for every consumer.
Smithy-rs solved the equivalent problem for AWS SDKs by splitting the
runtime into a stable API crate and a churning implementation crate;
the API crate pins a narrow, versioned surface, and modules depend
only on that.

A monolithic `chassis-runtime` with cargo features is not a
substitute. Re-exports leak implementation; feature unification across
a dep graph still pulls the heaviest configuration; and WASM targets
fail on transitive deps that never enter the stable surface at all.

Runtime-side DSL evaluation (OPA-shaped) is rejected for the same
class of reason: DSL surfaces expand uncontrollably, partial-eval
cliffs break determinism, bundle staleness creates its own drift
problem, and governance-at-runtime is the opposite of the chassis
model (govern at build, attest at release, verify at deploy).

## Decision

1. **Two crates.** `chassis-runtime-api` (stable traits, types, and
   five functions) and `chassis-runtime` (implementation behind the
   API, may churn, heavy deps behind features).
2. **`chassis-runtime-api` is the pinned surface.** Every 0.x.y
   release is source-compatible. Consumers pin to a major (0.x).
   Additive changes are y-bumps; non-additive changes are x-bumps.
   The surface is **five functions**:
   - `emit(diagnostic)`
   - `attest(claim)`
   - `Validator::validate(&self, input) -> Result<...>`
   - `check_exemption(rule_id, ctx) -> ExemptionState`
   - `coherence_report() -> CoherenceReport`
3. **`chassis-runtime` implements the API.** No public re-exports
   from `chassis-runtime-api`. Heavy deps gated behind opt-in
   features: `validation` (jsonschema), `exemptions` (sqlite),
   `introspect` (tracing-subscriber).
4. **Defaults are lean.** `diagnostics` + `assurance` only. <50 KB
   alloc-only. WASM-clean (`wasm32-unknown-unknown` builds with the
   default feature set).
5. **No runtime DSL, no policy hot-reload, no runtime gate
   evaluation in v1.** Gates run at build time via the chassis CLI.
   The runtime's job is to *emit*, not to *decide*.
6. **Cedar is a shape, not a dependency.** Cedar's policy model is
   the right vocabulary for capability reasoning, but it enters as a
   design reference; we do not take a Cedar dep.

## Consequences

- Modules across release trains can compose as long as their
  `chassis-runtime-api` major matches; implementation-side churn in
  `chassis-runtime` does not break consumers.
- WASM and embedded targets are viable: default features compile
  without sqlite, jsonschema, or tracing-subscriber.
- The stable surface is small enough to audit, review, and pin
  contracts against.
- A new gate (`runtime_surface`) enforces the boundary: no leaks
  from impl to API, no undocumented additions, no default-feature
  bloat.
- Feature flags do real work: `validation` gates compile-in of
  jsonschema, not a runtime toggle.

## Alternatives considered

- **Single crate with features.** Features unify across the graph,
  heavy configs win, re-exports leak surface.
- **OPA-shaped DSL engine.** Partial-eval cliffs, bundle-staleness
  drift, runtime governance is the wrong axis.
- **Cedar as a dependency.** Overkill; the model is the value, not
  the crate.
- **Proc-macro-generated API crate.** Obscures the stable surface;
  makes `cargo doc --no-deps` a poor audit tool.

## Hard non-goals (v1)

- No embedded policy DSL.
- No runtime gate evaluation.
- No policy hot-reload.
- No dynamic claim issuance at request time.

## References

- ADR-0012 (app composition): the apps that pin this API.
- Smithy-rs `aws-smithy-runtime-api` / `aws-smithy-runtime`
  precedent.
- Blueprint §"runtime version-skew" (blind-spot #1).

## Status

Accepted. Crate split lands alongside this ADR.
