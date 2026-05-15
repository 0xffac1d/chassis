#!/usr/bin/env bash
# Fail-closed OPA gate: export Chassis evidence as OPA input, run Rego tests, then evaluate allow.
# Chassis produces facts; OPA (policy/chassis_release.rego) decides release policy.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export CHASSIS_REPO_ROOT="${CHASSIS_REPO_ROOT:-$ROOT}"

REPO="${1:-$ROOT}"
POLICY_FILE="$ROOT/policy/chassis_release.rego"
OUT_DIR="${POLICY_GATE_OUT_DIR:-${2:-$ROOT/dist/policy-gate}}"
LOG_FILE="${POLICY_GATE_LOG:-$OUT_DIR/policy-gate.log}"

mkdir -p "$OUT_DIR"
exec > >(tee -a "$LOG_FILE") 2>&1

echo "policy-gate: repo=$REPO out=$OUT_DIR"

if ! command -v opa >/dev/null 2>&1; then
	echo "policy-gate: ERROR: opa not on PATH (install Open Policy Agent)" >&2
	exit 1
fi

if [[ ! -f "$POLICY_FILE" ]]; then
	echo "policy-gate: ERROR: missing $POLICY_FILE" >&2
	exit 1
fi

INPUT_JSON="$OUT_DIR/policy-input.json"
RESULT_JSON="$OUT_DIR/policy-result.json"

echo ">> opa test policy/"
opa test "$ROOT/policy" -v

echo ">> chassis export --format opa"
set +e
cargo run -p chassis-cli --quiet -- export --format opa --repo "$REPO" --json >"$INPUT_JSON"
X=$?
set -e
if [[ "$X" -ne 0 ]]; then
	echo "policy-gate: chassis export failed with exit $X" >&2
	exit "$X"
fi

echo ">> opa eval data.chassis.release.result"
EVAL_RAW="$(mktemp)"
opa eval --format json -d "$POLICY_FILE" -i "$INPUT_JSON" 'data.chassis.release.result' >"$EVAL_RAW"

python3 - "$EVAL_RAW" "$RESULT_JSON" <<'PY'
import json, sys

raw_path, out_path = sys.argv[1], sys.argv[2]
with open(raw_path) as f:
    data = json.load(f)
try:
    val = data["result"][0]["expressions"][0]["value"]
except (KeyError, IndexError) as e:
    print("policy-gate: unexpected opa eval output:", data, file=sys.stderr)
    raise SystemExit(1)
with open(out_path, "w") as f:
    json.dump(val, f, indent=2)
    f.write("\n")
allow = val.get("allow")
if allow is not True:
    print("policy-gate: DENY", file=sys.stderr)
    print(json.dumps(val.get("deny_reasons", []), indent=2), file=sys.stderr)
    raise SystemExit(2)
print("policy-gate: ALLOW")
PY

echo "policy-gate: wrote $INPUT_JSON and $RESULT_JSON"
echo "policy-gate: OK"
