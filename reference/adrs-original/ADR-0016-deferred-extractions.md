---
id: ADR-0016
title: Deferred extractions — record of modules intentionally not yet carved out of a legacy product workspace
status: accepted
date: "2026-04-20"
enforces:
  - rule: EXTRACTION-UNAUTHORIZED-SECOND-CARVE
    description: "A second module extraction landed before the first one completed one breaking-change cycle through the extraction gates."
  - rule: EXTRACTION-CANDIDATE-UNCLASSIFIED
    description: "A crate in scope of the 77-crate workspace review lacks a status row in ADR-0016's candidate table."
  - rule: EXTRACTION-MCP-TREATED-AS-MODULE
    description: "The MCP server surface is referenced as a chassis module candidate; it is an app, not a module (blueprint blind-spot #5)."
applies_to:
  - "docs/adr/ADR-0016-deferred-extractions.md"
  - "docs/chassis/guides/module-lifecycle.md"
tags:
  - chassis
  - governance
  - extraction
  - modularization
  - legacy-product
---

# ADR-0016: Deferred extractions

## Context

The 90-day blueprint pointed the legacy product modularization at a
**one-at-a-time carve-out cadence**: extract the lowest-coupling leaf,
run it through a full breaking-change cycle (y-bump to x-bump, app.lock
regeneration, tier-3 drift gate firing), and only then extract the
next. This ADR fixes the record at Day-90: what was extracted today,
what was considered and deferred, what should be merged or deleted
instead of extracted, and — per blueprint blind-spot #5 — what was
misclassified as a module when it is in fact an app.

On 2026-04-20 exactly one crate was extracted from the legacy product
workspace: `aux-provider-error`. It ships from the private git
registry at the configured private registry, rev
`13ae00be8344f950cf7990a90ce15aada2ce054a`, pinned from
`crates/Cargo.toml`. The workspace still contains 77 local members;
the next carve-out is blocked behind a breaking-change cycle on the
first.

The blueprint's Q2 scorecard scores each candidate on nine 0–2 axes
(library / plugin / service determination, coupling, deploy surface,
stability, etc.); scorecard sums drive the recommended shape. LOC
measurements below come from
`find crates/<name>/src -name '*.rs' | xargs wc -l`; dependent counts
come from `grep -l "^<name>\s*=" crates/*/Cargo.toml`.

## Decision

The following table is the record at Day-90. Every row is a real
crate from `crates/Cargo.toml`; LOC and dependent counts were
inspected, not invented. Scorecard sums are coarse O/1/2 estimates
per blueprint §Q2 axes (lib-shape, plugin-shape, service-shape,
coupling, stability, deploy-surface, schema-coupling, tooling-surface,
churn). A sum ≤ 6 suggests a library shape; 7–12 a plugin; 13+ a
service.

