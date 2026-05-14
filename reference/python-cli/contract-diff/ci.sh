#!/usr/bin/env bash
# chassis-contract-diff.sh — gate breaking CONTRACT.yaml changes between BASE and HEAD.
#
# Iterates over CONTRACT.yaml files modified since BASE (default: origin/main),
# extracts the BASE version of each file via `git show`, and compares it against
# the working tree using scripts/chassis/contract-diff/diff.py with --fail-on-breaking.
#
# Exits 0 if no contracts changed, all changes are non-breaking, or a contract
# is newly added (no old version to compare against). Exits non-zero on the
# first breaking change.
#
# Usage:
#   scripts/ci/chassis-contract-diff.sh                  # base = origin/main
#   scripts/ci/chassis-contract-diff.sh main             # base = main
#   BASE=upstream/main scripts/ci/chassis-contract-diff.sh
set -euo pipefail

BASE="${1:-${BASE:-origin/main}}"
REPO_ROOT="$(git rev-parse --show-toplevel)"
DIFF_TOOL="$REPO_ROOT/scripts/chassis/contract-diff/diff.py"

if [[ ! -f "$DIFF_TOOL" ]]; then
  echo "chassis-contract-diff: $DIFF_TOOL not found" >&2
  exit 2
fi

if ! command -v yq >/dev/null 2>&1; then
  echo "chassis-contract-diff: yq (Mike Farah) is required on PATH" >&2
  exit 2
fi

# Verify BASE ref exists. If not, treat as no-op (e.g. shallow clones, first PR).
if ! git rev-parse --verify --quiet "$BASE" >/dev/null; then
  echo "chassis-contract-diff: base ref '$BASE' not found; skipping" >&2
  exit 0
fi

# Find CONTRACT.yaml files that differ between BASE and the current working
# tree. Using `git diff BASE` (no HEAD, no `...`) compares BASE against the
# working tree, so this catches both committed PR changes in CI and
# uncommitted local modifications during development. Newly added files
# (no BASE version) are skipped below — there is nothing to diff.
# Use :(glob) magic so the pattern matches CONTRACT.yaml at every depth,
# including the repo root. Without :(glob), git treats '**/CONTRACT.yaml'
# as '*/CONTRACT.yaml' and misses a top-level file.
mapfile -t CHANGED < <(
  git diff --name-only --diff-filter=AMR "$BASE" -- ':(glob)**/CONTRACT.yaml' \
    | sort -u
)

if [[ ${#CHANGED[@]} -eq 0 ]]; then
  echo "chassis-contract-diff: no CONTRACT.yaml changes vs $BASE"
  exit 0
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

FAILED=0
for path in "${CHANGED[@]}"; do
  # Skip files that did not exist in BASE (newly added contracts).
  if ! git show "$BASE:$path" >/dev/null 2>&1; then
    echo "chassis-contract-diff: $path is new (no base version); skipping"
    continue
  fi

  base_copy="$WORK/$(echo "$path" | tr '/' '_').old"
  git show "$BASE:$path" > "$base_copy"

  echo "chassis-contract-diff: $path (vs $BASE)"
  if ! python3 "$DIFF_TOOL" \
       --old "$base_copy" \
       --new "$REPO_ROOT/$path" \
       --format text \
       --fail-on-breaking; then
    FAILED=1
    echo "  -> BREAKING CHANGES DETECTED in $path" >&2
  fi
done

if [[ "$FAILED" -ne 0 ]]; then
  echo "" >&2
  echo "chassis-contract-diff: one or more contracts have breaking changes" >&2
  echo "Resolve by reverting the breaking change, or by acknowledging it via" >&2
  echo "an explicit major-version bump in CONTRACT.yaml 'since:' field." >&2
  exit 1
fi

echo "chassis-contract-diff: all contract changes are non-breaking"
exit 0
