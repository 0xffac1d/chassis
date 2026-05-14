---
id: ADR-0002
title: "Assurance ladder semantics — evidence, promotion, demotion, and MVP unblock order"
status: accepted
date: "2026-05-14"
enforces:
  - rule: ASSURANCE-RUNG-NO-SKIP
    description: "A claim cannot advance to rung N unless evidence exists for every lower rung in order."
  - rule: ASSURANCE-PROMOTION-EVIDENCE-REQUIRED
    description: "Promotion to any rung above declared requires verifier-emitted artifacts enumerated in this ADR."
  - rule: ASSURANCE-DEMOTION-LOGGED
    description: "Demotion is never silent; it must be recorded in attestation output or verifier diagnostics."
applies_to:
  - "schemas/contract.schema.json"
  - "schemas/coherence-report.schema.json"
  - "**/CONTRACT.yaml"
  - "docs/ASSURANCE-LADDER.md"
tags:
  - foundation
  - assurance
---

## Context

The five-rung assurance ladder is load-bearing vocabulary for how strongly a `CONTRACT.yaml` claim is evidenced. ADR-0001 scopes this repository to Rust + TypeScript and explicitly avoids overstating enforcement before an enforcement point exists. The historical ADR in `reference/adrs-original/ADR-0002-assurance-ladder.md` coupled assurance to Python gates and optional `assurance_level`; this rebuild makes evidence artifacts and verifier boundaries explicit so downstream waves (trace graph, attestation, MCP) share one semantics contract.

## Decision

### Canonical rungs (ordinal)

The ladder remains `declared → coherent → verified → enforced → observed`. The **manifest** carries `assurance_level` as the author’s declared ceiling for the module; **claims** advance only on verifier evidence (see promotion).

### Verifier artifact per rung

Each rung’s verifier emits a concrete artifact (or a deterministic projection into the release attestation). Implementations may wrap these behind a single CLI entrypoint later; the artifact types are stable contracts.

| Rung | Verifier (planned surface) | Evidence artifact |
|------|---------------------------|-------------------|
| `declared` | `chassis validate` / `CanonicalMetadataContractValidator` | The CONTRACT document instance after it validates against `schemas/contract.schema.json` (no separate sidecar file required). |
| `coherent` | `chassis coherence` | `coherence-report.json` validating against `schemas/coherence-report.schema.json`, with **zero** unresolved references touching the claim’s linkage closure (depends_on, ADR refs, code pointers, exemptions). |
| `verified` | `chassis trace` + CI test runner integration | A **test-result bundle** embedded in the signed release attestation (or emitted as JSON alongside it) listing passing tests keyed by `claim_id` / stable linkage IDs. |
| `enforced` | Enforcement points (CI gates, MCP tool guards, admission checks) | **Enforcement diagnostics log**: structured records that a gate denied an action or merge because a specific `ruleId` / claim linkage fired (not merely that a unit test could observe failure). |
| `observed` | Telemetry / observability pipeline | **Observed-events batch** keyed by `claim_id` with sampling window metadata (production evidence; distinct from CI test passes). |

Naming note (ADR-0001): until an enforcement point exists, user-facing documentation avoids calling this layer a “runtime”; this ADR uses **enforcement point** and **telemetry pipeline** instead.

### Promotion rule (no skipping)

Let \(R(c)\) be the highest rung for claim \(c\) with **non-stale** evidence in the current evaluation window.

1. **Sequential evidence:** For every rung strictly below \(R(c)\), corresponding evidence **must exist and be non-stale**. You cannot assert \(R(c)=\texttt{enforced}\) if \(c\) lacks current `verified` evidence, even if enforcement coincidentally passes.
2. **Manifest ceiling:** `assurance_level` on the CONTRACT is an upper bound the authors intend to maintain; verifiers reject manifests whose ceiling is below evidenced failure (e.g., stale coherence with unresolved refs while declaring `coherent`).
3. **Attestation truth:** The release attestation records the **current** rung per claim after evaluation — not historical maximum.

### Demotion

Demotion occurs when evidence for rung \(k\) is missing, stale, or contradicted (tests fail, coherence unresolved, enforcement sinks silent, telemetry shows violation).

**Demotion is logged:** emit `ASSURANCE-DEMOTION-LOGGED` diagnostics and capture the downgrade in attestation output (previous rung → new rung + reason code). Silent downgrade is forbidden.

### MVP commitment & unblock order

Per ADR-0001, **only `declared` is implementable today** via JSON Schema validation.

Unblock order for higher rungs:

1. **`coherent`** — coherence walker over repo manifests + `coherence-report.json` emitter bound to `schemas/coherence-report.schema.json`.
2. **`verified`** — static claim annotations + trace graph + CI bridge attaching `claim_id` to test outcomes consumable by attestation.
3. **`enforced`** — first concrete enforcement point (CI and/or MCP guard) that denies violating actions with stable `ruleId` linkage.
4. **`observed`** — telemetry ingestion producing observed-events batches referenced from attestation.

## Consequences

- Downstream tooling must treat assurance as **evidence-backed ordinals**, not decorative strings on YAML.
- Contract-diff and trace implementations must preserve claim identity (`claim_id`) so promotion history remains explainable.

## Relationship to predecessor

`reference/adrs-original/ADR-0002-assurance-ladder.md` tied coherence to `depends_on` resolution and referenced Python gates. This ADR keeps the ordinal semantics but re-binds artifacts to the salvaged schema set, defers Python-specific gates, and encodes **no-skip** / **demotion logged** as explicit rule IDs for future CI wiring.