| module | LOC | dependents | scorecard sum | recommended shape | status | rationale |
|---|---|---|---|---|---|---|
| `aux-provider-error` | ~80 (extracted) | 1 (`aux-llm-provider`) | 4 | library | **extracted** | Lowest-coupling leaf; proved the extraction pipeline (registry publish, rev-pin, tier-3 gate round-trip). |
| `hub-proto` | 107 | 0 | 3 | library | **deferred: merge-into-`aux-server-grpc`** | Zero workspace dependents; only meaningful at the gRPC server boundary. Merge after tier-3 cycle on `aux-provider-error` completes. |
| `types` | 271 | 0 (facade; re-exports `types-internal` + `aux-config`) | 3 | library | **deferred: delete** | Thin re-export shim with zero direct `Cargo.toml` importers. Blueprint blind-spot #3 candidate: delete once `types-internal` + `aux-config` are imported directly by the five call sites that still use `aux-types`. |
| `aux-macro-program` | 311 | 3 | 7 | library | deferred | Stable surface but depends on `aux-skill-bundle` and `aux-execution`; wait for the macro-program DSL shape to settle before committing to an external pin. |
| `aux-workflow-control-plane` | 386 | 2 | 9 | plugin | deferred | Exposes a control-plane trait consumed by `aux-orchestration-kernel`. Carve-out requires freezing the WCP trait first; re-evaluate after `aux-orchestration-kernel` stabilizes. |
| `missions` | 404 | low | 8 | plugin | deferred | Evaluator harness: churns with eval surface. Re-evaluate when `eval` and `aux-eval` stop co-changing. |
| `aux-gateway` | 454 (7 files) | 1 | 6 | library | deferred | Thin channel trait layer, good library shape. Deferred because the only consumer path is the server surface, which is mid-refactor. |
| `extension` | 458 | 0 | 5 | library | **deferred: merge-into-`aux-agent` or delete** | Zero dependents in the workspace today; candidate for deletion pending a consumer audit. Revisit at quarterly re-review. |
| `aux-trust-boundary` | 499 | 4 | 11 | plugin | deferred | High internal fan-in; the trust-boundary trait surface is not yet stable. Re-evaluate after `aux-security` hardening pass finishes. |
| `perf-regression` | 533 | 0 | 6 | library | deferred | Tooling/xtask-adjacent; zero runtime dependents. Consider moving under `xtask` rather than extracting. |
| `server-audit` | 536 | low | 10 | plugin | deferred | Thin wrapper over `aux-audit`; coupled to `aux-server`. Merge or carve only after `aux-server` decomposition lands. |
| `aux-skill-bundle` | 889 | 1 | 8 | plugin | deferred | Defines the skill-bundle format consumed by `aux-skills`. Wait for skill-bundle schema fingerprint to stabilize via ADR-0015. |
| `aux-memory-provider` | 938 | 1 | 8 | plugin | deferred | Memory-provider trait; single consumer. Re-evaluate after the first app.lock cycle surfaces its capability fingerprint shape. |
| `aux-context-compressor` | 1 037 | 2 | 9 | plugin | deferred | Compression strategy layer over `aux-context`. Scorecard straddles library/plugin — needs stable consumer surface first. |
| `aux-checkpoint` | 1 077 | 1 | 8 | plugin | deferred | Checkpoint/restore; couples to `aux-orchestration-kernel` and `aux-memory`. Single consumer today, carve after orchestration kernel stabilizes. |
| `aux-execution` | 1 108 | 1 | 10 | plugin | deferred | Execution primitives consumed by `aux-macro-program`. Re-evaluate in concert with macro-program. |
| `aux-orchestration-kernel` | 1 379 | 5 | 13 | **service-adjacent** | deferred | High fan-in (five workspace dependents) and deploy-surface characteristics. Scorecard 13+ on the service axis; extracting as a library would invert the dependency shape. Needs deploy infra (ADR-0012 app.lock cycle) before carve-out. |
| `aux-provenance-bus` | 1 822 | 3 | 11 | plugin | deferred | Event bus: schema-coupling axis is high. Re-evaluate after ADR-0015 fingerprint adoption covers provenance event schemas. |
| `server` | 146 763 | many | 18 | service | **keep in-workspace** | The legacy gateway/service. Decomposing `server` is a workstream of its own; it is not a module-extraction target. |
| `database` | 129 544 | many | 17 | service | **keep in-workspace** | Embedded multi-model database engine. Same reasoning as `server`: decomposition happens inside the workspace before any carve-out is even meaningful. |
| `agent` | 105 670 | many | 16 | service | **keep in-workspace** | Agent runtime. Intra-workspace decomposition target, not an extraction candidate. |
| `memory` | 45 684 | many | 14 | plugin-or-service | deferred | Memory hierarchy. Large enough that extraction is premature; wait for internal decomposition. |
| `tools` | 29 631 | many | 13 | plugin | deferred | Tool registry and effect model. Scorecard straddles plugin/service; stabilize the effect model first. |
| **MCP server surface** (`aux-server` MCP routes) | N/A (part of `server`) | N/A | N/A | **app** | **not-a-module** | Blueprint blind-spot #5: the MCP server is an *app* composed from chassis-governed modules, not a chassis module itself. It belongs in an `app.yaml` (ADR-0012), not in the module registry. Record here so future re-reviews do not re-raise it. |

