# Project history

A chronological record of how the tree got into its current shape. This is reference material, not a backlog; for current state see `CLAUDE.md`, and for next work see ADR-0001 and `CLAUDE.md`'s "Immediate next work".

## Phase 1 — Salvage extraction (commit `335ec60`)

Extracted ~15% of a prior, larger codebase ("Chassis"). The audit determined the remaining ~85% was either scope-bloat or competed unfavorably with GitHub Spec Kit; see `docs/adr/ADR-0001-project-scope-and-positioning.md` for the resulting scope decision.

Imported as canonical:
- 8 JSON Schemas → `schemas/`
- `chassis-core` source (10 .rs files) → `crates/chassis-core/`
- `@chassis/types` codegen output → `packages/chassis-types/`
- Fixtures (happy-path, adversarial, brownfield-messy) → `fixtures/`

Imported as reference only:
- Original Python CLI (11 files) → `reference/python-cli/`
- Extended schemas for component/api/data/service/event/state → `reference/schemas-extended/`
- 32 historical ADRs → `reference/adrs-original/`
- Release-gate artifact example → `reference/artifacts/`

Verification snapshot at end of extraction: 10 Rust source files, 8 canonical schemas, 13 reference schemas, 11 Python reference files, 6 fixtures.

## Phase 2 — Compile-blocker fixup (commit `6bd3cdf`)

The salvage was structurally sound but did not compile. Two errors blocked `cargo check`:

1. `chassis-core/src/contract.rs:31` referenced `crate::metadata::debt_item::DebtItem` — `metadata` module never existed in the salvaged source tree.
2. `chassis-core/src/validators.rs:49` implemented `chassis_runtime_api::Validator` for `CanonicalMetadataContractValidator` — the `chassis-runtime-api` crate was deliberately dropped during salvage.

Both were patched: the `DebtItem` reference was removed, the validator was rewritten to use a local `Validator` trait in `chassis-core::validators`. Orphaned source files were also removed.

Other fixup work in the same commit:
- Authored `fixtures/happy-path/rust-minimal/CONTRACT.yaml` and `fixtures/happy-path/typescript-vite/CONTRACT.yaml` (the salvaged fixtures had only fixture descriptors, no contracts). Both validate against `schemas/contract.schema.json`.
- Corrected the test path in `validators.rs` (`../../../` → `../../`).
- Repaired `.gitignore` to keep `packages/chassis-types/dist/` tracked while excluding other `dist/` outputs.
- Relocated original ADRs from `docs/adr/` to `reference/adrs-original/` (they describe the prior project, not this one).
- Authored ADR-0001 establishing scope and positioning.
- Rebuilt `chassis-types` against the canonical 8-schema set: 8 schemas → 9 `.d.ts` files.

End-state verification:
- `cargo check`: OK (Rust 1.95.0)
- `cargo test`: 4 passed, 0 failed
- All 8 schemas parse as valid JSON
- No external `$ref`s in `contract.schema.json`
- Happy-path fixtures validate against `contract.schema.json`
- No residual `chassis_runtime_api` or cross-module dangling references
- All `include_str!` paths resolve

## Deferred to forward work

The `illegal-layout` adversarial fixture has no verifier — there is no layout validator yet. The fixture exists as a forward-pointing reference for the planned `chassis doctor`-style surface.
