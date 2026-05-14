---
id: ADR-0023
title: Structural integrity policy (layout + doc-graph)
status: accepted
date: "2026-04-26"
applies_to:
  - "config/chassis.layout.yaml"
  - "config/chassis.doc-graph.yaml"
  - "schemas/policy/layout.schema.json"
  - "schemas/policy/doc-graph.schema.json"
  - "scripts/chassis/gates/**"
enforces:
  - rule: CHASSIS-LAYOUT-FORBIDDEN-FILE
    description: "A file or directory basename matches a layout forbidden_patterns[] entry for its directory."
  - rule: CHASSIS-LAYOUT-UNKNOWN-FILE
    description: "A file basename is not allowed by the matching path rule’s allowed_files[] (when present)."
  - rule: CHASSIS-LAYOUT-UNKNOWN-DIRECTORY
    description: "A subdirectory is not in allowed_directories (when the rule defines an allowlist) at the rule anchor path."
  - rule: CHASSIS-LAYOUT-NAMING-VIOLATION
    description: "A file or directory name violates the rule’s naming_convention or naming_pattern."
  - rule: CHASSIS-LAYOUT-MISSING-FRONTMATTER
    description: "A markdown or YAML file is missing a required key in frontmatter or as top-level YAML."
  - rule: CHASSIS-LAYOUT-MISSING-SECTION
    description: "A markdown file is missing a required H2 section."
  - rule: CHASSIS-LAYOUT-UNCLASSIFIED-PATH
    description: "No config/chassis.layout.yaml paths[] rule classifies the path."
  - rule: CHASSIS-LAYOUT-EXEMPTED
    description: "A finding is suppressed and replaced with an info diagnostic when a matching .exemptions/registry.yaml entry exists."
  - rule: CHASSIS-DOCGRAPH-PARENT-CYCLE
    description: "A node's parent chain forms a cycle in the authority graph."
  - rule: CHASSIS-DOCGRAPH-SUPERSEDES-CYCLE
    description: "An ADR supersedes chain forms a cycle."
  - rule: CHASSIS-DOCGRAPH-SYSTEM-DEP-CYCLE
    description: "architecture.yaml system depends_on edges form a cycle."
  - rule: CHASSIS-DOCGRAPH-PROJECTION-PARENT-UNDECLARED
    description: "A projection node references a parent canonical that is not declared in the doc graph."
  - rule: CHASSIS-DOCGRAPH-PROJECTION-PARENT-MISSING
    description: "A projection node's declared parent file does not exist on disk."
  - rule: CHASSIS-DOCGRAPH-SUPERSEDES-ASYMMETRIC
    description: "ADR supersedes link is not mirrored by the superseded ADR's superseded_by frontmatter."
  - rule: CHASSIS-DOCGRAPH-SUPERSEDES-UNRESOLVED-TARGET
    description: "ADR frontmatter supersedes target does not resolve to any known ADR id."
  - rule: CHASSIS-DOCGRAPH-DUPLICATE-ADR-ID
    description: "Two or more ADR files declare the same ADR id."
  - rule: CHASSIS-DOCGRAPH-NODE-MISSING-FILE
    description: "A doc-graph node references a path that does not exist on disk."
  - rule: CHASSIS-DOCGRAPH-HISTORICAL-DELETED
    description: "A node marked historical references a path that should remain on disk for archive purposes."
  - rule: CHASSIS-DOCGRAPH-ORPHAN-NODE
    description: "A node has no parent edge and is not declared as a root canonical."
  - rule: CHASSIS-DOCGRAPH-NODE-UNREACHABLE
    description: "A non-root node is not reachable from any declared root canonical via parent or projection edges."
  - rule: CHASSIS-DOCGRAPH-POLICY-INVALID
    description: "config/chassis.doc-graph.yaml fails to load or validate against schemas/policy/doc-graph.schema.json."
  - rule: PATH-EXISTENCE-MAKEFILE-MANIFEST
    description: "The path-existence gate found a Makefile that does not declare expected chassis targets."
  - rule: PATH-EXISTENCE-TOOLCHAIN-COMMENT
    description: "The path-existence gate found a toolchain file with unexpected comment structure."
  - rule: PATH-EXISTENCE-MD-LINK
    description: "The path-existence gate found a markdown file with a broken or unindexed link."
  - rule: AGENT-SURFACE-FRESH
    description: "The agent-surface-freshness gate reports that agent instruction files are fresh (info-level)."
  - rule: SKILL-FRESHNESS
    description: "The skill-freshness gate reports that canonical skills are fresh (info-level)."
supersedes: []
tags:
  - chassis
  - layout
  - governance
  - architecture
---

# ADR-0023: Structural integrity policy

## Context

Structural rules exist as prose, narrow point-checks, and per-file
frontmatter. No declarative source-of-truth for layout exists; no gate
enforces authority-graph acyclicity. The result is predictable drift:
agents create root-level clutter, ADR supersedes chains diverge, and
duplicate ADR files (the ADR-0021 incident) escape detection until someone
notices manually.

## Decision

Author two YAML policy files: `config/chassis.layout.yaml` (filesystem
layout, naming, required frontmatter, forbidden patterns per directory)
and `config/chassis.doc-graph.yaml` (authority graph: canonical
surfaces, projections, supersedes chains, and system-level dependencies
from `architecture.yaml`). Both validate against JSON Schemas under
`schemas/policy/`. Two gates, `layout-check` and `doc-graph-check`, run on
every pull request. `scripts/chassis/validate_distribution_layout.py` is
subsumed by `layout-check` and is removed once `layout-check` covers the
same checks.

## Consequences

- Structural rules become machine-testable. Adding a new file kind
  requires a layout policy change and an ADR when the change is
  structural.
- `architecture.yaml` dependency edges are checked for cycles together
  with the doc graph.
- Duplicate ADR id collisions are caught at gate time instead of by
  inspection.
- `CONVENTIONS.md` remains a projection of `AGENTS.md` (current state).
  A future amendment could fold layout descriptions into `AGENTS.md`
  where appropriate; this ADR does not require that move.

## Alternatives considered

- **Retain only prose and ad-hoc scripts.** Rejected: it preserves the
  current failure modes and does not scale with agent-driven churn.
- **A single combined policy file.** Rejected: filesystem rules and
  authority graphs evolve on different axes; splitting keeps reviews
  focused and schemas smaller.

## References

- ADR-0022 (metadata scaffold pattern): companion metadata governance.
- ADR-0014 (governance tiers): tiered assurance for consumer surfaces.
- `scripts/chassis/validate_distribution_layout.py`: predecessor with
  narrower distribution-marker scope.
- `architecture.yaml`: system nodes and `depends_on` input to the
  doc-graph gate.
- `EXPORT_SURFACE.json`: publishable-surface manifest input to the
  doc-graph policy.

## Status

Accepted. Policy YAML authoring and gate wiring land in follow-up work.
