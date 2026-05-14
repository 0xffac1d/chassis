Fix 1: validators.rs patched
Fix 2: contract.rs + contract.schema.json patched
Fix 3: orphans removed
Fix 4: ADRs relocated to reference/adrs-original/
Fix 5: happy-path CONTRACT.yaml fixtures authored and validated
Fix 6: SKIPPED (SRC unset or typescript-minimal not at $SRC/tests/consumer-fixtures/typescript-minimal)
Fix 7: .gitignore repaired
Fix 8: CLAUDE.md tidied
OK: chassis-types rebuilt — 8 schemas, 9 .d.ts files
Fix 9: chassis-types rebuilt against current schema set
Fix 10: ADR-0001 authored
Fix 1 supplemental: corrected test fixture path (../../../ -> ../../)
=== Final verification summary ===
1. No residual chassis_runtime_api refs: OK
2. No residual cross-module refs: OK
3. include_str! paths resolve: OK
4. Schemas valid JSON (8/8): OK
5. No external $refs in contract.schema.json: OK
6. happy-path fixtures validate (2/2): OK
7. Orphans removed: OK
8. ADR relocation (1 new in docs/adr, 32 in reference/adrs-original): OK
9. cargo check: OK (Rust 1.95.0)
10. cargo test: OK (4 passed, 0 failed)
FAIL count: 0