Counts of entries above the horizontal boundary: 15 `aux-*` candidate
rows plus the five largest non-`aux-*` crates by LOC (`server`,
`database`, `agent`, `memory`, `tools`) plus three additional
small-and-revealing rows (`hub-proto`, `types`, `extension`, and
`perf-regression` as a tooling candidate). The MCP row sits outside
the table body because it is deliberately **not a module**.

### Rules the record enforces

1. **One extraction per breaking-change cycle.** A second module
   does not enter the registry until `aux-provider-error` has
   shipped a y-bump + x-bump sequence and both fired the tier-3
   drift gate in the consumer. Violating this triggers
   `EXTRACTION-UNAUTHORIZED-SECOND-CARVE`.
2. **Every in-scope crate has a row.** The scope is "the 15
   `aux-*`-prefix crates plus the five largest non-`aux-*` crates".
   A crate in scope without a row triggers
   `EXTRACTION-CANDIDATE-UNCLASSIFIED`.
3. **Delete-don't-extract is a first-class outcome.** `types` and
   `extension` are candidates for deletion, not extraction.
   Blueprint blind-spot #3 is explicit: the 77-crate workspace
   contains crates that exist as accidents of history.
4. **MCP is an app.** `EXTRACTION-MCP-TREATED-AS-MODULE` fires if
   the MCP server surface is proposed as a chassis module. The
   composition artifact for MCP lives in `app.yaml`.
5. **Re-review is quarterly.** Each deferred row carries an implicit
   re-review checkpoint: "revisit at the next quarterly review, or
   sooner if the rationale's preconditions change." The chassis
   module lifecycle guide (`docs/chassis/guides/module-lifecycle.md`)
   owns the cadence; this ADR owns the list.

## Consequences

- The record is authoritative for the quarterly re-review. New
  ADRs do not need to re-litigate the Day-90 decisions; they
  update rows here.
- The "keep in-workspace" entries (`server`, `database`, `agent`)
  make explicit that intra-workspace decomposition happens *before*
  extraction is meaningful. Pushing them out of the workspace as
  they stand today would export churn to the registry.
- Deletion candidates (`types`, `extension`) turn into follow-up
  PRs against the downstream product workspace, not chassis ADRs.
- The MCP row defuses a predictable recurring confusion: the
  MCP surface will surface in design discussions as "another
  module to extract" every few months. The ADR's answer is: no,
  it is an app.

## Alternatives considered

- **Extract everything at once.** Blueprint's explicit rejection:
  the version-skew / schema-drift blast radius from N simultaneous
  carve-outs is unmanageable. ADR-0013 (runtime-library split)
  and ADR-0015 (schema fingerprint identity) are the scaffolding
  that makes sequential extraction possible; batch extraction
  nullifies both.
- **Keep everything in-workspace.** Defeats the whole
  modularization program. The stated goal is 8–12 composable
  modules; staying at 77 crates concedes the point.
- **Split by team ownership, not by coupling.** Chrome Mojo
  precedent: the wrong axis. Team boundaries reorganize; coupling
  doesn't. Extract along the coupling graph.
- **Classify every crate, not just top-20.** The top-20 cover the
  decisions with non-trivial consequences; below that, the
  scorecard sums cluster at "obviously library, obviously leaf,
  no one depends on it" — delete-or-merge candidates, which is
  captured in the re-review cadence rather than the table.

## References

- ADR-0012 (app composition artifact): MCP surfaces as an app.yaml,
  not a module.
- ADR-0013 (runtime-library split): the stability discipline that
  makes sequential extraction tractable.
- ADR-0015 (schema fingerprint identity): the drift primitive each
  extraction relies on.
- `docs/chassis/guides/module-lifecycle.md`: the cadence and
  re-review process that consumes this record.
- Downstream extraction roadmap and module-extraction playbook:
  product-owned follow-ups derived from this ADR.
- Blueprint §Q2 scorecard; blueprint blind-spots #3 and #5.

## Status

Accepted. Day-90 record. Next re-review: 2026-07-20 (quarterly),
or earlier if `aux-provider-error` completes its breaking-change
cycle before then.
