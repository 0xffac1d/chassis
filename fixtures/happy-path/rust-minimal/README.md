# rust-minimal

A minimal Cargo crate with a hand-authored `CONTRACT.yaml`. The contract validates against `schemas/contract.schema.json` and is exercised by the `canonical_validator_accepts_repo_contract_yaml` test in `crates/chassis-core/src/validators.rs`.

Use this fixture as the canonical happy-path reference for what a tiny, valid Rust-side contract looks like. Future tooling (planned `chassis validate` / `chassis trace` / `chassis doctor`) is expected to bootstrap and validate cleanly against trees of this shape.
