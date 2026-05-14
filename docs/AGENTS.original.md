# AGENTS.md — Chassis standalone distribution

Canonical instructions for AI agents (Claude Code, Cursor, Windsurf, Copilot, Aider, Continue) working in this repository. Derivative formats (`CLAUDE.md`, `.cursor/rules/`, `.windsurfrules`, `.github/copilot-instructions.md`, `.aider.conf.yml`, `.continue/rules/`) are **generated** from this file via `chassis emit agent-surface`. Do not hand-edit the derivatives — CI will revert drift.

## What this repo is

The canonical home for **Chassis**: a metadata-governance substrate for deterministic software generation. It ships JSON Schemas, validators, a CLI (`scripts/chassis/chassis`), the `chassis-schemas` Rust crate, and ~50 guides. It does **not** ship any application engine, dashboard, or runtime services — those live in downstream consumer repositories. See `REPO_BOUNDARY.md`.

## Primary commands

```bash
./scripts/chassis/chassis validate                   # CONTRACT.yaml / chassis.unit.yaml vs JSON Schema
./scripts/chassis/chassis validate-all               # distribution-layout (if guide present) + validate + validate-domain + validate-objectives
./scripts/chassis/chassis validate-objectives        # objective registry + linked_objectives (skipped if no registry)
./scripts/chassis/chassis validate-distribution-layout  # standalone layout guard
./scripts/chassis/chassis drift                      # export / freshness / docs drift
./scripts/chassis/chassis coherence --format json    # repository coherence report
./scripts/chassis/chassis release-gate               # expansion gate suite (19 gates; see guides/expansion-pack.md)
./scripts/chassis/chassis adr validate               # ADR frontmatter + supersedes chain
./scripts/chassis/chassis adr index                  # regenerate docs/index.json
./scripts/chassis/chassis exempt check               # CI gate for .exemptions/registry.yaml
./scripts/chassis/chassis codegen --lang <tgt>       # Rust / TS / Python / Go / C# emitters
```

## Project structure

| Path | Role |
|------|------|
| `schemas/` | JSON Schemas — metadata, domain, architecture, coherence, structure, diagnostics, decision, exemption |
| `templates/` | `chassis init` scaffolds + consumer-scope CI templates (`templates/ci/gate-configs/`) |
| `blueprints/` | Multi-artifact composition specs |
| `crates/chassis-schemas/` | **Committed** generated Rust crate — the canonical in-repo binding tree |
| `generated/` | On-demand TS/Python/C#/Go/TypeSpec codegen output — **not committed** (see `.gitignore`) |
| `codegen/ts-types/` | Committed `@chassis/types` npm package (`dist/` + fingerprint) — see `REPO_BOUNDARY.md` |
| `config/` | `chassis.config.yaml`, orphans policy, gate baselines, posture, proof-paths |
| `docs/chassis/guides/` | ~50 guides; `docs/chassis/AGENTS.md` is the docs-tree index, this file is the root canonical |
| `docs/adr/` | Architecture Decision Records (ADRs) that authoritatively define rule IDs |
| `scripts/chassis/` | Python modules + bash dispatcher for the CLI |
| `scripts/chassis/gates/` | Expansion gates + `release_gate.py` orchestrator (19 gates incl. quarterly-review); see guides/expansion-pack.md |
| `.exemptions/` | CODEOWNERS-protected waiver registry (90-day max lifetime, 25-entry cap) |

## Conventions

### Stable IDs everywhere

- **Rule IDs** (diagnostic emissions) match `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$`. Every diagnostic `ruleId` must resolve to an ADR's `enforces[].rule`. The binding-link gate enforces this.
- **Claim IDs** on `CONTRACT.yaml` invariants / edge_cases use kebab-case snake_case (`^[a-z][a-z0-9_.-]*$`) and flow into `test_linkage.claim_id`.
- **ADR IDs** format `ADR-NNNN` with 4+ digits.
- **Exemption IDs** format `EX-YYYY-NNNN` (year + counter).

### CONTRACT.yaml

- Use the **structured `{id, text}` form** for invariants and edge_cases. String-only form is legacy and deprecated.
- `test_linkage[].claim_id` references the structured ID (not prose excerpts).
- `assurance_level` advances along the ladder: `declared → coherent → verified → enforced → observed`.

