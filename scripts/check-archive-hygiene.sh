#!/usr/bin/env bash
#
# check-archive-hygiene.sh -- fail if a candidate tree or archive lacks required
# source paths, contains build artifacts/vendored caches, or carries stale
# developer-machine paths outside reference/**.
#
# Usage:
#   scripts/check-archive-hygiene.sh <path>
#
# <path> may be one of:
#   - a directory (scanned in place)
#   - a .tar.gz / .tgz / .tar / .zip file (extracted to a temp dir, then scanned)
#
# Forbidden artifacts (any depth unless noted):
#   - .git/                       (root only -- a packaged source archive must
#                                  not embed a git database)
#   - target/                     (cargo build output)
#   - node_modules/               (npm install output)
#   - __pycache__/                (python bytecode cache)
#   - *.pyc                       (python bytecode files)
#
# Forbidden file content:
#   - absolute paths matching     /mnt/C/chassis  or  /mnt/C/0xffac1d/chassis
#     anywhere in tracked files, EXCEPT under reference/** which is
#     documented in ADR-0025 as the snapshot zone where the prior
#     project's literal paths are preserved on purpose. The hygiene
#     script itself, `scripts/docs-lint.sh`, and the supply-chain ADR are also
#     exempt because they must embed or cite the literal pattern to enforce it.
#
# Required paths (relative to the archive root directory — normally the prefix
# from `git archive`, e.g. chassis-<sha>/) match files required by docs-lint,
# README, CI evidence, and active documentation that cites reference/.
#
# Exit code: 0 on clean, 1 if any check fails (with diagnostic output).

set -euo pipefail

usage() {
    cat >&2 <<EOF
usage: $(basename "$0") <path-to-tree-or-archive>
EOF
    exit 2
}

if [[ $# -ne 1 ]]; then
    usage
fi

input="$1"
if [[ ! -e "$input" ]]; then
    echo "hygiene: input does not exist: $input" >&2
    exit 2
fi

# Resolve the directory to scan. If the input is an archive, extract it.
scan_dir=""
cleanup_dir=""
cleanup() {
    if [[ -n "$cleanup_dir" && -d "$cleanup_dir" ]]; then
        rm -rf "$cleanup_dir"
    fi
}
trap cleanup EXIT

if [[ -d "$input" ]]; then
    scan_dir="$input"
else
    cleanup_dir="$(mktemp -d -t chassis-hygiene-XXXXXX)"
    case "$input" in
        *.tar.gz|*.tgz)
            tar -xzf "$input" -C "$cleanup_dir"
            ;;
        *.tar)
            tar -xf "$input" -C "$cleanup_dir"
            ;;
        *.zip)
            unzip -q "$input" -d "$cleanup_dir"
            ;;
        *)
            echo "hygiene: unrecognised archive extension: $input" >&2
            echo "hygiene: supported: .tar.gz .tgz .tar .zip or a directory" >&2
            exit 2
            ;;
    esac
    # If the archive contained a single top-level dir (the convention for
    # `git archive --prefix=foo/`), scan that dir; otherwise scan the
    # extraction root.
    entries=( "$cleanup_dir"/* )
    if [[ ${#entries[@]} -eq 1 && -d "${entries[0]}" ]]; then
        scan_dir="${entries[0]}"
    else
        scan_dir="$cleanup_dir"
    fi
fi

violations=0
report() {
    violations=$((violations + 1))
    echo "hygiene: $1" >&2
}

# ---------------------------------------------------------------------------
# 1. Forbidden directories / files
# ---------------------------------------------------------------------------

# Root .git/ only -- nested .git inside a fixture (e.g. fixtures/drift-repo)
# is intentional, but a real .git database at the archive root means we
# packaged the developer's working tree by mistake.
if [[ -e "$scan_dir/.git" ]]; then
    report "root .git/ present at $scan_dir/.git"
fi

# target/, node_modules/, __pycache__/ at any depth.
while IFS= read -r -d '' hit; do
    report "build/cache directory: ${hit#"$scan_dir/"}"
done < <(find "$scan_dir" \
    \( -type d -name target \
    -o -type d -name node_modules \
    -o -type d -name __pycache__ \) \
    -print0 2>/dev/null)

# *.pyc files at any depth.
while IFS= read -r -d '' hit; do
    report "python bytecode: ${hit#"$scan_dir/"}"
done < <(find "$scan_dir" -type f -name '*.pyc' -print0 2>/dev/null)

# ---------------------------------------------------------------------------
# 2. Stale developer-machine paths in tracked file content
# ---------------------------------------------------------------------------
# Allow the entire reference/ tree (snapshot zone, see ADR-0025), the hygiene
# script itself, `scripts/docs-lint.sh` (it embeds the same detector regex in its
# active-docs forbidden-pattern table), and the supply-chain ADR (both must
# mention the pattern in order to detect it).
# Use grep -P for fixed-string alternation; fall back to extended regex for
# portability when -P is unavailable.
PATTERN='/mnt/C/(0xffac1d/)?chassis'

# grep returns 1 on no matches; we want to *succeed* on no matches. Capture
# matches into an array and report them.
mapfile -t hits < <(
    grep -RInE "$PATTERN" "$scan_dir" \
        --binary-files=without-match \
        --exclude-dir=.git \
        --exclude-dir=target \
        --exclude-dir=node_modules \
        --exclude-dir=__pycache__ \
        --exclude-dir=reference \
        --exclude=check-archive-hygiene.sh \
        --exclude=docs-lint.sh \
        --exclude=ADR-0025-supply-chain-policy.md \
        2>/dev/null \
        || true
)

for hit in "${hits[@]}"; do
    [[ -z "$hit" ]] && continue
    report "stale developer-machine path: ${hit#"$scan_dir/"}"
done

# ---------------------------------------------------------------------------
# 3. Required archive contents (active docs / foundation verifier parity)
# ---------------------------------------------------------------------------
REQUIRED=(
  "CLAUDE.md"
  ".gitignore"
  ".github/workflows/foundation.yml"
  ".github/workflows/supply-chain.yml"
  ".github/workflows/policy-gate.yml"
  ".github/workflows/self-attest.yml"
  ".github/workflows/source-archive.yml"
  ".github/workflows/release-evidence.yml"
  ".github/workflows/semgrep.yml"
  ".github/workflows/codeql.yml"
  ".github/workflows/renovate-config-validator.yml"
  "README.md"
  "CONTRACT.yaml"
  "docs/WAVE-PLAN.md"
  "docs/ASSURANCE-LADDER.md"
  "scripts/docs-lint.sh"
  "scripts/verify-foundation.sh"
  ".pre-commit-config.yaml"
  ".semgrep.yml"
  "renovate.json"
  "packages/chassis-types/README.md"
)
for rel in "${REQUIRED[@]}"; do
    p="$scan_dir/$rel"
    if [[ ! -e "$p" ]]; then
        report "missing required path: $rel"
    fi
done
if [[ ! -d "$scan_dir/reference" ]]; then
    report "missing required directory: reference/"
fi

# ---------------------------------------------------------------------------
if [[ $violations -gt 0 ]]; then
    echo "hygiene: FAILED ($violations violation(s)) in $scan_dir" >&2
    exit 1
fi

echo "hygiene: OK ($scan_dir)"
