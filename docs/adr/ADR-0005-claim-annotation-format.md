---
id: ADR-0005
title: "Claim annotation format — Rust line comments and TypeScript JSDoc tags"
status: superseded
date: "2026-05-15"
superseded_by: ADR-0023
applies_to:
  - "crates/**/*.rs"
  - "packages/**/*.ts"
  - "packages/**/*.tsx"
tags:
  - foundation
  - trace
  - historical
---

## Status

Superseded by [ADR-0023](ADR-0023-claim-annotations.md) on 2026-05-15.

The original decision admitted **two** syntaxes — `// @claim <id>` for Rust and `/** @claim <id> */` (JSDoc) for TypeScript. The trace-graph scanner (`crates/chassis-core/src/trace/extract/`) implements a single line-oriented grammar (`// @claim <id>`) for both languages. The divergence caused TypeScript `@claim` annotations written in JSDoc form to be silently dropped during trace-graph construction.

ADR-0023 unifies the grammar on the line-comment form for both languages and requires the scanner to surface `CH-TRACE-MALFORMED-CLAIM` on the rejected JSDoc form so the mismatch can no longer fail silently.

## Historical context

The text below is preserved for historical reference. **Do not rely on it.** The authoritative grammar for `@claim` annotations is ADR-0023.

### Rust (still accepted under ADR-0023)

Use a line comment immediately above the bound item:

```rust
// @claim bucket.never-exceeds-capacity
pub fn refill(tokens: u32) -> Result<(), Error> { ... }
```

### TypeScript (no longer accepted — see ADR-0023)

The original decision called for a JSDoc tag attached to the declaration:

```ts
/** @claim api.allow-returns-deterministic-decision */
export function allow(...): Decision { ... }
```

Under ADR-0023 the accepted TypeScript form is the same as the Rust form (`// @claim <id>` on its own line, immediately above the backed declaration).

### Placement / multiplicity / tests

Unchanged from ADR-0023: annotations precede their bound item, one claim per line, tests use the same markers as production code.
