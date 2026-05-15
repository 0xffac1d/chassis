#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

remove_impl_claims() {
  local repo="$1"
  python3 - "$repo" <<'PY'
import pathlib
import sys

repo = pathlib.Path(sys.argv[1])

rust = repo / "crates/demo-rust/src/lib.rs"
if rust.exists():
    text = rust.read_text(encoding="utf-8")
    text = text.replace("// @claim demo.rust.greeting\n", "", 1)
    rust.write_text(text, encoding="utf-8")

ts = repo / "packages/demo-ts/src/index.ts"
if ts.exists():
    text = ts.read_text(encoding="utf-8")
    text = text.replace("// @claim demo.ts.greeting\n", "", 1)
    ts.write_text(text, encoding="utf-8")
PY
}

rust_repo="$(prepare_repo "$DEMO_DIR/fixtures/rust-library" rust-drift-fail)"
ts_repo="$(prepare_repo "$DEMO_DIR/fixtures/typescript-package" ts-drift-fail)"

remove_impl_claims "$rust_repo"
remove_impl_claims "$ts_repo"

run_gate "rust-library/fail-drift" "$rust_repo" || true
run_gate "typescript-package/fail-drift" "$ts_repo" || true
