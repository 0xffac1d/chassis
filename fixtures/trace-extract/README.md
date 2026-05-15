# trace-extract fixtures

Source-file fixtures driving the claim-annotation scanner tests
(`crates/chassis-core/tests/trace_extract_fixtures.rs`). Each subdirectory holds
one or more `.rs`, `.ts`, or `.tsx` source files plus an `expected.json` file
describing the sites and diagnostics the scanner must produce.

Authoritative annotation grammar: [ADR-0023](../../docs/adr/ADR-0023-claim-annotations.md)
— `// @claim <id>` on its own line immediately before the backed item, for both
Rust and TypeScript. The rejected pre-ADR-0023 JSDoc form (`/** @claim <id> */`)
must surface `CH-TRACE-MALFORMED-CLAIM` (see `ts-jsdoc-rejected/`) so claims
written that way fail loudly rather than silently disappearing from the trace
graph.

## Layout

| Directory                | Language   | What it exercises                                   |
|--------------------------|------------|------------------------------------------------------|
| `rust-line-comments/`    | Rust       | accepted `// @claim` on an impl site                |
| `rust-test-site/`        | Rust       | `// @claim` on a `#[test]` function (test site)     |
| `rust-malformed-id/`     | Rust       | claim id violates STABLE-IDS grammar                |
| `rust-duplicate-id/`     | Rust       | same id repeated at the same site (info diagnostic) |
| `ts-line-comments/`      | TypeScript | accepted `// @claim` on an `export function`        |
| `ts-test-site/`          | TypeScript | `// @claim` on a Jest/Vitest `test(...)` call       |
| `ts-jsdoc-rejected/`     | TypeScript | rejected `/** @claim ... */` JSDoc form             |
| `ts-malformed-id/`       | TypeScript | claim id violates STABLE-IDS grammar                |
| `ts-duplicate-id/`       | TypeScript | same id repeated at the same site (info diagnostic) |

Every fixture is a *unit* of behaviour — adding more rows here belongs in new
sibling directories with their own `expected.json` rather than packing multiple
cases into one file.
