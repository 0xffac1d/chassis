---
id: ADR-0005
title: "Claim annotation format — Rust line comments and TypeScript JSDoc tags"
status: accepted
date: "2026-05-14"
enforces:
  - rule: CLAIM-ANNOTATION-FORMAT-RUST
    description: "Rust sources MUST use `// @claim <claim_id>` immediately associated items."
  - rule: CLAIM-ANNOTATION-FORMAT-TS
    description: "TypeScript sources MUST use `/** @claim <claim_id> */` attached to associated declarations."
  - rule: CLAIM-ANNOTATION-PLACEMENT
    description: "Annotations MUST immediately precede their bound AST item (module decl, class, function, or method)."
applies_to:
  - "crates/**/*.rs"
  - "packages/**/*.ts"
  - "packages/**/*.tsx"
tags:
  - foundation
  - trace
---

## Context

The trace graph wave needs deterministic extraction of `claim_id` bindings from Rust + TypeScript sources **without** proc-macros (the prior `chassis-capability-derive` crate was deliberately dropped per ADR-0001). Decorators require experimental TS configuration and often imply runtime emission — unacceptable for static scanning.

## Decision

### Rust

Use a **line comment** immediately above the bound item:

```rust
// @claim bucket.never-exceeds-capacity
pub fn refill(tokens: u32) -> Result<(), Error> { ... }
```

Attributes such as `#[doc("@claim ...")]` are **not** used — rustdoc is for humans, not scanners. A proc-macro attribute (`#[chassis::claim]`) remains **out of scope** until/unless a dedicated ADR reintroduces proc-macros.

### TypeScript

Use a **JSDoc tag** attached to the declaration:

```ts
/** @claim api.allow-returns-deterministic-decision */
export function allow(...): Decision { ... }
```

Decorators (`@Claim(...)`) are **forbidden** for annotations because they require decorator semantics and transpiler support beyond static parsing.

### Placement

The annotation MUST appear **immediately before** its bound syntactic item (module-level `fn`, `struct`, `enum`, `impl` method, TS `function`, `class`, or exported `const` arrow). Scanning tools MAY reject ambiguous spacing (blank lines with intervening comments are allowed if they remain contiguous documentation blocks in TS).

### Multiple claims

Use **one annotation per line/tag** (repeat the comment/JSDoc lines). Comma-separated bundling is **not** valid — simplifies scanners and diffs.

### Tests

Tests use the **same** formats (`// @claim` in Rust test modules, `/** @claim */` above test functions). Divergent markers would duplicate scanner logic without benefit.

## Consequences

- Trace tooling can rely on lexer/parser-adjacent scans without executing rustc or `tsc`.
- IDE folding treats annotations as standard comments/docstrings.

## Relationship to predecessor

No direct predecessor existed for this cross-language annotation contract; this ADR codifies new discipline aligned with the dropped proc-macro path.
