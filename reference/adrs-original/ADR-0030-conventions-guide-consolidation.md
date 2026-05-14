---
id: ADR-0030
title: Conventions guide consolidation
status: accepted
date: "2026-04-27"

enforces: []
applies_to:
  - "docs/chassis/guides/conventions.md"
  - "docs/chassis/CONVENTIONS.md"
  - "docs/chassis/guides/positioning.md"
  - "config/chassis.doc-graph.yaml"
supersedes: []
tags:
  - chassis
  - documentation
  - conventions
  - guides
---

# ADR-0030: Conventions guide consolidation

## Context

The chassis guides tree carried eleven `conventions-*.md` files
(`accessibility`, `api-design`, `error-handling`, `file-structure`, `i18n`,
`loading-states`, `naming`, `performance`, `security`, `state-management`,
`testing`). Each file averaged ~50 lines and was visited rarely; cross-doc
discovery was poor because no index linked them as a coherent set. The
`docs/chassis/CONVENTIONS.md` projection (generated from `AGENTS.md`)
already lists each topic in a single page, but the detailed reference was
fragmented across eleven files with no shared navigation.

The split also created governance overhead: eleven doc-graph nodes for
content that could be one node, eleven separate inbound-link surfaces, and
eleven separate baseline entries when path tokens broke.

## Decision

The eleven `conventions-*.md` files are merged into a single
`docs/chassis/guides/conventions.md` with one H2 section per topic. Section
slugs are GitHub-style and stable: `#accessibility`, `#api-design`,
`#error-handling`, `#file-structure`, `#i18n`, `#loading-states`,
`#naming`, `#performance`, `#security`, `#state-management`, `#testing`.
Inbound references in `docs/chassis/CONVENTIONS.md` and
`docs/chassis/guides/positioning.md` are rewritten to use
`conventions.md#section-anchor` form. The doc-graph drops the eleven
per-topic nodes and adds one `conventions.md` node parented to ADR-0007
(repo-boundary, the same parent the predecessors used).

A "Redirects (legacy filenames)" section at the bottom of
`conventions.md` enumerates the old filenames mapped to the new anchors so
external consumer documentation that deep-linked to a per-topic file can
follow the trail.

## Consequences

- One canonical conventions reference replaces eleven scattered files.
  Discovery via `docs/chassis/guides/README.md` and the new
  `conventions.md` table of contents is unified.
- Inbound links shift to the `path#anchor` form, motivating Gate E
  (doc-anchors) which validates that anchors resolve to real headings.
- Doc-graph node count drops by ten on the conventions surface.
- External consumer docs that linked to `conventions-*.md` directly need
  a one-time link rewrite. The Redirects section in `conventions.md`
  documents the mapping.

## Alternatives considered

- **Keep the eleven separate files but author an index.** Rejected: the
  files are short and read together. Eleven separate files inflated the
  doc-graph and added eleven inbound-link surfaces to maintain.
- **Move the per-topic content into `docs/chassis/CONVENTIONS.md`.**
  Rejected: that file is generated from `AGENTS.md`, so any per-topic
  detail living there would be auto-overwritten or pulled into `AGENTS.md`
  itself, polluting the canonical agent surface with reference detail.

## References

- ADR-0007 (repo boundary): the parent ADR for the conventions guide and
  its predecessors.
- ADR-0027 (doc anchor stability): the gate that validates the new
  `path#anchor` references the consolidation introduces.
- `docs/chassis/CONVENTIONS.md`: generated projection; consumes the new
  anchors as `conventions.md#section`.
- `docs/chassis/guides/positioning.md`: rewritten inbound link.

## Status

Accepted. Consolidation landed in Tranche 2 of the chassis self-governance
hardening plan.
