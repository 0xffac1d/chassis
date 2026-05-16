#!/usr/bin/env bash
# Fail if a GitHub Actions workflow pins a critical third-party action to a
# mutable ref (tag/branch) instead of a commit SHA. Reusable workflow calls may
# stay on semver tags; list them in .github/action-pin-allowlist.txt (one
# `owner/repo/path@ref` per line).
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
ALLOWLIST="${ROOT}/.github/action-pin-allowlist.txt"

violations=0
while IFS= read -r -d '' f; do
	while IFS= read -r line; do
		[[ "$line" =~ uses:[[:space:]]*([a-zA-Z0-9._-]+/[a-zA-Z0-9._./-]+)@([a-zA-Z0-9._+/-]+) ]] || continue
		spec="${BASH_REMATCH[1]}"
		ref="${BASH_REMATCH[2]}"
		# Trim YAML inline comments
		ref="${ref%%#*}"
		ref="${ref%"${ref##*[![:space:]]}"}"

		if [[ "$ref" =~ ^[a-f0-9]{40}$ ]]; then
			continue
		fi
		if [[ -f "$ALLOWLIST" ]] && grep -Fxq "${spec}@${ref}" "$ALLOWLIST"; then
			continue
		fi
		# GitHub's first-party CodeQL bundles are pinned in-tree as full SHAs.
		if [[ "$spec" =~ ^github/codeql-action/ ]]; then
			if [[ "$ref" =~ ^[a-f0-9]{40}$ ]]; then
				continue
			fi
		fi
		echo "check-action-pins: unpinned action in ${f#"$ROOT/"}: uses: ${spec}@${ref}" >&2
		violations=$((violations + 1))
	done < <(grep -E '^[[:space:]]*-[[:space:]]+uses:|[[:space:]]uses:' "$f" || true)
done < <(find "$ROOT/.github/workflows" -name '*.yml' -print0)

if [[ "$violations" -ne 0 ]]; then
	echo "check-action-pins: FAILED ($violations unpinned use(s)); pin to a 40-char SHA or add an allowlist entry." >&2
	exit 1
fi
echo "check-action-pins: OK"
