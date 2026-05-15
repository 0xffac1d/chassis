#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export CHASSIS_REPO_ROOT="$ROOT"

echo ">> cargo fmt --check"
cargo fmt --check --all

echo ">> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo ">> cargo check"
cargo check --workspace

echo ">> cargo test"
cargo test --workspace

echo ">> npm ci (chassis-types)"
npm ci --prefix packages/chassis-types

echo ">> npm run build (chassis-types)"
npm run build --prefix packages/chassis-types

echo ">> verify-fingerprint.mjs"
node packages/chassis-types/scripts/verify-fingerprint.mjs

echo ">> npm test (chassis-types)"
npm test --prefix packages/chassis-types

echo ">> python compileall (reference, best-effort)"
python3 -m compileall -q reference/python-cli || true

echo "verify-foundation: OK"
