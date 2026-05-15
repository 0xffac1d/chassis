---
id: ADR-0023
title: "Source @claim annotations (Rust + TypeScript)"
status: accepted
date: "2026-05-15"
supersedes:
  - ADR-0005
enforces:
  - rule: CH-TRACE-MALFORMED-CLAIM
    description: "The @claim line does not match the grammar or claim-id is not per STABLE-IDS. The rejected TypeScript JSDoc form (`/** @claim ... */`) is reported under this rule so the previously divergent grammar from ADR-0005 cannot fail silently."
  - rule: CH-TRACE-CLAIM-NOT-IN-CONTRACT
    description: "A well-formed @claim id has no matching invariant/edge_case on any CONTRACT.yaml (warning)."
  - rule: CH-TRACE-DUPLICATE-CLAIM-AT-SITE
    description: "More than one @claim for the same id immediately above the same site (info)."
applies_to:
  - "crates/**/*.rs"
  - "packages/**/*.ts"
  - "packages/**/*.tsx"
tags:
  - foundation
  - trace
---

# ADR-0023 — `@claim` annotation grammar

## Context

Trace graph construction must bind contract claim IDs to implementation and test sites without proc-macros or full language servers. ADR-0005 originally permitted **two** syntaxes (Rust line comments + TypeScript JSDoc). The line-oriented scanner in `crates/chassis-core/src/trace/extract/` only accepted the line-comment form, so TypeScript claims written in JSDoc form were silently dropped from the trace graph. This ADR supersedes ADR-0005 with a single grammar covering both languages and an explicit rejection diagnostic for the legacy JSDoc form so the mismatch cannot recur.

## Decision

- **Syntax (Rust and TypeScript, identical):** a line that is exactly `// @claim <claim-id>` (trimmed), on its own line, immediately *before* the backed item (`fn`, `#[test]`, `impl` block start, exported `const`, TypeScript `function`/`class`/`export const`). Multiple consecutive `@claim` lines are allowed; order is preserved.
- **Rejected TypeScript form:** the JSDoc tag previously specified by ADR-0005 (`/** @claim <id> */`, or any other block-comment form containing `@claim`) is **not** accepted. The scanner detects it and emits `CH-TRACE-MALFORMED-CLAIM` with a message that names the rejected form, so a claim written this way fails loudly instead of silently disappearing from the trace graph.
- **Extraction:** line-oriented scan of raw source files. No AST and no proc-macros. Valid lines are those matching the implementation’s regex; malformed lines use `CH-TRACE-MALFORMED-CLAIM`.
- **Claim IDs:** grammar and namespaces are authoritative in `docs/STABLE-IDS.md` (claim ID section).
- **Diagnostics:** `CH-TRACE-CLAIM-NOT-IN-CONTRACT` (warning) when the id is syntactically valid but absent from all contracts; `CH-TRACE-DUPLICATE-CLAIM-AT-SITE` (info) when the same id is repeated for the same site.
- **Tests:** test functions use the same `// @claim` form (`#[test]`, `#[tokio::test]`, `#[rstest]`, Jest/Vitest `test(...)`/`describe(...)`). The scanner classifies the site as `SiteKind::Test` and routes it into `ClaimNode.test_sites`.

## Consequences

- Extractors in `chassis-core::trace::extract` stay line-based to match this ADR; richer static analysis is out of scope.
- A TypeScript file using the rejected JSDoc form fails the trace check with `CH-TRACE-MALFORMED-CLAIM` rather than producing an orphaned-claim contract.
- ADR-0005 is preserved with `status: superseded` and `superseded_by: ADR-0023` for traceability.
