#!/usr/bin/env bash
# Ensure `.pre-commit-config.yaml` names every load-bearing command that
# `verify-foundation.sh` runs so local pre-commit hooks cannot silently drift.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
cfg=".pre-commit-config.yaml"
if [[ ! -f "$cfg" ]]; then
	echo "verify-pre-commit-parity: missing $cfg" >&2
	exit 1
fi
# Content checks: each string should appear in the YAML.
needles=(
	"docs-lint.sh"
	"cargo fmt"
	"cargo clippy"
	"cargo check"
	"cargo test"
	"chassis-types"
	"verify-schema-metadata"
	"verify-fingerprint"
	"diagnostic_governance"
	"action-pin-hygiene"
	"check-action-pins"
)
for n in "${needles[@]}"; do
	if ! grep -qF "$n" "$cfg"; then
		echo "verify-pre-commit-parity: $cfg must reference '$n' (mirror verify-foundation.sh)" >&2
		exit 1
	fi
done
echo "verify-pre-commit-parity: OK"
