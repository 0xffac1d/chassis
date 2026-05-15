# Repo scripts

## `verify-foundation.sh`

Runs the canonical local verification gates before pushing:

- Rust: `cargo fmt --check`, `cargo clippy` (warnings denied), `cargo check`, `cargo test`
- TypeScript package: `npm ci`, `npm run build`, fingerprint verification, `npm test`
- Reference Python tree: best-effort `compileall` (quarantined; failures ignored)

Exports `CHASSIS_REPO_ROOT` so fingerprint scripts resolve the workspace root reliably.

Usage from the repository root:

```bash
chmod +x scripts/verify-foundation.sh   # once, if needed
./scripts/verify-foundation.sh
```
