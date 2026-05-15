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

## `check-archive-hygiene.sh`

Validates release source trees or archives (`.tar.gz`, `.tgz`, `.tar`, `.zip`, or a directory):

- Forbids root `.git/`, Cargo/npm/Python cache dirs, bytecode, and stale absolute paths (`/mnt/C/…/chassis`) outside `reference/`.
- Asserts required paths present for parity with docs-lint and CI (`CLAUDE.md`, `.gitignore`, `.github/workflows/ci.yml`, core scripts and docs, and `reference/`).

CI uses this after `./scripts/build-source-archive.sh`; run locally:

```bash
bash scripts/check-archive-hygiene.sh path/to/source.tar.gz
```

## `build-source-archive.sh`

Produces `dist/chassis-source-<sha>.tar.gz` from `git archive` using `.gitattributes` export rules, then runs `check-archive-hygiene.sh` on the tarball.

```bash
./scripts/build-source-archive.sh [dist/out.tar.gz] [git-ref]
```

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

## `policy-gate.sh`

Runs the **OPA policy spine** on top of Chassis exports:

1. `opa test policy/` — Rego unit tests (`policy/chassis_release_test.rego`).
2. `cargo run -p chassis-cli -- export --format opa --repo <repo> --json` → `policy-input.json` (or the directory from `POLICY_GATE_OUT_DIR`).
3. `opa eval data.chassis.release.result` — writes `policy-result.json` with `allow` and sorted `deny_reasons`. Exits **2** if `allow` is not true.

Requires the **OPA CLI** on `PATH`. CI installs it via `open-policy-agent/setup-opa`. Environment variables:

- `POLICY_GATE_OUT_DIR` — output directory for inputs, results, and `policy-gate.log` (default `./dist/policy-gate` under the repo root when not set).
- `POLICY_GATE_LOG` — optional explicit log path.

```bash
./scripts/policy-gate.sh /path/to/repo
```
