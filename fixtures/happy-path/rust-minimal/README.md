# rust-minimal

Simple Cargo crate used by `chassis/tests/consumer-fixtures/run-matrix.sh` to prove
Chassis bootstraps cleanly against a repo with nothing but `Cargo.toml` + `src/`.

No pre-existing Chassis files — the matrix writes everything Chassis produces under
`$CHASSIS_FIXTURE_WORK/rust-minimal/` at run time.
