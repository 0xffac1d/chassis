---
id: ADR-0011
title: Diagnostic ruleId stability discipline — bind every ruleId to an ADR; deprecate *-GENERIC
status: accepted
date: "2026-04-19"
enforces:
  - rule: DIAGNOSTIC-RULEID-ORPHAN
    description: "A ruleId emitted by some gate does not resolve to any ADR's enforces[] entry; binding-link gate fails."
  - rule: DIAGNOSTIC-RULEID-GENERIC-USED
    description: "A gate emitted a *-GENERIC ruleId in non-advisory mode after the grace period; warns to encourage migration to specific rule IDs."
  - rule: DIAGNOSTIC-RULEID-MALFORMED
    description: "A ruleId does not match the canonical pattern ^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$."
  - rule: DIAGNOSTIC-RULEID-COLLISION
    description: "Two ADRs claim the same ruleId in their enforces[] arrays; ambiguous binding."
applies_to:
  - "scripts/chassis/gates/**"
  - "schemas/diagnostics/diagnostic.schema.json"
  - "docs/adr/**"
tags:
  - chassis
  - governance
  - diagnostics
  - ruleid
---

# ADR-0011: Diagnostic ruleId stability discipline

## Context

ADR-0001 established the diagnostic schema and the per-gate
`*-GENERIC` fallback as a transitional mechanism: every gate's
findings would resolve to *something* in an ADR's `enforces[]`, even
before the gate adopted finer-grained rule IDs. The intent was that
gates would migrate to specific rule IDs over time, and the
`*-GENERIC` fallback would shrink.

Without an explicit policy, gates may stay on `*-GENERIC` forever,
which defeats the contract: agents consuming diagnostics can route
on rule IDs only when those IDs are stable and meaningful. This ADR
pins the discipline.

## Decision

1. **Every emitted ruleId must resolve to an ADR.** The
   `binding_link` gate enforces this by walking
   `chassis release-gate --json` output and checking each
   `finding.ruleId` against the registry built from
   `docs/index.json`.
2. **`*-GENERIC` is a transitional safety net, not a steady state.**
   ADR-0001 keeps `*-GENERIC` rules registered so binding-link does
   not fail during gate migration. After all gates have specific rule
   IDs for their finding types, `*-GENERIC` use becomes a warning
   (not error) via `DIAGNOSTIC-RULEID-GENERIC-USED`.
3. **ruleId pattern is fixed.** `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`. The
   diagnostic schema enforces the pattern at validation time. Gates
   that emit malformed ruleIds fail their own diagnostic schema
   conformance test.
4. **No ruleId collisions across ADRs.** Two ADRs claiming the same
   ruleId in `enforces[]` make binding ambiguous. The `chassis adr
   index` step detects collisions and fails.
5. **Migration etiquette.** When a gate moves from `*-GENERIC` to
   specific rule IDs, the new IDs are added to the relevant
   conceptual ADR (this one for governance, ADR-0010 for posture,
   etc.), and the gate's `result.add(...)` calls are updated to pass
   `rule_id=`. ADR-0001's `*-GENERIC` registration stays.

## Consequences

- AI agents consuming `chassis release-gate --json` can dispatch on
  rule ID with confidence: every ID is meaningful and bound to an
  ADR.
- Gate authors are forced to think about *what kind of finding this
  is* before emitting it. The `*-GENERIC` shortcut still works during
  migration but the warning makes the cost visible.
- The ADR collection grows over time, but each addition is bounded
  to a specific decision and rule-ID surface.

## Status

Accepted. binding_link enforcement shipped with ADR-0001 and is
extended here to govern the migration trajectory.
