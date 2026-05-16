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
download_one semgrep.yml semgrep-sarif "$OUT/semgrep-sarif"
download_one semgrep.yml scanner-semgrep-summary "$OUT/scanner-semgrep-summary"
download_one codeql.yml codeql-sarif "$OUT/codeql-sarif"
download_one codeql.yml scanner-codeql-summary "$OUT/scanner-codeql-summary"

mkdir -p "$OUT/dist"
if [[ -f "$OUT/scanner-semgrep-summary/scanner-semgrep.json" ]]; then
	cp "$OUT/scanner-semgrep-summary/scanner-semgrep.json" "$OUT/dist/"
fi
if [[ -f "$OUT/scanner-codeql-summary/scanner-codeql.json" ]]; then
	cp "$OUT/scanner-codeql-summary/scanner-codeql.json" "$OUT/dist/"
fi

semgrep_sarif_sha256=""
if [[ -f "$OUT/semgrep-sarif/semgrep.sarif" ]]; then
	semgrep_sarif_sha256="$(sha256sum "$OUT/semgrep-sarif/semgrep.sarif" | awk '{print $1}')"
fi
codeql_sarif_sha256=""
cq="$(find "$OUT/codeql-sarif" -type f -name "*.sarif" -print -quit)"
if [[ -n "${cq:-}" ]]; then
	codeql_sarif_sha256="$(sha256sum "$cq" | awk '{print $1}')"
fi
norm_semgrep_sha=""
norm_codeql_sha=""
if [[ -f "$OUT/dist/scanner-semgrep.json" ]]; then
	norm_semgrep_sha="$(sha256sum "$OUT/dist/scanner-semgrep.json" | awk '{print $1}')"
fi
if [[ -f "$OUT/dist/scanner-codeql.json" ]]; then
	norm_codeql_sha="$(sha256sum "$OUT/dist/scanner-codeql.json" | awk '{print $1}')"
fi

jq -n \
	--arg ss "$semgrep_sarif_sha256" \
	--arg cs "$codeql_sarif_sha256" \
	--arg ns "$norm_semgrep_sha" \
	--arg nc "$norm_codeql_sha" \
	'{
	  semgrepSarifSha256: (if $ss == "" then null else $ss end),
	  codeqlSarifSha256: (if $cs == "" then null else $cs end),
	  normalizedSummarySha256s: {
	    semgrep: (if $ns == "" then null else $ns end),
	    codeql: (if $nc == "" then null else $nc end)
	  }
	}' > "$OUT/scanner-manifest.json"

download_optional source-archive.yml archive-smoke-logs "$OUT/archive-smoke-logs"
download_optional source-archive.yml source-archive-logs "$OUT/source-archive-logs"

# SLSA provenance artifact name is chosen by slsa-github-generator (*.intoto.jsonl).
rid_archive="$(gh run list --repo "$REPO" --workflow source-archive.yml --commit "$SHA" --json databaseId,conclusion --jq '.[] | select(.conclusion=="success") | .databaseId' | head -1)"
	mkdir -p "$OUT/slsa-provenance"
	mapfile -t prov_names < <(gh api "repos/$REPO/actions/runs/$rid_archive/artifacts" --paginate --jq '.artifacts[] | select(.name | endswith("intoto.jsonl")) | .name' | sort -u)
	for aname in "${prov_names[@]}"; do
		[[ -z "$aname" ]] && continue
		gh run download "$rid_archive" -n "$aname" -D "$OUT/slsa-provenance" || true
	done
fi

echo "bundle: OK -> $OUT"
