#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

rust_repo="$(prepare_repo "$DEMO_DIR/fixtures/rust-library" rust-pass)"
ts_repo="$(prepare_repo "$DEMO_DIR/fixtures/typescript-package" ts-pass)"

run_gate "rust-library/pass" "$rust_repo"
run_gate "typescript-package/pass" "$ts_repo"
