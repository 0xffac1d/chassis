# Chassis Spec Kit preset

This document defines the **Chassis Spec Kit bridge**: how Markdown-first Spec Kit
documents map to deterministic, machine-readable **`artifacts/spec-index.json`**
that Chassis validates (`schemas/spec-index.schema.json`), exports into policy
input, and links against **`CONTRACT.yaml`** claim IDs.

## Goals

- **No ambiguous Markdown parsing in CI**: the preset records intent in YAML
  (or split Markdown with documented anchors); the canonical consumer is JSON
  produced by `chassis spec-index export`.
- **Fail closed**: duplicate IDs, missing acceptance criteria, empty claim bindings
  where required, or invalid task dependency edges abort export with structured
  errors before any policy evaluation.

## Directory layout

Recommended repository layout (names are conventional, not hard-coded):

| Path | Role |
|------|------|
| `spec/constitution.md` | Principles (human narrative) |
| `spec/spec.md` | Feature intent, requirements |
| `spec/plan.md` | Plan / sequencing |
| `spec/tasks.md` | Task checklist |
| `.chassis/spec-index-source.yaml` | **Authoritative machine source** merged from the above (copy/paste or generator) |

The exporter reads **one** YAML file (default: `.chassis/spec-index-source.yaml`)
and writes **`artifacts/spec-index.json`**.

## YAML source format

The YAML mirrors the JSON schema (`spec-index.schema.json`) using **snake_case**
keys. Required top-level fields:

| Field | Description |
|-------|-------------|
| `version` | Must be `1` (wire version). |
| `chassis_preset_version` | Preset revision; must be `1` for this wave. |
| `feature_id` | Stable identifier (`^[A-Za-z][A-Za-z0-9._-]*$`). |
| `constitution_principles` | `{ id, text }[]` — principle IDs unique, uppercase `^[A-Z][A-Z0-9_-]*$`. |
| `non_goals` | Explicit exclusions. |
| `requirements` | Structured requirements (see below). |
| `tasks` | Executable tasks with `depends_on` edges. |
| `implementation_constraints` | Repo/tooling constraints. |

Optional: `title`, `summary`.

### Requirements

Each requirement **must** include:

| Field | Description |
|-------|-------------|
| `id` | Stable ID (e.g. `REQ-001`). Unique across requirements. |
| `title` | Short title. |
| `description` | Full text. |
| `acceptance_criteria` | Non-empty list of strings (fail closed if empty). |
| `claim_ids` | Chassis **contract claim IDs** this requirement binds to. Empty list is a **linker error** (unbound requirement). |
| `related_task_ids` | Task IDs implementing this requirement (each ID must exist in `tasks[]`). |
| `touched_paths` | Repo-relative paths; each path must either appear as a traced impl/test site for the linked claims or exactly match a path listed in any CONTRACT `exports[].path` entry. |

### Tasks

| Field | Description |
|-------|-------------|
| `id` | Unique task ID (e.g. `TASK-001`). |
| `title` | Short title. |
| `description` | Optional detail. |
| `depends_on` | Other task IDs (dependency edges). Every ID must exist. |
| `parallel_group` | Optional string grouping parallel work. |
| `touched_paths` | Paths this task touches. |

**Orphan tasks**: every task ID must appear in the union of all
`requirements[].related_task_ids`. If you introduce a task before assigning it,
add it to a requirement’s `related_task_ids` or remove the task until it is
planned.

## Markdown anchors (documentation-only)

When keeping narrative in Markdown alongside YAML, use stable headings so humans
and tooling can cross-reference:

- Requirements: `### REQ-001 Title` with HTML anchor `#req-001` (GitHub slug rules).
- Tasks: `- [ ] TASK-001 — description` under `## Tasks`.

The **exporter does not** parse these files in v1; maintainers sync into
`.chassis/spec-index-source.yaml` (or a future merged generator).

## Determinism

`chassis spec-index export` canonicalizes and sorts arrays (by `id` where
appropriate) before writing JSON. The **spec index digest** in policy input is
SHA-256 of the canonical UTF-8 JSON (64 lowercase hex characters).

## Related commands

- `chassis spec-index export --from <yaml> --out artifacts/spec-index.json`
- `chassis spec-index validate artifacts/spec-index.json`
- `chassis spec-index link --index artifacts/spec-index.json` (joins spec ↔ repo; pass `--repo`)

Release gate and `chassis export` pick up **`artifacts/spec-index.json`** when
present, validate it, compute the digest for policy input, and merge linker
diagnostics into the export/gate outcome.

## Linker semantics

The linker checks **contract bindings** (claim IDs exist in `CONTRACT.yaml` entries),
**task wiring** (`related_task_ids` and `depends_on` refer to real tasks), and
**trace evidence**: every bound claim must have at least one **implementation** and
one **test** site in the trace graph. It also validates **`touched_paths`** using
the conservative coverage rule described in the requirements/tasks tables above.
Orphan tasks (tasks not referenced by any requirement) are reported at info severity.

See **ADR-0026** for diagnostic rule IDs and governance.
