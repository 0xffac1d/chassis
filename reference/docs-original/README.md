# Original documentation from the prior Chassis project

These five files describe the **prior** Chassis project — the codebase this repo was extracted from. They are reference material only; they do not describe how this project works or what binds it. Do not rely on any command, schema path, configuration key, or decision in these files without verifying it against the current tree.

- **AGENTS.original.md** — Canonical AI-agent instructions for the original Chassis. Documents commands (`chassis validate-all`, `release-gate`, etc.) and ~50 guides that no longer exist here.
- **DECISIONS.original.md** — Distribution-level design rationale: standalone vs full-product scope split, codegen tree policy, baseline philosophy. Useful background; this project's scope is narrower (Rust + TypeScript only, see ADR-0001).
- **PROTOCOL.original.md** — Protocol versioning and a **source-of-truth precedence model** (runtime behavior > passing tests > CONTRACT invariants > DECISIONS prose). The precedence model is worth re-authoring as a new doc once equivalent infrastructure exists here.
- **OBJECTIVES-REGISTRY.original.md** — How the original objective registry (`config/chassis.objectives.yaml`) worked. No equivalent in this project yet.
- **ROADMAP.original.md** — The original assurance-ladder roadmap. Documents the `declared → coherent → verified → enforced → observed` ladder that this project inherits (see `docs/ASSURANCE-LADDER.md` and ADR-0001).

For the current project's docs, see `docs/` at the repo root.
