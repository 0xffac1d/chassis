---
id: ADR-0032
title: CLI module wiring contract
status: accepted
date: "2026-04-27"

enforces:
  - rule: CHASSIS-MODULE-WIRING-DEAD
    description: "A Python module under scripts/chassis/ has zero inbound imports from any in-repo module and is not declared as an intentional CLI entry point in the bash dispatcher."
  - rule: CHASSIS-MODULE-WIRING-CLI-UNDECLARED
    description: "A Python module is referenced as an entry point in the bash dispatcher but is not present at the expected path, or is present but not invoked from any case branch."
  - rule: CHASSIS-MODULE-WIRING-DUPLICATE-ENTRY
    description: "Two or more case branches in the bash dispatcher invoke the same Python module path."
applies_to:
  - "scripts/chassis/*.py"
  - "scripts/chassis/chassis"
  - "scripts/chassis/gates/module_wiring.py"
  - "config/chassis/module-wiring.toml"
supersedes: []
tags:
  - chassis
  - cli
  - governance
  - decomposition
---

# ADR-0032: CLI module wiring contract

## Context

`scripts/chassis/` flattens dozens of top-level Python files into a single
namespace. Some are libraries imported by other modules (for example
`repo_layout.py` is imported 35× across the tree). Others are intentional
CLI entry points invoked from the bash dispatcher (`scripts/chassis/chassis`)
via `python3 "$SCRIPT_DIR/<name>.py"`. A growing set has neither inbound
imports nor a dispatcher case branch — it is dead code, but the filesystem
layout cannot tell the three classes apart.

The chassis enforces decomposition rules on consumer codebases via the
`completeness`, `decomposition`, and `surface-wiring` gates, but no gate
enforces that every module under `scripts/chassis/` is reachable from at
least one entry point. The result is a slow accumulation of unreferenced
`.py` files that cannot be safely deleted because no tool can prove they
are unreached.

## Decision

Every `*.py` file at `scripts/chassis/*.py` (top level, excluding
subdirectories) must satisfy at least one of:

1. Be imported by another in-repo Python module (`from <module> import ...`
   or `import <module>`).
2. Be invoked as a CLI entry point from `scripts/chassis/chassis` (the bash
   dispatcher; case branches and `python3 ... "$SCRIPT_DIR/<file>.py"`
   patterns are the source of truth).
3. Appear in the `[allowlist]` section of `config/chassis/module-wiring.toml`
   under either `keep` (intentional entry not yet wired) or `pending_removal`
   (scheduled for removal with a `since` date and a `remove_by` date).

A new gate at `scripts/chassis/gates/module_wiring.py` enforces this. It
parses the dispatcher with regex over `case "<name>")` blocks, collects the
inbound import graph via `ast.parse` over each module, and emits one of the
three rule IDs per finding.

The gate's day-1 baseline is `config/chassis/module-wiring.toml` seeded with
the post-Tranche-3 unwired set, each tagged `pending_removal` with
`remove_by = "2026-Q3"`. Hard-fails apply only to genuinely new dead modules
outside the allowlist.

## Consequences

- Adding a new top-level Python file to `scripts/chassis/` requires either
  importing it from somewhere or wiring it into the dispatcher in the same
  PR. The CI gate enforces this.
- Modules slated for removal become first-class metadata: the
  `pending_removal` table records `since`, `remove_by`, and rationale, and
  the gate warns when the date passes.
- Duplicate dispatcher entries are caught early; today they are silent.
- The bash dispatcher becomes an authoritative surface — agent edits that
  drift from it are visible.

## Alternatives considered

- **A pyproject-style `[project.scripts]` registry.** Rejected: chassis
  ships as a script-first tool with a bash dispatcher, not a console-script
  package. Switching to entry points would conflate library and CLI
  distribution and is a larger structural change with its own ADR.
- **Static analysis only (no allowlist).** Rejected: some genuine entry
  points are reached via documentation examples or external CI scripts that
  the gate cannot statically discover. The `keep` allowlist is a typed
  escape hatch with explicit ownership.

## References

- ADR-0006 (codegen and generated-artifact policy): adjacent decomposition
  enforcement on the generated-artifact surface.
- ADR-0011 (ruleId stability discipline): every ruleId emitted by this gate
  must resolve here.
- `scripts/chassis/gates/_common.py`: `Finding` and emission helpers.
- `scripts/chassis/repo_layout.py`: reused for repo-root resolution.
- `config/chassis/loc-caps.toml`: shape precedent for
  `module-wiring.toml`.

## Status

Accepted. Gate implementation lands after Tranche 3 of the chassis self-
governance hardening plan (Gate B).
