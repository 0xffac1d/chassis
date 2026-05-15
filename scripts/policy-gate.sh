#!/usr/bin/env bash
# Fail-closed OPA gate over Chassis export facts.
#
# Steps:
#   1. `opa test policy/` (Rego unit tests).
#   2. `cargo run -p chassis-cli -- export --format opa` writes JSON validated by the
#      CLI against schemas/opa-input.schema.json (saved as policy-input.json here).
#   3. `opa eval --schema schemas/opa-input.schema.json` evaluates data.chassis.release.result.
#
# Writes under POLICY_GATE_OUT_DIR (default dist/policy-gate/): policy-input.json,
# policy-result.json, policy-gate.log (paths in the log are repo-relative when possible).
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

_rel_from_root() {
	local p="$1"
	case "$p" in
		"$ROOT"|"$ROOT"/) printf '.' ;;
		"$ROOT"/*) printf '%s' "${p#"$ROOT"/}" ;;
		*) printf '%s' "$p" ;;
	esac
}
echo "policy-gate: repo=$(_rel_from_root "$REPO") out=$(_rel_from_root "$OUT_DIR")"

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
OPA_INPUT_SCHEMA="$ROOT/schemas/opa-input.schema.json"

echo ">> opa test policy/"
opa test "$ROOT/policy" -v

echo ">> chassis export --format opa"
echo "policy-gate: chassis validates export JSON against schemas/opa-input.schema.json before writing."
set +e
cargo run -p chassis-cli --quiet -- export --format opa --repo "$REPO" --json >"$INPUT_JSON"
X=$?
set -e
if [[ "$X" -ne 0 ]]; then
	echo "policy-gate: chassis export failed with exit $X" >&2
	exit "$X"
fi

if [[ ! -f "$OPA_INPUT_SCHEMA" ]]; then
	echo "policy-gate: ERROR: missing $OPA_INPUT_SCHEMA" >&2
	exit 1
fi

# Defense-in-depth: re-check the exported OPA input against the canonical
# schema here, independent of the CLI's own emit-time check. Catches the case
# where someone hand-edits POLICY_GATE_OUT_DIR/policy-input.json or where the
# CLI is mocked in CI. Uses python+jsonschema when available; else a minimal
# inline shape check for `input.version == 1` and a few required fields. Fails
# closed: a missing dependency does NOT silently weaken the gate.
echo ">> shape-check $INPUT_JSON against opa-input.schema.json"
python3 - "$INPUT_JSON" "$OPA_INPUT_SCHEMA" <<'PY'
import json, sys
from pathlib import Path

input_path, schema_path = Path(sys.argv[1]), Path(sys.argv[2])
try:
    payload = json.loads(input_path.read_text())
    schema = json.loads(schema_path.read_text())
except Exception as e:
    print(f"policy-gate: cannot read input/schema: {e}", file=sys.stderr)
    raise SystemExit(1)

try:
    import jsonschema  # type: ignore
    try:
        jsonschema.validate(payload, schema)
        print("policy-gate: opa-input shape valid (jsonschema)")
        sys.exit(0)
    except jsonschema.ValidationError as e:
        print(f"policy-gate: opa-input shape INVALID: {e.message}", file=sys.stderr)
        raise SystemExit(1)
except ImportError:
    pass

# Minimal fallback: enforce the load-bearing constraints by hand. Keep this
# in lockstep with schemas/opa-input.schema.json so a missing python
# `jsonschema` dependency cannot weaken the gate beyond shape sanity.
def fail(msg: str):
    print(f"policy-gate: opa-input shape INVALID: {msg}", file=sys.stderr)
    raise SystemExit(1)

if not isinstance(payload, dict) or "input" not in payload:
    fail("top-level object must carry an `input` field")
inp = payload["input"]
if not isinstance(inp, dict):
    fail("`input` must be an object")
if inp.get("version") != 1:
    fail("`input.version` must be the integer 1 (Chassis policy-input v1)")
for field in ("repo", "contracts", "claims", "diagnostics", "exemptions", "drift_summary"):
    if field not in inp:
        fail(f"`input.{field}` required")
if not isinstance(inp.get("repo"), dict) or "root" not in inp["repo"]:
    fail("`input.repo.root` required")
ds = inp["drift_summary"]
if not isinstance(ds, dict) or any(k not in ds for k in ("stale", "abandoned", "missing")):
    fail("`input.drift_summary` must include stale/abandoned/missing")
print("policy-gate: opa-input shape valid (fallback shape check; install python `jsonschema` for full draft-2020-12 coverage)")
PY

echo ">> opa eval data.chassis.release.result (schema-backed)"
EVAL_RAW="$(mktemp)"
set +e
opa eval --format json \
	--schema "$OPA_INPUT_SCHEMA" \
	-d "$POLICY_FILE" \
	-i "$INPUT_JSON" \
	'data.chassis.release.result' >"$EVAL_RAW"
EVAL_EXIT=$?
set -e
if [[ "$EVAL_EXIT" -ne 0 ]]; then
	echo "policy-gate: opa eval exited $EVAL_EXIT" >&2
	cat "$EVAL_RAW" >&2 || true
	exit "$EVAL_EXIT"
fi

python3 - "$EVAL_RAW" "$RESULT_JSON" <<'PY'
import json, sys

raw_path, out_path = sys.argv[1], sys.argv[2]
with open(raw_path) as f:
    data = json.load(f)
errs = data.get("errors")
if errs:
    print("policy-gate: opa eval reported errors:", file=sys.stderr)
    print(json.dumps(errs, indent=2), file=sys.stderr)
    raise SystemExit(1)
results = data.get("result")
if not isinstance(results, list) or len(results) == 0:
    print("policy-gate: opa eval returned no result (undefined query?)", data, file=sys.stderr)
    raise SystemExit(1)
try:
    val = results[0]["expressions"][0]["value"]
except (KeyError, IndexError, TypeError) as e:
    print("policy-gate: unexpected opa eval output:", data, file=sys.stderr)
    raise SystemExit(1) from e
if val is None:
    print("policy-gate: policy result is undefined", file=sys.stderr)
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
py_ec=$?
rm -f "$EVAL_RAW"
if [[ "$py_ec" -ne 0 ]]; then
	exit "$py_ec"
fi

echo "policy-gate: wrote $INPUT_JSON and $RESULT_JSON"
echo "policy-gate: OK"
