#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

write_bad_exemption() {
  local repo="$1"
  local path="$2"
  local owner="$3"

  cat >"$repo/.chassis/exemptions.yaml" <<YAML
version: 2
entries:
  - id: EX-2099-0001
    rule_id: CH-DRIFT-IMPL-MISSING
    reason: "Demo waiver intentionally omits the required CODEOWNERS acknowledgment."
    owner: demo@chassis.invalid
    created_at: "2099-01-01"
    expires_at: "2099-03-01"
    path:
      - "$path"
    codeowner_acknowledgments:
      - "$owner"
YAML
}

rust_repo="$(prepare_repo "$DEMO_DIR/fixtures/rust-library" rust-exemption-fail)"
ts_repo="$(prepare_repo "$DEMO_DIR/fixtures/typescript-package" ts-exemption-fail)"

write_bad_exemption "$rust_repo" "crates/demo-rust/src/lib.rs" "@not-demo-rust"
write_bad_exemption "$ts_repo" "packages/demo-ts/src/index.ts" "@not-demo-ts"

run_gate "rust-library/fail-exemption" "$rust_repo" || true
run_gate "typescript-package/fail-exemption" "$ts_repo" || true
