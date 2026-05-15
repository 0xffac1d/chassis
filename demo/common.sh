#!/usr/bin/env bash
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/.." && pwd)"
WORK_ROOT="${TMPDIR:-/tmp}/chassis-demo-${USER:-user}-$$"

cleanup_demo_workdir() {
  rm -rf "$WORK_ROOT"
}

trap cleanup_demo_workdir EXIT

chassis() {
  if [[ -n "${CHASSIS_BIN:-}" ]]; then
    "$CHASSIS_BIN" "$@"
  else
    cargo run --quiet --manifest-path "$REPO_ROOT/Cargo.toml" -p chassis-cli -- "$@"
  fi
}

prepare_repo() {
  local fixture="$1"
  local name="$2"
  local dest="$WORK_ROOT/$name"

  mkdir -p "$WORK_ROOT"
  cp -R "$fixture" "$dest"

  (
    cd "$dest"
    git init --quiet
    git add .
    GIT_AUTHOR_NAME="Chassis Demo" \
      GIT_AUTHOR_EMAIL="demo@chassis.invalid" \
      GIT_AUTHOR_DATE="2026-05-01T00:00:00Z" \
      GIT_COMMITTER_NAME="Chassis Demo" \
      GIT_COMMITTER_EMAIL="demo@chassis.invalid" \
      GIT_COMMITTER_DATE="2026-05-01T00:00:00Z" \
      git -c user.name="Chassis Demo" -c user.email="demo@chassis.invalid" commit --quiet -m "initial demo state"
  )

  printf '%s\n' "$dest"
}

run_gate() {
  local label="$1"
  local repo="$2"
  local output="$repo/.chassis/release-gate.out"
  local artifact="$repo/.chassis/release-gate.json"
  local code

  set +e
  chassis --repo "$repo" --json release-gate --fail-on-drift --out "$artifact" >"$output" 2>&1
  code=$?
  set -e

  python3 - "$label" "$code" "$output" <<'PY'
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

if payload is None:
    print(f"{label}: exit={code} no-json-output")
    for line in raw[-5:]:
        print(f"  {line}")
    sys.exit(0)

ok = str(payload.get("ok")).lower()
contract = payload.get("contract_validation") or {}
trace = payload.get("trace_summary") or {}
drift = payload.get("drift_summary") or {}
exemptions = payload.get("exemption_summary") or {}

if payload.get("stage"):
    stage = payload["stage"]
elif contract.get("invalid", 0) > 0:
    stage = "contract"
elif exemptions.get("errors", 0) > 0:
    stage = "exemptions"
elif drift.get("unsuppressed_blocking", 0) > 0 or trace.get("missing_impl", 0) > 0:
    stage = "drift"
else:
    stage = "release_gate"

print(f"{label}: exit={code} ok={ok} stage={stage}")

if payload.get("ok"):
    print(
        f"  contracts={contract.get('checked', payload.get('contracts', 0))} "
        f"claims={trace.get('claims', 0)} "
        f"orphans={trace.get('orphan_sites', trace.get('orphanSites', 0))} "
        f"drift_missing={drift.get('missing', 0)} "
        f"exemption_errors={exemptions.get('errors', 0)}"
    )
else:
    detail = payload.get("detail", {})
    rule_ids = detail.get("ruleIds")
    if rule_ids:
        print(f"  rules={','.join(sorted(set(rule_ids)))}")
    elif contract.get("errors"):
        rules = sorted({err.get("code", "contract") for err in contract["errors"]})
        print(f"  rules={','.join(rules)}")
    elif exemptions.get("errors", 0) > 0:
        print("  rules=CH-EXEMPT-VERIFY")
    elif drift.get("unsuppressed_blocking", 0) > 0 or drift.get("missing", 0) > 0:
        print("  rules=CH-DRIFT-DETECTED")
    else:
        print(f"  rule={payload.get('ruleId')}")
PY

  return "$code"
}
