#!/usr/bin/env bash
# Forbidden-phrase check for active documentation.
#
# Active docs must describe the repo as it is, not as it once was. A phrase is
# forbidden here if the surface it names has changed but the wording lingers in
# active docs (so the wording would mislead a fresh reader). The lint is line
# oriented; a flagged line wins unless it carries an inline allow marker
# (see ALLOW_MARKER below).
#
# Exit codes:
#   0  clean
#   1  one or more forbidden phrases hit in active docs
#   2  invocation error (missing file, bad shell)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Active docs the lint enforces against. ADRs live in docs/adr/ and are
# intentionally NOT included: ADRs are immutable records of decisions made at a
# point in time, not statements of current state.
FILES=(
  "$ROOT/README.md"
  "$ROOT/CLAUDE.md"
  "$ROOT/CONTRACT.yaml"
  "$ROOT/docs/WAVE-PLAN.md"
  "$ROOT/docs/ASSURANCE-LADDER.md"
  "$ROOT/packages/chassis-types/README.md"
)

# Parallel arrays: label[i], flag[i], pattern[i], reason[i].
# flag is the grep mode: F = fixed string, E = extended regex.
LABELS=(
  cli-binary-yet
  mcp-server
  scripts-chassis
  codegen-ts-types
  chassis-schemas
  stale-mnt-path
  stale-schema-count
)
FLAGS=(
  F
  F
  F
  F
  E
  E
  E
)
PATTERNS=(
  'no CLI binary yet'
  'MCP server'
  'scripts/chassis'
  'codegen/ts-types'
  'chassis-schemas([^-]|$)'
  '/mnt/C/(0xffac1d/)?chassis'
  '\b(12 root|17 root|20 (schema|generated|leaf)|20 .d.ts|25 (modules|schemas))\b'
)
REASONS=(
  '`crates/chassis-cli/` ships the `chassis` binary; drop the disclaimer.'
  '`crates/chassis-jsonrpc/` exposes JSON-RPC 2.0, not the Model Context Protocol. Use "JSON-RPC sidecar" or "machine surface" until a real MCP shim lands.'
  '`scripts/chassis/` was the previous-monolith path. Current entrypoints live under `scripts/` and `crates/chassis-cli/`.'
  '`codegen/ts-types/` was the previous-monolith path. TypeScript types live at `packages/chassis-types/`.'
  '`chassis-schemas` was a previous-monolith crate. Schemas live under `schemas/` (the regex excludes the manifest kind `chassis-schemas-manifest`).'
  'Stale absolute developer-machine path. Active docs must not embed local checkout paths; only `reference/` (snapshot zone, ADR-0025) is allowed to preserve them. `scripts/check-archive-hygiene.sh` enforces the same rule on source archives.'
  'Stale schema/module inventory in active docs. Canonical JSON schemas live under `schemas/` as 18 root artifacts plus 8 contract-kind branches (26 schemas total); `packages/chassis-types/` generates matching leaf modules.'
)

# An inline allow marker lets an active doc carry one of these phrases when the
# intent is to *forbid* it (e.g., a claim that says "must not call it an MCP
# server"). Put `chassis-lint-allow:<label>` somewhere on the same line.
ALLOW_MARKER='chassis-lint-allow'

for f in "${FILES[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "docs-lint: missing $f" >&2
    exit 2
  fi
done

fail=0

for i in "${!LABELS[@]}"; do
  label="${LABELS[$i]}"
  flag="${FLAGS[$i]}"
  pattern="${PATTERNS[$i]}"
  reason="${REASONS[$i]}"
  case "$flag" in
    F) grep_mode='-iFnH' ;;
    E) grep_mode='-iEnH' ;;
    *) echo "docs-lint: bad flag '$flag' for rule '$label'" >&2; exit 2 ;;
  esac
  for f in "${FILES[@]}"; do
    hits="$(grep $grep_mode -- "$pattern" "$f" 2>/dev/null || true)"
    [[ -z "$hits" ]] && continue
    while IFS= read -r hit; do
      [[ -z "$hit" ]] && continue
      if [[ "$hit" == *"${ALLOW_MARKER}:${label}"* ]]; then
        continue
      fi
      # Specific carve-out: the CONTRACT.yaml claim that *defines* the
      # mcp-server policy necessarily contains the phrase.
      if [[ "$label" == "mcp-server" && "$hit" == *"must not call it an MCP server"* ]]; then
        continue
      fi
      printf 'docs-lint [%s]: %s\n  reason: %s\n' "$label" "$hit" "$reason" >&2
      fail=1
    done <<<"$hits"
  done
done

if [[ $fail -ne 0 ]]; then
  echo "docs-lint: FAIL" >&2
  exit 1
fi

echo "docs-lint: OK"
