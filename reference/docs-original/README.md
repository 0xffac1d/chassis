# Historical and reference documentation

Reference material only — these files do not describe how this project works or what binds it. Do not rely on any command, schema path, configuration key, or decision here without verifying it against the current tree. To adopt an idea, re-author it as a fresh doc in `docs/` (and as an ADR in `docs/adr/` if it's a decision).

## From the prior Chassis project

The codebase this repo was extracted from.

- **AGENTS.original.md** — Canonical AI-agent instructions for the original Chassis. Documents commands (`chassis validate-all`, `release-gate`, etc.) and ~50 guides that no longer exist here.
- **DECISIONS.original.md** — Distribution-level design rationale: standalone vs full-product scope split, codegen tree policy, baseline philosophy. Useful background; this project's scope is narrower (Rust + TypeScript only, see ADR-0001).
- **PROTOCOL.original.md** — Protocol versioning and a **source-of-truth precedence model** (runtime behavior > passing tests > CONTRACT invariants > DECISIONS prose). The precedence model is worth re-authoring as a new doc once equivalent infrastructure exists here.
- **OBJECTIVES-REGISTRY.original.md** — How the original objective registry (`config/chassis.objectives.yaml`) worked. No equivalent in this project yet.
- **ROADMAP.original.md** — The original assurance-ladder roadmap. Documents the `declared → coherent → verified → enforced → observed` ladder that this project inherits (see `docs/ASSURANCE-LADDER.md` and ADR-0001).

## Process notes from this project's setup

Snapshots of one-time work; not authoritative going forward.

- **HISTORY.md** — Narrative of how the tree got into its current shape (salvage extraction + compile-blocker fixup). Describes what was done; not a backlog.
- **CONTRACT-SCHEMA-LOOSENESS-SURVEY.md** — Survey of `schemas/contract.schema.json` looseness. Design input for the planned `kind`-discriminated tightening using subschemas in `reference/schemas-extended/`.

For the current project's docs, see `docs/` at the repo root. Mirrors `reference/adrs-original/` — same authoritative-vs-reference semantics.
