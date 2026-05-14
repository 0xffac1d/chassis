---
id: ADR-0001
title: Gate diagnostic contract — ruleId format, GENERIC fallback, per-gate adoption
status: accepted
date: "2026-04-19"
enforces:
  - rule: AGENT-HYGIENE-GENERIC
    description: "agent-hygiene gate findings (commit trailer policy, scope discipline)"
  - rule: AGENT-TELL-GENERIC
    description: "agent-tell gate findings (agent prompt-injection sentinel hits)"
  - rule: CLAIM-DRIFT-GENERIC
    description: "claim-drift gate findings (numeric claim proof failures)"
  - rule: COMPLETENESS-GENERIC
    description: "completeness gate findings (invariant/test_linkage coverage)"
  - rule: DECOMPOSITION-GENERIC
    description: "decomposition gate findings (file size, module fan-out)"
  - rule: DOC-BINDING-GENERIC
    description: "doc-binding gate findings (doc path references, docs/code ratio)"
  - rule: PANIC-BUDGET-GENERIC
    description: "panic-budget gate findings (Rust panic!() / unwrap() regressions)"
  - rule: POSTURE-AUDIT-GENERIC
    description: "posture-audit gate findings (security-sensitive config default posture)"
  - rule: PROCESS-RATIO-GENERIC
    description: "process-ratio gate findings (process surface vs test ratio)"
  - rule: SURFACE-WIRING-GENERIC
    description: "surface-wiring gate findings (route classification)"
  - rule: RELEASE-GATE-GENERIC
    description: "release-gate orchestrator findings"
  - rule: BINDING-LINK-ORPHAN-RULE
    description: "A diagnostic ruleId does not resolve to any ADR's enforces[] entry"
  - rule: BINDING-LINK-NO-REGISTRY
    description: "docs/index.json not yet built; run `chassis adr index`"
  - rule: BINDING-LINK-ORCHESTRATOR-FAIL
    description: "binding-link could not parse release-gate JSON output"
  - rule: BINDING-LINK-GENERIC
    description: "binding-link gate findings without a more specific ruleId"
  - rule: DRIFT-GUARD-AGENT-SURFACE
    description: "AGENTS.md derivative out of sync; run `chassis emit agent-surface`"
  - rule: DRIFT-GUARD-SUPPORT-MATRIX
    description: "support-matrix.json/.md under docs/chassis/reference/ out of sync; run `chassis support-matrix`"
  - rule: DRIFT-GUARD-SCHEMA-CATALOG
    description: "schemas/catalog.md or artifact-kinds.json out of sync; run `python3 scripts/chassis/schema_catalog.py`"
  - rule: DRIFT-GUARD-GENERIC
    description: "drift-guard gate findings without a more specific ruleId"
  - rule: CHASSIS-CONSUMER-FIXTURE-MATRIX
    description: "consumer-fixture-matrix gate: standalone consumer fixture harness failed"
  - rule: CHASSIS-AGENT-PROJECTION-MISSING-WRITER
    description: "An enabled agent projection entry has no registered writer"
  - rule: CHASSIS-AGENT-PROJECTION-ORPHAN-WRITER
    description: "A registered agent projection writer lacks a config entry or writes outside the declared target"
  - rule: CHASSIS-AGENT-PROJECTION-MISSING-TARGET
    description: "An enabled agent projection target or writer output is missing"
  - rule: CHASSIS-AGENT-PROJECTION-STALE-TARGET
    description: "An agent projection target lacks the generated projection marker"
  - rule: CHASSIS-AGENT-PROJECTION-DISABLED-BUT-PRESENT
    description: "A disabled agent projection entry still has its target on disk"
  - rule: NAMING-RUST-TYPE-PASCAL
    description: "Rust type declarations must be PascalCase"
  - rule: NAMING-RUST-FN-SNAKE
    description: "Rust function names must be snake_case"
  - rule: SUPPRESSION-RUST-ALLOW-WITHOUT-EXEMPTION
    description: "#[allow(...)] attribute lacks paired EX-YYYY-NNNN exemption marker"
  - rule: SUPPRESSION-TS-ESLINT-DISABLE-WITHOUT-EXEMPTION
    description: "eslint-disable comment lacks paired EX-YYYY-NNNN exemption marker"
  - rule: VALIDATE-MANIFEST-GENERIC
    description: "MCP validateManifest verb findings"
applies_to:
  - "scripts/chassis/gates/**"
  - "schemas/diagnostics/diagnostic.schema.json"
tags:
  - chassis
  - gates
  - diagnostics
  - milestone-a
---

# ADR-0001: Gate diagnostic contract

## Context

Milestone A of the Chassis completion plan introduces `schemas/diagnostics/diagnostic.schema.json` — a structured, agent-routable shape for every finding emitted by a Chassis gate. The schema requires each finding to carry a `ruleId`, and the binding-link gate (A.5) fails on any `ruleId` that does not resolve to an ADR.

Before Milestone A, the 11 expansion gates (`agent-hygiene`, `agent-tell`, `claim-drift`, `completeness`, `decomposition`, `doc-binding`, `panic-budget`, `posture-audit`, `process-ratio`, `surface-wiring`) shared a uniform `{level, detail, path}` shape via `scripts/chassis/gates/_common.py::Finding`, but without stable rule IDs.

## Decision

1. **`ruleId` format.** `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$` (e.g. `CHASSIS-001`, `GATE-EXPORT-03`, `DOC-BINDING-GENERIC`). Enforced by the diagnostic schema.

2. **Per-gate GENERIC fallback.** Until each gate adopts finer-grained rule IDs, `Finding.to_diagnostic()` defaults every finding without an explicit `rule_id` to `<GATE_NAME>-GENERIC` (with `_` → `-`). This ADR formalizes all 11 gate-level GENERIC rules as the initial convention surface so the binding-link gate has something to resolve to.

3. **Adoption path.** As gates migrate from "one GENERIC rule per gate" to specific rules per finding type, new rules are added to this ADR (or a successor ADR) and the finding's `rule_id` is set explicitly in the gate code. Old GENERIC entries stay in `enforces[]` until no callsite emits them.

4. **Consumer contract.** Agents consuming `chassis release-gate --json` SHOULD dispatch on `ruleId`. When `ruleId` ends in `-GENERIC`, the agent treats the finding as "convention-aware but not yet rule-specific" — useful for routing but not for auto-fix.

## Consequences

- Every diagnostic emitted via `--json` satisfies the binding-link gate's resolution check today. The system is closed-world from the start.
- Gates can migrate to finer-grained rules one at a time, opportunistically, without breaking the binding-link invariant.
- A downstream product-scope ADR could add rules like `SECURITY-PIPELINE-STAGE-COUNT`, `OSCAL-PROFILE-VALID`, etc. that the consumer-scope gate configs emit.

## Status

Accepted. Initial version shipped with Milestone A.4 (see `docs/roadmap/chassis-completion-plan-2026-04-17.md`). Next revision expected when two or more gates complete adoption of fine-grained rule IDs and retire their GENERIC fallback.
