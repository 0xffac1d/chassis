# brownfield-messy

Intentionally incomplete mixed-language tree used by the consumer matrix to
exercise advisory (`--mode metadata-only`), standard (`--mode standard`), and
strict (`--mode strict`) bootstrap behavior side by side.

What's messy about it:

* A Cargo.toml + Rust source *and* a package.json with no `main`, so both
  adapters fire.
* `messy_extras/` holds loose Python utilities not reachable from `src/`.
* No tests, no README at subtree level, no existing `.chassis/` state.

The matrix driver runs `chassis bootstrap … --mode metadata-only` first (expected
to pass even with coverage gaps), then `--mode strict` (expected to surface
coverage-metadata failures that the report can be inspected against).
