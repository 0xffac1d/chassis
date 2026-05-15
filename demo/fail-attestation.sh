#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

# Sign a release-gate attestation under --ephemeral-key (non-release-grade,
# fine for demos), tamper with the resulting DSSE envelope, then run
# `chassis attest verify`. The verifier must reject the modified envelope —
# proof that a release verdict cannot be silently rewritten after signing.
run_attest_tamper() {
  local label="$1"
  local repo="$2"
  local dsse="$repo/release-gate.dsse"
  local pub="$dsse.ephemeral.pub"
  local gate_out="$repo/.chassis/release-gate.attest-sign.out"
  local verify_out="$repo/.chassis/release-gate.attest-verify.out"
  local sign_code
  local verify_code

  set +e
  chassis --repo "$repo" --json release-gate \
    --fail-on-drift --attest --ephemeral-key \
    --out "$repo/.chassis/release-gate.json" \
    --attest-out "$dsse" \
    >"$gate_out" 2>&1
  sign_code=$?
  set -e

  if [[ "$sign_code" -ne 0 ]]; then
    echo "$label: exit=$sign_code stage=sign-prep ok=false"
    echo "  rules=attestation-precondition-failed"
    return 0
  fi

  python3 - "$dsse" <<'PY'
import base64
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
env = json.loads(path.read_text(encoding="utf-8"))
# Replace the signed payload with a forged Statement. The DSSE PAE bytes
# include both the payload and its type, so this guarantees signature
# verification fails — there is no way to rewrite the verdict after the fact.
env["payload"] = base64.standard_b64encode(b'{"tampered":true}').decode("ascii")
path.write_text(json.dumps(env, indent=2), encoding="utf-8")
PY

  set +e
  chassis --repo "$repo" --json attest verify "$dsse" \
    --public-key "$pub" \
    >"$verify_out" 2>&1
  verify_code=$?
  set -e

  python3 - "$label" "$verify_code" "$verify_out" <<'PY'
import json
import sys

label, code, path = sys.argv[1], int(sys.argv[2]), sys.argv[3]
raw = open(path, encoding="utf-8").read().splitlines()
payload = None
for line in reversed(raw):
    line = line.strip()
    if line.startswith("{"):
        payload = json.loads(line)
        break

stage = "attestation"
if payload is None:
    print(f"{label}: exit={code} no-json-output stage={stage}")
    sys.exit(0)

err = payload.get("error") or {}
rule = err.get("code") or payload.get("ruleId") or "attestation-tamper-detected"
print(f"{label}: exit={code} ok=false stage={stage}")
print(f"  rules={rule}")
PY
}

rust_repo="$(prepare_repo "$DEMO_DIR/fixtures/rust-library" rust-attestation-fail)"
ts_repo="$(prepare_repo "$DEMO_DIR/fixtures/typescript-package" ts-attestation-fail)"

run_attest_tamper "rust-library/fail-attestation" "$rust_repo" || true
run_attest_tamper "typescript-package/fail-attestation" "$ts_repo" || true
