#!/usr/bin/env bash
#
# check-banned-crates.sh -- belt-and-suspenders guard that fails if any
# crate from the banned set ever appears in Cargo.lock.
#
# cargo-deny already enforces this list (`[bans] deny = [...]` in deny.toml),
# but cargo-deny is configurable: a future PR could weaken the policy in
# deny.toml itself. This script reads the same intent straight off the
# lockfile so weakening one without the other still trips CI.
#
# The list mirrors ADR-0025 ("Banned crates" section). To add or remove an
# entry, update both this script AND deny.toml AND ADR-0025.
#
# Usage:
#   scripts/check-banned-crates.sh [<path-to-Cargo.lock>]
#
# Defaults to <repo-root>/Cargo.lock.
#
# Exit code: 0 on clean, 1 on any banned crate present.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lockfile="${1:-$ROOT/Cargo.lock}"

if [[ ! -f "$lockfile" ]]; then
    echo "banned-crates: Cargo.lock not found at $lockfile" >&2
    exit 2
fi

# Synced with deny.toml `[bans] deny` and ADR-0025 "Banned crates".
# Anything in this list is structurally incompatible with chassis being
# sync- and offline-by-default. If a transitive dep starts to pull one
# of these in, the right response is an ADR explaining why -- not a
# silent allowance.
banned=(
    "openssl"
    "openssl-sys"
    "native-tls"
    "reqwest"
    "hyper"
    "tokio"
    "async-std"
)

violations=0
for crate in "${banned[@]}"; do
    # Match the canonical `name = "<crate>"` line that opens each
    # [[package]] block in Cargo.lock. Anchoring on `^name = "` avoids
    # false positives from dependency lines like `tokio = "..."`.
    if grep -qE "^name = \"${crate}\"\$" "$lockfile"; then
        echo "banned-crates: forbidden crate \"$crate\" present in $lockfile" >&2
        echo "banned-crates: see ADR-0025 (Banned crates). To admit \"$crate\"," >&2
        echo "banned-crates: write a new ADR, then remove it from deny.toml and this script." >&2
        violations=$((violations + 1))
    fi
done

if [[ $violations -gt 0 ]]; then
    echo "banned-crates: FAILED ($violations violation(s))" >&2
    exit 1
fi

echo "banned-crates: OK (${#banned[@]} crates checked against $lockfile)"