## Chassis metadata operations

All metadata authoring (`CONTRACT.yaml`, `CRATE.md`, `DECISIONS.md`, `ADR-NNNN`, `capability.fingerprint.json` sidecars) goes through `chassis scaffold`. The full pattern is documented in [`skills/chassis-metadata/SKILL.md`](skills/chassis-metadata/SKILL.md). Do not hand-craft these files; the scaffold tool produces canonical output and emits placeholder markers for fields requiring authorial judgment. After any metadata change, run `chassis validate --changed HEAD`.

- Read [`skills/chassis-metadata/EXAMPLES.md`](skills/chassis-metadata/EXAMPLES.md) for worked metadata scenarios before adding crates, ADRs, invariants, decisions, projections, or capability fingerprints.

### Diagnostic contract (Milestone A)

Every gate's `--json` output emits findings that validate against `schemas/diagnostics/diagnostic.schema.json`: `ruleId`, `severity`, `message`, optional `violated.convention` (ADR link), `docs`, `fix.applicability`, `location.range`. Use `Finding.to_diagnostic()` in `_common.py`.

### Generated artifacts

- **Committed canonical generated (class 1):** `crates/chassis-schemas/` (Rust) and `codegen/ts-types/dist/` + `codegen/ts-types/fingerprint.sha256` (`@chassis/types`). Regenerate via `chassis codegen --check --lang rust` and the `ts-types-build` CI job / local npm build.
- **On-demand, never commit (class 2):** `generated/{typescript,python,csharp,go,typespec}/` — output of `chassis codegen --lang <lang>`.
- Never commit a `runtime/` application tree in this repo — consumer-repo territory.

### Gate scope split

- `config/chassis/*.toml` = standalone-distribution scope. Minimal, targets only what this repo owns.
- `templates/ci/gate-configs/*.consumer.*` = full-product scope. Shipped for downstream consumer repos.
- When you change a standalone config, consider whether the consumer template needs the same change.

### Exemptions

- Every `// eslint-disable-next-line`, `#[allow(...)]`, or inline suppression must reference an `EX-YYYY-NNNN` entry in `.exemptions/registry.yaml`.
- Adding an exemption: `chassis exempt add --rule <ID> --scope <path> --reason "<40+ chars>" --ticket <ref> --owner <email> --adr ADR-NNNN`.
- Hard quota: 25 total, 1 per file, 90-day maximum lifetime.

## Boundaries — what to NOT do

- Do not create `docs/guides/` at repo root. Guides live under `docs/chassis/guides/` (enforced by `validate-distribution-layout`).
- Do not duplicate JSON Schemas under `crates/chassis-schemas/schemas/`. The crate embeds with `include_str!` from the repo-root `schemas/` tree.
- Do not add product-runtime references to standalone configs (`crates/agent/`, `docs/security/`, `scripts/ci/check-route-alignment.sh`). Move such references to the `*.consumer.*` template.
- Do not introduce new `jsonschema.RefResolver` imports. Use `scripts/chassis/jsonschema_support.py::draft7_validator_for_schema_file`.
- When using objectives, add `linked_objectives` on `CONTRACT.yaml` only with ids that resolve in `config/chassis.objectives.yaml` (see `chassis validate-objectives` and `guides/objectives-registry.md`).
- Do not write new inline suppression comments (`// eslint-disable`, `#[allow(...)]`) without a paired exemption entry.

## Security

- `config/chassis.config.yaml` is the target-repo marker; `--repo-root <path>` (or `CHASSIS_TARGET_REPO_ROOT`) overrides it, and `CHASSIS_ROOT` points at the Chassis distribution (schemas, templates, CLI). `CHASSIS_REPO_ROOT` is still accepted as a legacy alias.
- The CLI does not execute network calls or fetch remote schemas — every `$ref` resolves via the `referencing` library against repo-local files.
- `schemas/diagnostics/diagnostic.schema.json` is the machine contract for agent-routable output. Do not emit findings outside the schema.

## Where to find more

- Repo boundary: [`REPO_BOUNDARY.md`](REPO_BOUNDARY.md)
- Canonical CLAUDE.md router: [`CLAUDE.md`](CLAUDE.md) (imports this file)
- ADR registry: [`docs/index.json`](docs/index.json) (regenerate: `chassis adr index`)
