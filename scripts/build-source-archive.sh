#!/usr/bin/env bash
#
# build-source-archive.sh -- produce a release-ready source tarball from
# the current git HEAD and verify it with check-archive-hygiene.sh.
#
# Usage:
#   scripts/build-source-archive.sh [<output-tarball>] [<git-ref>]
#
# Defaults:
#   <output-tarball>  dist/chassis-source-<short-sha>.tar.gz
#   <git-ref>         HEAD
#
# The tarball is produced via `git archive`, which only includes
# committed paths -- untracked files (target/, node_modules/, .env, etc.)
# are excluded by construction. Patterns in `.gitattributes` tagged
# `export-ignore` are omitted as well — keep those limited to IDE/local-only
# noise so release archives stay complete for `scripts/docs-lint.sh` and
# `scripts/check-archive-hygiene.sh`.
#
# The hygiene script rejects build/cache leakage and verifies required paths.
#
# Acceptance: see ADR-0025 -- this script is the single chokepoint
# through which release archives must be produced.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "build-source-archive: not inside a git working tree" >&2
    exit 2
fi

ref="${2:-HEAD}"
short_sha="$(git rev-parse --short=12 "$ref")"
default_out="dist/chassis-source-${short_sha}.tar.gz"
out="${1:-$default_out}"

# Use a normalized prefix so extraction lands in chassis-<sha>/ regardless
# of the local checkout name.
prefix="chassis-${short_sha}/"

mkdir -p "$(dirname "$out")"

echo "build-source-archive: ref=$ref prefix=$prefix out=$out"

# `git archive` honors `.gitattributes` export-ignore rules; consult that file
# before adding new excludes.
git archive \
    --format=tar.gz \
    --prefix="$prefix" \
    --output="$out" \
    "$ref"

echo "build-source-archive: produced $out ($(wc -c <"$out") bytes)"

# Defense-in-depth verification of the produced archive.
"$ROOT/scripts/check-archive-hygiene.sh" "$out"

echo "build-source-archive: OK"
