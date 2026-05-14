---
id: ADR-0005
title: Ring model — dependency direction enforcement (foundation=0)
status: accepted
date: "2026-04-19"
enforces:
  - rule: RING-DEPENDENCY-VIOLATION
    description: "A module in ring N depends_on a module in ring M where M > N (lower rings cannot depend on higher rings)."
  - rule: RING-MISSING
    description: "Manifest declares a depends_on edge to a ringed module but omits its own ring; either both must be ringed or neither."
  - rule: RING-OUT-OF-RANGE
    description: "ring value is outside the schema range (0–10)."
applies_to:
  - "**/CONTRACT.yaml"
  - "**/chassis.unit.yaml"
tags:
  - chassis
  - governance
  - architecture
---

# ADR-0005: Ring model for dependency direction

## Context

`CONTRACT.yaml` carries an optional `ring: <integer 0..10>` field. The
field exists to support a build-order discipline where "foundation"
modules (ring 0) cannot depend on "application" modules (higher rings).
The semantics were never written down, and the field was used
inconsistently across early adopters.

## Decision

1. **Lower rings are more foundational.** A module in ring 0 is a
   foundation primitive (e.g. logging, error types, schema primitives).
   A module in ring 5 is an application-level component that may
   compose ring 0–4 modules.
2. **Dependency direction is one-way.** A ring-N module's `depends_on`
   array MUST NOT contain any ring-M module where M > N. The
   `RING-DEPENDENCY-VIOLATION` rule catches violations.
3. **Ring is opt-in.** Manifests without a `ring` field are simply not
   ring-checked. A ringed module may depend on an unringed module
   (treated as "out of band"); an unringed module may depend on
   anything.
4. **Boundary discipline.** A repo that adopts rings should ring its
   foundational modules first and grow upward; mixing ringed and
   unringed at the same conceptual layer defeats the discipline.
5. **Range.** Ring 0–10 (inclusive). 11+ is out of range and rejected
   by the schema. If a project genuinely needs more than 11 rings, the
   architecture probably wants a different abstraction.

## Consequences

- Foundational layers stay foundational. A logging primitive cannot
  silently accumulate dependencies on UI components.
- Adoption is incremental. A team can introduce rings on the bottom
  layer first; no big-bang ringing of the entire repo is required.
- Cross-cutting concerns (telemetry, feature flags) may need to be
  unringed deliberately if they otherwise would force unwanted
  dependency edges.

## Status

Accepted. The schema field has shipped since v0.1; this ADR
formalizes the previously-informal semantics and registers the rule
IDs for the planned `chassis ring-check` validator.
