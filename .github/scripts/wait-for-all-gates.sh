#!/usr/bin/env bash
# Poll until every gate workflow has a successful run for HEAD_SHA.
set -euo pipefail
REPO="${GITHUB_REPOSITORY:?}"
SHA="${HEAD_SHA:?}"
for attempt in $(seq 1 90); do
	missing=""
	for wf in foundation.yml supply-chain.yml policy-gate.yml self-attest.yml source-archive.yml; do
		rid="$(gh run list --repo "$REPO" --workflow "$wf" --commit "$SHA" --json databaseId,conclusion --jq '.[] | select(.conclusion=="success") | .databaseId' | head -1)"
		if [[ -z "${rid:-}" ]]; then
			missing="$missing $wf"
		fi
	done
	if [[ -z "$missing" ]]; then
		echo "wait-for-all-gates: OK (commit $SHA)"
		exit 0
	fi
	echo "wait-for-all-gates: attempt $attempt missing:$missing"
	sleep 10
done
echo "wait-for-all-gates: TIMEOUT waiting for gates at $SHA" >&2
exit 1
