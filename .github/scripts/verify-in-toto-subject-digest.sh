#!/usr/bin/env bash
# Re-verify in-toto subject digests against the repo checkout (schema manifest).
set -euo pipefail
ROOT="${1:?repo root}"
STMT="${2:?path to in-toto statement JSON}"
cd "$ROOT"
EXPECT="$(jq -r '.subject[0].digest.sha256' "$STMT")"
GOT="$(cargo run -p chassis-core --example fingerprint -q)"
if [[ "$EXPECT" != "$GOT" ]]; then
	echo "CH-EVIDENCE-DIGEST-MISMATCH: expected schema subject sha256=$EXPECT got=$GOT" >&2
	exit 1
fi
echo "evidence-digest: OK schema manifest matches repo (sha256=$GOT)"
