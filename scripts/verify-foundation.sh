#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export CHASSIS_REPO_ROOT="$ROOT"

# Optional per-step log capture. When CHASSIS_VERIFY_LOG_DIR is set, each step's
# stdout+stderr is teed into a dedicated file under that directory so CI can
# upload the logs as build artifacts even when an earlier step fails.
LOG_DIR="${CHASSIS_VERIFY_LOG_DIR:-}"
if [[ -n "$LOG_DIR" ]]; then
  mkdir -p "$LOG_DIR"
fi

run_step() {
  local label="$1"
  shift
  echo ">> $label"
  if [[ -n "$LOG_DIR" ]]; then
    # PIPESTATUS preserves the real command's exit code through `tee`.
    "$@" 2>&1 | tee "$LOG_DIR/${label}.log"
    return "${PIPESTATUS[0]}"
  else
    "$@"
  fi
}

run_step docs-lint             bash scripts/docs-lint.sh
run_step pre-commit-parity     bash scripts/verify-pre-commit-parity.sh
run_step action-pin-hygiene    bash .github/scripts/check-action-pins.sh
run_step cargo-fmt          cargo fmt --check --all
run_step cargo-clippy       cargo clippy --workspace --all-targets -- -D warnings
run_step cargo-check        cargo check --workspace
# When we are running from an extracted source archive (`git archive` output has
# no `.git/`), tests that exercise the live repo's git metadata cannot run.
# The shipped CI matrix runs the full test suite in `foundation.yml`; this
# fallback keeps `source-archive (PR)` / `source-archive` extract-smoke
# meaningful without forcing an impossible precondition.
# `git rev-parse --git-dir` would walk to parents; check for a literal `.git`
# entry at $ROOT so we don't accidentally treat the surrounding CI checkout as
# our own git context.
if [[ -d "$ROOT/.git" ]] || [[ -f "$ROOT/.git" ]]; then
  run_step cargo-test         cargo test --workspace
else
  echo "verify-foundation: extracted release archive (no .git) — skipping git-dependent test sets"
  run_step cargo-test-no-git  cargo test --workspace -- \
    --skip drift::git::tests \
    --skip drift::report::tests::fixture_repo_drift_report_validates_schema \
    --skip attest::sign::tests \
    --skip attest::assemble::tests \
    --skip gate::tests::compute_produces_schema_valid_predicate_for_self_repo \
    --skip gate::tests::compute_require_scanners_rejects_nonzero_summary_errors \
    --skip cli_release_gate \
    --skip release_gate_happy_path_predicate_matches_schema_and_cli_fields \
    --skip release_gate_fail_on_drift_false_changes_outcome \
    --skip release_gate_happy_path_predicate_matches_schema \
    --skip drift_report_happy_path_matches_drift_schema
fi
# Explicit named gate: every Diagnostic emitted by any kernel surface must
# (a) validate against schemas/diagnostic.schema.json and
# (b) carry a ruleId bound to an accepted ADR's enforces[].
# This is also covered by the cargo-test step above; re-running here makes
# the governance contract visible in CI logs and lets `verify-foundation.sh`
# fail fast with a focused error if only the diagnostic surface regresses.
run_step diagnostic-governance \
    cargo test -p chassis-core --test diagnostic_governance
run_step npm-ci             npm ci --prefix packages/chassis-types
run_step npm-build          npm run build --prefix packages/chassis-types
run_step verify-schema-metadata node packages/chassis-types/scripts/verify-schema-metadata.mjs
run_step verify-fingerprint node packages/chassis-types/scripts/verify-fingerprint.mjs
run_step npm-test           npm test --prefix packages/chassis-types

# Best-effort compile of the reference Python CLI; failures here must not gate
# the foundation since reference/ is study-only material.
echo ">> python compileall (reference, best-effort)"
if [[ -n "$LOG_DIR" ]]; then
  python3 -m compileall -q reference/python-cli 2>&1 | tee "$LOG_DIR/python-compileall.log" || true
else
  python3 -m compileall -q reference/python-cli || true
fi

# When capturing logs, snapshot the schema-validation artifacts produced by the
# npm build so the CI artifact bundle is self-describing.
if [[ -n "$LOG_DIR" ]]; then
  for f in packages/chassis-types/fingerprint.sha256 packages/chassis-types/manifest.json; do
    if [[ -f "$f" ]]; then
      cp "$f" "$LOG_DIR/$(basename "$f")"
    fi
  done
fi

echo "verify-foundation: OK"
