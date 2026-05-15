#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

break_contract() {
  local repo="$1"
  python3 - "$repo/CONTRACT.yaml" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
path.write_text(text.replace('version: "0.1.0"', 'version: "not-semver"', 1), encoding="utf-8")
PY
}

rust_repo="$(prepare_repo "$DEMO_DIR/fixtures/rust-library" rust-contract-fail)"
ts_repo="$(prepare_repo "$DEMO_DIR/fixtures/typescript-package" ts-contract-fail)"

break_contract "$rust_repo"
break_contract "$ts_repo"

run_gate "rust-library/fail-contract" "$rust_repo" || true
run_gate "typescript-package/fail-contract" "$ts_repo" || true
