#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
export CHASSIS_REPO_ROOT="$ROOT"

if [[ -n "${SELF_ATTEST_DIR:-}" ]]; then
    ARTIFACT_DIR="$SELF_ATTEST_DIR"
elif [[ -n "${SELF_ATTEST_ARTIFACT:-}" ]]; then
    ARTIFACT_DIR="$(dirname "$SELF_ATTEST_ARTIFACT")"
else
    ARTIFACT_DIR="$TMP"
fi
mkdir -p "$ARTIFACT_DIR"

OUT="${SELF_ATTEST_ARTIFACT:-$ARTIFACT_DIR/release-gate.dsse}"
TRACE_OUT="$ARTIFACT_DIR/trace-graph.json"
DRIFT_OUT="$ARTIFACT_DIR/drift-report.json"
STATEMENT_OUT="$ARTIFACT_DIR/in-toto-statement.json"
PREDICATE_OUT="$ARTIFACT_DIR/release-gate.json"

cargo run -p chassis-core --example write_keypair --quiet -- "$TMP/priv.hex" "$TMP/pub.hex"

cargo run -p chassis-cli --quiet -- trace --repo "$ROOT" --json >"$TRACE_OUT"

# chassis-cli drift returns DRIFT_DETECTED (exit 5) for ANY non-zero drift
# count (stale + abandoned + missing). The python gate below is the actual
# release-gate policy: abandoned or missing implementation evidence is fatal;
# informational drift is not.
# Tolerate exit 5 here so the python check decides; surface every other
# non-zero exit as a real failure.
DRIFT_EXIT=0
cargo run -p chassis-cli --quiet -- drift --repo "$ROOT" --json >"$DRIFT_OUT" || DRIFT_EXIT=$?
if [[ "$DRIFT_EXIT" != "0" && "$DRIFT_EXIT" != "5" ]]; then
    echo "self-attest: drift command failed with exit $DRIFT_EXIT" >&2
    exit "$DRIFT_EXIT"
fi

# Re-validate every emitted artifact against its canonical schema, even though
# the CLI already gates on validation, so a regression in the CLI cannot smuggle
# a malformed artifact past CI.
cargo run -p chassis-core --example validate_artifact --quiet -- trace-graph    "$TRACE_OUT"
cargo run -p chassis-core --example validate_artifact --quiet -- drift-report   "$DRIFT_OUT"

python3 - "$TRACE_OUT" <<'PY'
import json, sys
path = sys.argv[1]
with open(path) as f:
    g = json.load(f)
orphans = g.get("orphan_sites", [])
if orphans:
    print("trace graph has orphan @claim sites:", file=sys.stderr)
    for site in orphans:
        print(f"  - {site.get('claim_id')} at {site.get('file')}:{site.get('line')}", file=sys.stderr)
    sys.exit(1)
PY

python3 - "$DRIFT_OUT" <<'PY'
import json, sys
path = sys.argv[1]
with open(path) as f:
    d = json.load(f)
summary = d.get("summary", {})
if summary.get("abandoned", 0) > 0 or summary.get("missing", 0) > 0:
    print(f"drift gate failed: {summary}", file=sys.stderr)
    sys.exit(1)
PY

cargo run -p chassis-cli --quiet -- attest sign --repo "$ROOT" --private-key "$TMP/priv.hex" --out "$OUT"

cargo run -p chassis-core --example validate_artifact --quiet -- dsse-envelope "$OUT"

cargo run -p chassis-cli --quiet -- attest verify "$OUT" --repo "$ROOT" --public-key "$TMP/pub.hex" --json >"$STATEMENT_OUT"

cargo run -p chassis-core --example validate_artifact --quiet -- in-toto-statement "$STATEMENT_OUT"

python3 - "$STATEMENT_OUT" "$PREDICATE_OUT" <<'PY'
import json, sys
statement_path, predicate_path = sys.argv[1:3]
with open(statement_path) as f:
    statement = json.load(f)
with open(predicate_path, "w") as f:
    json.dump(statement["predicate"], f, indent=2, sort_keys=True)
    f.write("\n")
PY

cargo run -p chassis-core --example validate_artifact --quiet -- release-gate "$PREDICATE_OUT"

echo "self-attest: OK"
