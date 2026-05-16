---
id: ADR-0029
title: "Spec Kit markdown bridge: yaml-meta fenced block"
status: accepted
date: "2026-05-15"
enforces:
  - rule: CH-SPEC-MARKDOWN-NO-FENCE
  - rule: CH-SPEC-MARKDOWN-MULTIPLE-FENCES
  - rule: CH-SPEC-MARKDOWN-EMPTY-FENCE
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

## Appendix A — Required `yaml-meta` preset (machine check)

Authors MUST produce:

- **Exactly one** fenced code block with info string `yaml-meta` (no duplicate fences).
- A **non-empty** fence body: valid YAML conforming to `schemas/spec-index.schema.json`.

Rejections (stable rule ids):

| Condition | `ruleId` |
|-----------|----------|
| No `yaml-meta` fence | `CH-SPEC-MARKDOWN-NO-FENCE` |
| More than one `yaml-meta` fence | `CH-SPEC-MARKDOWN-MULTIPLE-FENCES` |
| Fence body empty / whitespace-only | `CH-SPEC-MARKDOWN-EMPTY-FENCE` |

## Consequences

- Spec authors can keep human context in Markdown while preserving deterministic, schema-checked `spec-index.json`.
- **Trace id:** `chassis.spec-kit-markdown-bridge` binds the Markdown `yaml-meta` path to this ADR.
