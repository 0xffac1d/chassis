#!/usr/bin/env bash
# Regenerates a bare repo at fixtures/drift-repo/drift_fixture.git for drift::git tests
# (avoids nesting a normal .git worktree inside the chassis repo clone).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WT="$(mktemp -d)"
BARE="$ROOT/fixtures/drift-repo/drift_fixture.git"
trap 'rm -rf "$WT"' EXIT

rm -rf "$BARE"

cd "$WT"
git init --initial-branch=main
git config user.email "fixture@chassis.dev"
git config user.name "Chassis drift fixture"

write_contract() {
  local text="$1"
  cat > CONTRACT.yaml <<EOF
name: drift-fixture
kind: library
version: "0.1.0"
purpose: "Git fixture for drift git walker tests"
status: stable
since: "0.1.0"
assurance_level: declared
owner: chassis-fixtures
exports:
  - path: "src_impl.rs"
    kind: module
    description: "impl"
invariants:
  - id: drift.fixture.alpha
    text: "${text}"
edge_cases: []
EOF
}

export GIT_AUTHOR_DATE="2024-06-01T10:00:01Z"
export GIT_COMMITTER_DATE="$GIT_AUTHOR_DATE"
write_contract "first revision"
echo "// impl v1" > src_impl.rs
git add CONTRACT.yaml src_impl.rs
git commit -m "init contract + impl"

export GIT_AUTHOR_DATE="2024-06-03T09:00:04Z"
export GIT_COMMITTER_DATE="$GIT_AUTHOR_DATE"
echo "// impl v2" > src_impl.rs
git add src_impl.rs
git commit -m "impl churn 1"

export GIT_AUTHOR_DATE="2024-06-10T12:00:05Z"
export GIT_COMMITTER_DATE="$GIT_AUTHOR_DATE"
write_contract "second revision"
git add CONTRACT.yaml
git commit -m "revise claim text"

export GIT_AUTHOR_DATE="2024-06-12T15:00:09Z"
export GIT_COMMITTER_DATE="$GIT_AUTHOR_DATE"
echo "// impl v3" > src_impl.rs
git add src_impl.rs
git commit -m "impl churn 2"

cd "$ROOT"
git clone --bare "$WT" "$BARE"
# Drop sample hooks/templates from the bundled bare repo — only objects/refs matter for tests.
rm -rf "$BARE/hooks"
