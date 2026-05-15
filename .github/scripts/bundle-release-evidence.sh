#!/usr/bin/env bash
# Download workflow artifacts for a commit SHA and bundle them for the
# release-evidence workflow. Requires gh + GITHUB_TOKEN.
set -euo pipefail
REPO="${GITHUB_REPOSITORY:?}"
SHA="${HEAD_SHA:?}"
OUT="${EVIDENCE_DIR:-evidence}"
mkdir -p "$OUT"

download_one() {
	local wf="$1"
	local name="$2"
	local dest="$3"
	local rid
	rid="$(gh run list --repo "$REPO" --workflow "$wf" --commit "$SHA" --json databaseId,conclusion --jq '.[] | select(.conclusion=="success") | .databaseId' | head -1)"
	if [[ -z "$rid" ]]; then
		echo "bundle: ERROR no successful run for workflow=$wf commit=$SHA" >&2
		return 1
	fi
	mkdir -p "$dest"
	gh run download "$rid" -n "$name" -D "$dest"
}

# Optional: best-effort auxiliary logs
download_optional() {
	local wf="$1"
	local name="$2"
	local dest="$3"
	local rid
	rid="$(gh run list --repo "$REPO" --workflow "$wf" --commit "$SHA" --json databaseId,conclusion --jq '.[] | select(.conclusion=="success") | .databaseId' | head -1)"
	[[ -n "$rid" ]] || return 0
	mkdir -p "$dest"
	gh run download "$rid" -n "$name" -D "$dest" 2>/dev/null || true
}

# foundation.yml uploads matrix artifact per OS
download_one foundation.yml verify-foundation-logs-ubuntu-latest "$OUT/verify-foundation-logs-ubuntu-latest"
download_one supply-chain.yml supply-chain-logs "$OUT/supply-chain-logs"
download_one policy-gate.yml policy-gate-artifacts "$OUT/policy-gate-artifacts"
download_one self-attest.yml self-attest-artifacts "$OUT/self-attest-artifacts"
download_one source-archive.yml chassis-source-archive "$OUT/chassis-source-archive"
download_optional source-archive.yml archive-smoke-logs "$OUT/archive-smoke-logs"
download_optional source-archive.yml source-archive-logs "$OUT/source-archive-logs"

# SLSA provenance artifact name is chosen by slsa-github-generator (*.intoto.jsonl).
rid_archive="$(gh run list --repo "$REPO" --workflow source-archive.yml --commit "$SHA" --json databaseId,conclusion --jq '.[] | select(.conclusion=="success") | .databaseId' | head -1)"
if [[ -n "${rid_archive:-}" ]]; then
	mkdir -p "$OUT/slsa-provenance"
	mapfile -t prov_names < <(gh api "repos/$REPO/actions/runs/$rid_archive/artifacts" --paginate --jq '.artifacts[] | select(.name | endswith("intoto.jsonl")) | .name' | sort -u)
	for aname in "${prov_names[@]}"; do
		[[ -z "$aname" ]] && continue
		gh run download "$rid_archive" -n "$aname" -D "$OUT/slsa-provenance" || true
	done
fi

echo "bundle: OK -> $OUT"
