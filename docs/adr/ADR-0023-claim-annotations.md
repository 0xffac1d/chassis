---
id: ADR-0023
title: "Source @claim annotations (Rust + TypeScript)"
status: accepted
date: "2026-05-14"
supersedes: []
enforces:
  - rule: CH-TRACE-MALFORMED-CLAIM
    description: "The @claim line does not match the grammar or claim-id is not per STABLE-IDS."
  - rule: CH-TRACE-CLAIM-NOT-IN-CONTRACT
    description: "A well-formed @claim id has no matching invariant/edge_case on any CONTRACT.yaml (warning)."
  - rule: CH-TRACE-DUPLICATE-CLAIM-AT-SITE
    description: "More than one @claim for the same id immediately above the same site (info)."
---

# ADR-0023 — `@claim` annotation grammar

## Context

Trace graph construction must bind contract claim IDs to implementation and test sites without proc-macros or full language servers.

## Decision

- **Syntax (Rust and TypeScript, identical):** a line that is exactly `// @claim <claim-id>` (trimmed), on its own line, immediately *before* the backed item (`fn`, `#[test]`, `impl` block start, or exported `const`). Multiple consecutive `@claim` lines are allowed; order is preserved.
- **Extraction:** line-oriented scan of raw source files. No AST and no proc-macros. Valid lines are those matching the implementation’s regex; malformed lines use `CH-TRACE-MALFORMED-CLAIM`.
- **Claim IDs:** grammar and namespaces are authoritative in `docs/STABLE-IDS.md` (claim ID section).
- **Diagnostics:** `CH-TRACE-CLAIM-NOT-IN-CONTRACT` (warning) when the id is syntactically valid but absent from all contracts; `CH-TRACE-DUPLICATE-CLAIM-AT-SITE` (info) when the same id is repeated for the same site.

## Consequences

Extractors in `chassis-core::trace::extract` must stay line-based to match this ADR; richer static analysis is out of scope.
