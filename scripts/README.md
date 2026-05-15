# Repo scripts

## `build-drift-fixture-repo.sh`

Rebuilds `fixtures/drift-repo/drift_fixture.git` (bare repository, deterministic commits and author dates) used by `chassis_core::drift::git` unit tests.

```bash
./scripts/build-drift-fixture-repo.sh
```

## `docs-lint.sh`

Scans the active documentation set (`README.md`, `CLAUDE.md`, `CONTRACT.yaml`, `docs/WAVE-PLAN.md`, `docs/ASSURANCE-LADDER.md`, `packages/chassis-types/README.md`) for forbidden phrases that describe surfaces the repo no longer has — e.g. "no CLI binary yet" (a CLI ships), "MCP server" (the sidecar is JSON-RPC 2.0, not MCP), or paths from the previous monolith (`scripts/chassis`, `codegen/ts-types`, bare `chassis-schemas`).

ADRs are not scanned: they are immutable records of decisions made at points in time, not statements of current state.

```bash
bash scripts/docs-lint.sh
```

A line can opt out of a rule by including `chassis-lint-allow:<label>` (e.g., `chassis-lint-allow:mcp-server`) — used sparingly when an active doc must quote a forbidden phrase. The script also runs as the first step of `verify-foundation.sh`.

## `verify-foundation.sh`

Runs the canonical local verification gates before pushing:

- Docs lint: `bash scripts/docs-lint.sh`
- Rust: `cargo fmt --check`, `cargo clippy` (warnings denied), `cargo check`, `cargo test`
- TypeScript package: `npm ci`, `npm run build`, schema-metadata verification, fingerprint verification, `npm test`
- Reference Python tree: best-effort `compileall` (quarantined; failures ignored)

Exports `CHASSIS_REPO_ROOT` so fingerprint scripts resolve the workspace root reliably.

Usage from the repository root:

```bash
chmod +x scripts/verify-foundation.sh   # once, if needed
./scripts/verify-foundation.sh
```
