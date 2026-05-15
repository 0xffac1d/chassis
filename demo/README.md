# Chassis Demo

This demo shows why Chassis exists: typecheck and lint can prove code is well-formed, but they do not prove code still honors its declared product/spec contract.

The fixtures are two tiny repos:

- `fixtures/rust-library`: a Rust library under `crates/demo-rust`.
- `fixtures/typescript-package`: a TypeScript package under `packages/demo-ts`.

Each fixture has a valid `CONTRACT.yaml`, `// @claim ...` implementation annotations, linked test annotations, `CODEOWNERS`, and an exemption registry. The scripts copy those fixtures into disposable git repos, run the same release gate, and then summarize deterministic results.

Run from the repository root:

```sh
demo/pass.sh
demo/fail-contract.sh
demo/fail-drift.sh
demo/fail-exemption.sh
demo/fail-attestation.sh
```

What each script proves:

- `pass.sh`: both repos have valid contracts, implementation claims, linked tests, no drift, and clean exemptions.
- `fail-contract.sh`: the package still looks like ordinary source code, but the declared contract version is invalid, so the release gate fails at `contract`.
- `fail-drift.sh`: tests and source still typecheck, but an implementation claim annotation was removed. Chassis detects that a contract claim has no backing implementation site and fails at `drift`.
- `fail-exemption.sh`: source and tests are unchanged, but a waiver omits the required CODEOWNERS acknowledgment. Chassis fails at `exemptions`.
- `fail-attestation.sh`: a release-gate attestation is signed and then its payload is tampered. `chassis attest verify` rejects the modified envelope, proving the release verdict cannot be rewritten after signing.

Set `CHASSIS_BIN=/path/to/chassis` to use an already-built binary; otherwise the scripts run `cargo run -p chassis-cli`.
