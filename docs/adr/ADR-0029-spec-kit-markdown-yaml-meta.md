---
id: ADR-0029
title: "Spec Kit markdown bridge: yaml-meta fenced block"
status: accepted
date: "2026-05-15"
enforces: []
applies_to:
  - crates/chassis-core/src/spec_index_markdown.rs
tags:
  - spec-kit
  - spec-index
---

# ADR-0029 — Machine-readable YAML inside Markdown

## Decision

- The supported Chassis markdown bundle is any CommonMark file containing **exactly one** fenced code block labelled `yaml-meta`.
- The fence body MUST be valid YAML for [`schemas/spec-index.schema.json`](../../schemas/spec-index.schema.json) (same shape as `.chassis/spec-index-source.yaml`).
- Prose outside the fence is documentation-only; `chassis spec-index from-spec-kit` ignores it.
- `pulldown-cmark` parses the document; no free-form requirement extraction from narrative headings in CI (that remains a future extension).

## Consequences

- Spec authors can keep human context in Markdown while preserving deterministic, schema-checked `spec-index.json`.
- **Trace id:** `chassis.spec-kit-markdown-bridge` binds the Markdown `yaml-meta` path to this ADR.
