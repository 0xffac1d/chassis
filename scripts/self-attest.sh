#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
export CHASSIS_REPO_ROOT="$ROOT"

OUT="${SELF_ATTEST_ARTIFACT:-$TMP/release-gate.dsse}"

cargo run -p chassis-core --example write_keypair --quiet -- "$TMP/priv.hex" "$TMP/pub.hex"

cargo run -p chassis-cli --quiet -- trace --repo "$ROOT" --json >"$TMP/trace.json"
cargo run -p chassis-cli --quiet -- drift --repo "$ROOT" --json >"$TMP/drift.json"

python3 - "$TMP/drift.json" <<'PY'
import json, sys
path = sys.argv[1]
with open(path) as f:
    d = json.load(f)
if d.get("summary", {}).get("abandoned", 0) > 0:
    print("drift abandoned > 0", file=sys.stderr)
    sys.exit(1)
PY

cargo run -p chassis-cli --quiet -- attest sign --repo "$ROOT" --private-key "$TMP/priv.hex" --out "$OUT"

cargo run -p chassis-cli --quiet -- attest verify "$OUT" --repo "$ROOT" --public-key "$TMP/pub.hex"

echo "self-attest: OK"
