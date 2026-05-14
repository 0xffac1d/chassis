# brownfield-messy

Intentionally incomplete mixed-language tree, preserved as a forward-pointing reference target. It models the kind of repo the planned `chassis doctor` / `chassis bootstrap` surface needs to handle gracefully — partial inputs, no pre-existing `.chassis/` state, multiple adapters firing on the same tree.

What's messy about it:

* `Cargo.toml` + Rust source **and** a `package.json` with no `main` — both language adapters should fire.
* `messy_extras/` holds loose Python utilities not reachable from `src/`.
* No tests, no subtree README, no existing `.chassis/` state.

There is no `chassis` CLI here yet, so this fixture is not exercised by any current test — it documents intent for the planned advisory / standard / strict bootstrap modes.
