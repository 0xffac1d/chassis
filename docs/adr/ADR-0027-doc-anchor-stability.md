---
id: ADR-0027
title: Doc anchor stability
status: accepted
date: "2026-04-27"

enforces:
  - rule: CHASSIS-DOC-BINDING-ANCHOR-UNRESOLVED
    description: "A markdown link of the form path#anchor resolves the path but the anchor does not match any H2 or H3 heading slug in the target file."
  - rule: CHASSIS-DOC-BINDING-ANCHOR-AMBIGUOUS
    description: "A markdown link of the form path#anchor resolves to a heading slug that appears more than once in the target file."
applies_to:
  - "scripts/chassis/gates/doc_binding.py"
  - "config/chassis/doc-binding.toml"
  - "docs/**/*.md"
  - "AGENTS.md"
  - "README.md"
supersedes: []
tags:
  - chassis
  - doc-binding
  - governance
  - documentation
---

# ADR-0027: Doc anchor stability

## Context

The `doc-binding` gate validates that every path-shaped token in chassis
documentation resolves to an existing file. It does not validate the
anchor portion of `path#anchor` references. As a result, links can decay
silently when a heading is renamed or removed in the target file.

The Tranche-2 conventions consolidation (eleven `conventions-*.md` files
merged into one `conventions.md` with H2 sections per topic) introduces a
large new surface of `conventions.md#section-anchor` references. Without
anchor validation those references can rot the day after the consolidation
ships.

## Decision

`scripts/chassis/gates/doc_binding.py` is extended to detect a trailing
`#fragment` after each path token it resolves. When the fragment is
present, the gate slugifies all `## H2` and `### H3` headings in the
target markdown file (GitHub-style: lowercase, dashes for spaces, strip
punctuation) and asserts the fragment is in the resulting set. Two new
rule IDs are added:

- `CHASSIS-DOC-BINDING-ANCHOR-UNRESOLVED` — fragment is not in the heading
  slug set.
- `CHASSIS-DOC-BINDING-ANCHOR-AMBIGUOUS` — fragment matches more than one
  heading slug in the target file.

`config/chassis/doc-binding.toml` gains an `[anchors]` block with
`enabled = true`, `slug_dialect = "github"`, and `ignore_anchors`
(defaulting to `["top"]` for the conventional `#top` link).

The gate ships warn-only for one minor release with a baseline at
`config/chassis/baselines/doc-anchors.json` capturing the existing broken-
anchor inventory. The next minor release promotes to error.

## Consequences

- Heading renames force authors to update inbound anchor references in the
  same change. Today the stale anchor would silently break.
- The conventions consolidation ships safely; cross-file `conventions.md#x`
  references are validated as part of CI.
- Slug ambiguity is surfaced (two H2 sections with the same slug), which is
  itself a content issue worth fixing.

## Alternatives considered

- **A separate `doc-anchors` gate module.** Rejected: `doc-binding` already
  walks the docs and resolves path tokens. Adding the anchor check inline
  reuses the walker and emits findings under one gate, which is simpler to
  configure and operate.
- **CommonMark-strict slug dialect instead of GitHub.** Rejected: the
  existing chassis docs are rendered on GitHub and the documentation links
  use GitHub slug rules. Aligning the gate with the rendering avoids
  spurious failures.

## References

- ADR-0007 (repo boundary): `doc-binding` rule family this ADR extends.
- `scripts/chassis/gates/doc_binding.py`: the existing gate to extend.
- `config/chassis/doc-binding.toml`: the existing config to extend.
- `config/chassis/baselines/doc-binding.json`: baseline-shape precedent.

## Status

Accepted. Implementation lands as Gate E in the chassis self-governance
hardening plan, after the Tranche-2 conventions consolidation creates the
new anchor surface.
