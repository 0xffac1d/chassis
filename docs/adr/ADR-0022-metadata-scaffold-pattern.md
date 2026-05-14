---
id: ADR-0022
title: Metadata scaffold pattern for canonical Chassis files
status: accepted
date: "2026-04-26"
applies_to:
  - "templates/**"
  - "scripts/chassis/scaffold/**"
  - "schemas/scaffold/**"
  - "scripts/chassis/gates/metadata_completeness.py"
  - "docs/chassis/guides/scaffold.md"
enforces:
  - rule: CHASSIS-METADATA-MISSING-FILE
    description: "A chassis-governed crate is missing one of the required metadata files."
  - rule: CHASSIS-METADATA-EMPTY-FILE
    description: "A required metadata file exists but has no useful content."
  - rule: CHASSIS-METADATA-INVALID-CONTRACT
    description: "CONTRACT.yaml exists but is malformed or does not validate against the contract schema."
  - rule: CHASSIS-ADR-INDEX-DRIFT
    description: "The ADR directory and docs/index.json do not describe the same ADR set."
  - rule: CHASSIS-ADR-SUPERSEDES-INCONSISTENT
    description: "An ADR supersedes another ADR without a reciprocal superseded_by link."
  - rule: CHASSIS-CONFIG-ORPHANED
    description: "A root config YAML file is not referenced by the canonical config registry."
  - rule: CHASSIS-CONFIG-DANGLING-REFERENCE
    description: "The config registry references a YAML file that does not exist."
supersedes: []
tags:
  - chassis
  - metadata
  - scaffold
  - templates
---

# ADR-0022: Metadata scaffold pattern

## Context

Metadata authoring is repetitive and error-prone. Hand-rolled
`CONTRACT.yaml`, `CRATE.md`, `DECISIONS.md`, and ADR files drift from
canon through invented keys, missing required fields, and mis-ordered
frontmatter.

Existing examples can be referenced, such as "look at
`crates/chassis-schemas`", but agents and humans both fail to hold the
full shape in working memory. Example-driven authoring is useful as a
review aid, not as the canonical creation mechanism for metadata files.

## Decision

Introduce `chassis scaffold` as the canonical surface for creating
Chassis metadata files.

Templates live under `templates/<kind>/` and are schema-validated before
use. Each scaffold renders files that pass `chassis validate`
immediately. Template manifests declare the files they render, the
variables they accept, and any validators applied to resolved values.

Fields that require human or agent authorship, such as invariant text
and surface descriptions, emit a `placeholder.json` artifact consumed by
the existing placeholder gate. Required placeholders remain visible to
automation until a human or agent resolves them.

## Consequences

- New crates, ADRs, invariants, decisions, and projections are
  scaffolded instead of hand-rolled.
- Drift becomes detectable: if rendered output diverges from the
  template plus variables, `chassis scaffold --update` can detect and
  reconcile the difference.
- Templates become a versioned authority surface. Changes to scaffold
  shape are reviewed as contract changes, not as incidental example
  edits.
- Placeholder output makes intentionally unfinished fields explicit and
  gateable instead of hiding them in prose or TODO comments.

## Alternatives considered

- **Continue copying examples by hand.** Rejected because it is the
  current failure mode: examples are helpful, but they are not a
  machine-checked contract.
- **Document the canonical shapes only in guides.** Rejected because
  prose cannot prevent invented keys or missing required fields.
- **Generate files from code without template manifests.** Rejected
  because it would move authority into Python implementation details
  and make scaffold shape harder to review.

## References

- ADR-0006 (codegen policy): generated and rendered surfaces must have
  clear ownership and drift behavior.
- ADR-0021 (capability marker): stable machine-readable markers should
  be explicit rather than inferred from prose conventions.
- `docs/chassis/guides/scaffold.md`: user guide for `chassis scaffold`
  once the command ships.

## Status

Accepted. The schema contracts for scaffold template manifests and
placeholder files are introduced first; template authoring, gate
integration, and the `chassis scaffold` command land in follow-up work.
