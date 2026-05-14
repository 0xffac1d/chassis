# Chassis evolution roadmap

This document captures **prioritized backlog** for moving Chassis from a strong **metadata and structure** layer toward **objective-aligned assurance**: a durable chain from intent → claims → proof → enforcement → observed evidence.

It does not supersede [`PROTOCOL.md`](PROTOCOL.md) (precedence, versioning) or [`guides/tooling-by-language.md`](guides/tooling-by-language.md) (honest support depth). It **extends** them with planned work.

**Positioning honesty:** Today, Chassis is best described as a **language-agnostic metadata protocol** plus **TypeScript-first reference/runtime** for part of the surface, with **partial automation** elsewhere. Claims of uniform adherence across languages and transports apply only under the `strict` [enforcement profile](guides/enforcement-profiles.md); the advisory and standard profiles explicitly surface the same findings as warnings. The [support matrix](#p2-product-clarity-machine-readable-support-matrix) distinguishes advisory from enforced capabilities per language.

---

## Goals (what “done” means)

| Goal | Meaning |
|------|---------|
| **First-class objectives** | Product goals, constraints, acceptance criteria, and traceability to contracts, tests, evals, and telemetry — not only loose prose or downstream-only artifacts. |
| **Stable claim identity** | Invariants, edge cases, and requirements use **stable IDs**; proof links never depend on rewording or array index. |
| **Assurance ladder** | Every artifact and claim can report **declared → coherent → verified → enforced → observed** (see `assurance_level` on `CONTRACT.yaml`) with clear promotion rules. |
| **Fewer overlapping author surfaces** | One canonical artifact per concern where possible; projections and generated docs reduce hand-maintained duplication. |
| **Tighter schemas where it matters** | API, state, service, and component models are constrained enough for machine reasoning, with transport-specific families where a universal shape is misleading. |

---

## P0 — Alignment, proof, and identity

Ship these before implying “framework-grade” objective alignment.

| ID | Work | Status |
|----|------|--------|
| **P0.1** | **Objective / requirement model** | **Done (v1):** [`schemas/objective/objective-registry.schema.json`](../../schemas/objective/objective-registry.schema.json), [`config/chassis.objectives.yaml`](../../config/chassis.objectives.yaml), [`validate_objectives.py`](../../scripts/chassis/validate_objectives.py), wired into **`validate-all`**. See [`guides/objectives-registry.md`](guides/objectives-registry.md). |
| **P0.2** | **Stable IDs on every claim** | **Done (v1):** optional `id` on object-form `invariants` / `edge_cases`; `test_linkage.claim_id` prefers ids; semantics warn for unresolved slug-style ids and legacy excerpt↔id mix. Optional migration of existing contracts. |
| **P0.3** | **Assurance ladder** | **Done (v1):** optional `assurance_level` enum on `CONTRACT.yaml` (`declared` … `observed`). Promotion rules still policy. |
| **P0.4** | **Proof ratchet** | **Unchanged:** existing `status: stable` + `test_linkage` warnings; stricter CI thresholds remain future work. |
| **P0.5** | **Single support matrix** | **Done (v1):** [`config/chassis.support-matrix.yaml`](../../config/chassis.support-matrix.yaml) — keep aligned with [`guides/tooling-by-language.md`](guides/tooling-by-language.md). |

**Dependency note:** P0.2 unblocks durable traceability for P0.1. P0.5 should land early to stop internal drift while schemas evolve.

---

## P1 — Semantic tightening and ontology

| ID | Work | Status / artifacts |
|----|------|--------------------|
| **P1.1** | **Controlled tag ontology** | **Baseline:** [`schemas/common/tag-ontology.schema.json`](../../schemas/common/tag-ontology.schema.json). |
| **P1.2** | **API contract families** | **Progress:** optional `apiFamily` on [`schemas/api/endpoint.schema.json`](../../schemas/api/endpoint.schema.json). |
| **P1.3** | **State and effects** | **Baseline:** [`schemas/state/effect.schema.json`](../../schemas/state/effect.schema.json). |
| **P1.4** | **UI / design system depth** | **Progress:** optional `designSystem` on [`schemas/component/component.schema.json`](../../schemas/component/component.schema.json). |
| **P1.5** | **Services and ops** | **Progress:** optional `deployment` on [`schemas/service/service.schema.json`](../../schemas/service/service.schema.json). |
| **P1.6** | **Workflow depth** | **Progress:** v1.1 [`schemas/architecture/workflow.schema.json`](../../schemas/architecture/workflow.schema.json). |
| **P1.7** | **Manifest shape** | **Existing:** `extensions` on [`schemas/metadata/contract.schema.json`](../../schemas/metadata/contract.schema.json). |

---

## P2 — Positioning, governance, default gates

| ID | Work | Status / artifacts |
|----|------|--------------------|
| **P2.1** | **Core vs adapters vs runtime** | **Doc:** [`guides/core-adapters-runtime.md`](./guides/core-adapters-runtime.md). |
| **P2.2** | **Responsible-system schema layer** | **Baseline:** [`schemas/policy/responsible-system.schema.json`](../../schemas/policy/responsible-system.schema.json). |
| **P2.3** | **Promote expansion-pack gates** | **Clarified:** `chassis release-gate` runs 19 gates; `validate-all` = layout + validate + domain + objectives — [`guides/expansion-pack.md`](./guides/expansion-pack.md). |
| **P2.4** | **Dogfood** | Ongoing. Do not add new `jsonschema.RefResolver` imports; use `jsonschema_support.py`. The `RefResolver` in `codegen/` is a different helper. |
| **P2.5** | **Routing** | **Progress:** `pathParamNames` / `searchParamNames` on [`schemas/domain/route.schema.json`](../../schemas/domain/route.schema.json). |

---

## Suggested sequencing

1. **P0.5 → P0.2 → P0.3** — Truth surface and stable identity first.  
2. **P0.1** — Minimum objective/requirement schema tied to P0.2 IDs.  
3. **P0.4 + P2.4** — Ratchet + dogfood in parallel.  
4. **P1.x** — Pick API/state/workflow/UI items by product demand.  
5. **P2.1–P2.3** — Positioning and default gates once P0 stabilizes.

---

## Related reading

| Topic | Document |
|--------|----------|
| Precedence (runtime, tests, contract, prose) | [`PROTOCOL.md`](PROTOCOL.md) |
| Observed vs declared truth | [`guides/observed-truth.md`](guides/observed-truth.md) |
| Honest per-language tooling | [`guides/tooling-by-language.md`](guides/tooling-by-language.md) |
| Expansion pack gates | [`guides/expansion-pack.md`](guides/expansion-pack.md) |
| Contract testing linkage | [`guides/contract-testing.md`](guides/contract-testing.md) |
| Coherence report | [`guides/coherence-report.md`](guides/coherence-report.md) |
| Objective registry | [`guides/objectives-registry.md`](guides/objectives-registry.md) |
| Support matrix (machine-readable) | [`../../config/chassis.support-matrix.yaml`](../../config/chassis.support-matrix.yaml) |

---

## Maintenance

When backlog items **ship**, update this file: mark rows done, link ADRs or schema changelogs, and adjust **Positioning honesty** if support depth changes.
