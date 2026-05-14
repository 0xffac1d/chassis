---
id: ADR-0031
title: Doc-graph guide coverage
status: accepted
date: "2026-04-27"

enforces:
  - rule: CHASSIS-DOCGRAPH-GUIDE-UNINDEXED
    description: "A markdown file under docs/chassis/guides/ or skills/*/SKILL.md exists on disk but is not declared as a kind: guide node in config/chassis.doc-graph.yaml."
  - rule: CHASSIS-DOCGRAPH-GUIDE-MISSING-FILE
    description: "A kind: guide node in config/chassis.doc-graph.yaml references a path that does not exist on disk."
  - rule: CHASSIS-DOCGRAPH-GUIDE-PARENT-UNRESOLVED
    description: "A kind: guide node declares a parent that is not present in the doc-graph (neither an ADR nor a canonical doc node)."
applies_to:
  - "config/chassis.doc-graph.yaml"
  - "schemas/policy/doc-graph.schema.json"
  - "scripts/chassis/gates/doc_graph_check.py"
  - "docs/chassis/guides/**/*.md"
  - "skills/*/SKILL.md"
supersedes: []
tags:
  - chassis
  - doc-graph
  - governance
  - guides
---

# ADR-0031: Doc-graph guide coverage

## Context

ADR-0023 introduced the doc-graph policy and the `doc-graph-check` gate.
Authored content under `docs/chassis/guides/**/*.md` and `skills/*/SKILL.md`
is already declared in `config/chassis.doc-graph.yaml` (67 guide nodes plus
15 skill nodes at the time of writing) — node registration itself is
present. The gate already emits a "file on disk not declared in doc-graph"
finding and the inverse `CHASSIS-DOCGRAPH-NODE-MISSING-FILE` for missing
files.

The actual gap is not registration but **enforcement strictness**. The
existing on-disk-not-declared finding fires as a *warning*, not an error,
and applies uniformly to every authority surface (the same rule absorbs
both genuine governance gaps and known-deferred files like
`docs/adr/.chassis/placeholders.json`). A new guide can therefore be added
to disk without a doc-graph node and the gate will not block CI. Likewise,
a guide-node `parent` that does not resolve to an ADR or canonical root
doc is not specifically caught — it falls through into the generic orphan
checks.

The result is that the chassis depends on author discipline (and the
`docs/chassis/guides/` and `skills/` trees being well-curated today) rather
than on the gate. ADR-0031 promotes coverage of those two specific surfaces
from convention to invariant.

## Decision

A new gate function `_check_guide_coverage` is added to
`scripts/chassis/gates/doc_graph_check.py`. It walks two surfaces explicitly
scoped by configuration — `docs/chassis/guides/**/*.md` and
`skills/*/SKILL.md` (plus `skills/*/EXAMPLES.md` where present) — and emits
hard-fail findings for the three guide-specific failure modes:

- `CHASSIS-DOCGRAPH-GUIDE-UNINDEXED` — file exists on disk under a guide
  surface but is not declared as a `kind: guide` node. (Promotes the
  existing soft warning to an error, scoped to guides only.)
- `CHASSIS-DOCGRAPH-GUIDE-MISSING-FILE` — a `kind: guide` node references
  a path with no file on disk. (Specialization of the existing
  `CHASSIS-DOCGRAPH-NODE-MISSING-FILE` rule, narrower scope and explicit
  about the guide surface.)
- `CHASSIS-DOCGRAPH-GUIDE-PARENT-UNRESOLVED` — a `kind: guide` node
  declares a `parent` that does not resolve to an ADR id, a canonical
  root doc, or another tracked node.

`config/chassis.doc-graph.yaml` gains a top-level `guide_coverage:` block
listing the include globs and an `exclude_patterns:` list (for example,
`docs/chassis/guides/schema-migrations/_template.md`, which ADR-0008
defines as the on-demand migration template and which is already
allowlisted under `release-public-surface-allowlist.yaml`). The schema at
`schemas/policy/doc-graph.schema.json` is extended to permit the new key.

The gate ships warn-only for one minor release with a baseline at
`config/chassis/baselines/doc-graph-guide-coverage.json` capturing any
genuine grandfathered exceptions. Coverage is currently complete (every
guide on disk has a node), so the baseline is expected to be empty;
warn-only is retained as a safety margin in case the audit missed a
file. The next minor release promotes to error.

## Consequences

- Adding a guide without a doc-graph node fails CI. Today the same
  omission produces a warning that scrolls past on a green run.
- Guide-node parents are validated against the doc-graph specifically,
  not just against on-disk file existence.
- The skills tree is first-class alongside guides under the same
  orphan-policy gate, removing the implicit two-tier treatment.
- Stale guide nodes (declared in graph but file deleted) are caught with
  a guide-specific message rather than the generic `NODE-MISSING-FILE`
  emission.
- The pre-existing soft warning for non-guide authority files
  (`DECISIONS.md`, `docs/adr/.chassis/placeholders.json`) is unchanged —
  this ADR scopes the strictness promotion to the two guide surfaces and
  does not touch other authority files.

## Alternatives considered

- **A separate `guide-coverage` gate module.** Rejected: the existing
  `doc-graph-check` already loads the graph, has the rule-emission helpers
  wired, and is the natural seam. A second module would duplicate graph
  loading and proliferate ruleId namespaces.
- **Treat skills as a separate surface with its own gate.** Rejected:
  `skills/*/SKILL.md` is doc-graph-shaped (canonical surface, parent
  authority). Splitting into two gates fragments the orphan policy without
  benefit.

## References

- ADR-0022 (metadata scaffold pattern): governs how new chassis metadata
  files come into existence; doc-graph nodes for guides do not bypass it.
- ADR-0023 (structural integrity policy): introduces the doc-graph and the
  `CHASSIS-DOCGRAPH-ORPHAN-NODE` rule this ADR extends.
- `scripts/chassis/gates/doc_graph_check.py`: implementation site.
- `config/chassis.doc-graph.yaml`: authoritative node registry.
- `config/chassis/baselines/doc-binding.json`: baseline-shape precedent.

## Status

Accepted. Gate-extension and config changes land in a follow-up PR (Gate A
in the chassis self-governance hardening plan).
