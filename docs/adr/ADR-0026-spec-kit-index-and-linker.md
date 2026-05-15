---
id: ADR-0026
title: "Spec Kit index artifact, exporter, and spec-to-contract linker"
status: accepted
date: "2026-05-15"
supersedes: []
enforces:
  - rule: CH-SPEC-SOURCE-PARSE
    description: "Spec index YAML/JSON source cannot be parsed into the typed wire shape."
  - rule: CH-SPEC-DUPLICATE-ID
    description: "Duplicate principle, requirement, or task IDs in a spec index source export."
  - rule: CH-SPEC-ACCEPTANCE-MISSING
    description: "A requirement lists no acceptance criteria (preset export validation)."
  - rule: CH-SPEC-INDEX-SCHEMA
    description: "Emitted/Persisted spec-index.json fails JSON Schema validation."
  - rule: CH-SPEC-UNBOUND-REQUIREMENT
    description: "A requirement has no claim_ids binding it to CONTRACT.yaml claims."
  - rule: CH-SPEC-UNKNOWN-CLAIM-REF
    description: "A requirement references a claim_id not declared in any repo CONTRACT.yaml."
  - rule: CH-SPEC-INVALID-TASK-EDGE
    description: "A task depends_on references a non-existent task id."
  - rule: CH-SPEC-ORPHAN-TASK
    description: "A task id is not listed in any requirement.related_task_ids."
  - rule: CH-SPEC-RELATED-TASK-MISSING
    description: "A requirement lists a related_task_ids entry that does not name a task in the spec index."
  - rule: CH-SPEC-DUPLICATE-CLAIM-REF
    description: "A requirement lists the same claim_id more than once."
  - rule: CH-SPEC-CLAIM-IMPL-MISSING
    description: "A bound claim has no implementation sites in the trace graph."
  - rule: CH-SPEC-CLAIM-TEST-MISSING
    description: "A bound claim has no test sites in the trace graph."
  - rule: CH-SPEC-TOUCHED-PATH-UNCOVERED
    description: "A requirement or task touched_path is not listed by any CONTRACT.yaml exports[].path entry and does not appear on a traced impl/test site."
---

# ADR-0026 — Spec Kit index artifact, exporter, and spec-to-contract linker

## Context

Spec Kit is Markdown-first. Chassis needs machine-readable intent for policy,
trace, and governance exports without brittle Markdown scraping in CI.

## Decision

1. Canonical artifact: **`artifacts/spec-index.json`** validated by
   `schemas/spec-index.schema.json`.
2. Authors maintain **YAML** (`.chassis/spec-index-source.yaml`) merged from Spec
   Kit docs; **`chassis spec-index export`** canonicalizes and writes JSON.
3. **`chassis spec-index link`** joins requirements to CONTRACT claim IDs,
   validates related tasks, checks trace-graph implementation/test evidence for
   bound claims, and conservatively validates `touched_paths` against
   CONTRACT `exports[].path` entries plus traced sites. It reports `CH-SPEC-*`
   diagnostics consumed by `chassis export` and the release gate when
   `artifacts/spec-index.json` is present.
4. **`spec-index` YAML source is authoritative** — Spec Kit Markdown is not
   machine-parsed in v1; Markdown remains human documentation only.

## Consequences

- Policy input may include `spec_kit.spec_index_digest` when the artifact exists.
- OPA continues to evaluate merged diagnostics; spec linker errors are
  surfaced as standard Chassis diagnostics.
