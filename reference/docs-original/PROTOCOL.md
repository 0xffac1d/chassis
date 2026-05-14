# Chassis protocol versioning

This document describes **repository configuration and metadata evolution**, not the semver of published npm packages.

## `config/chassis.config.yaml` — `version`

The top-level `version` field (e.g. `1.0.0`) is the **config document revision** for that file. It is **not** currently enforced against the `chassis` CLI script version. Future tooling may warn on mismatch.

When breaking changes are introduced to **required** config keys or their meaning, bump this version and document the change in the changelog section below.

## `project.language` and `project.languages`

- **`project.language`:** Primary language hint for humans, templates, and future tooling (defaults such as codegen language). It does **not** override per-manifest `drift.language`.
- **`project.languages`:** Optional list of additional languages present in the repo (informational; supports polyglot documentation). Empty or omitted means “unspecified”.

Per-module **`drift.language`** (`auto` | `typescript` | `rust` | `csharp`) controls export scanning for `chassis drift`.

## Metadata schema evolution

`CONTRACT.yaml` and `chassis.unit.yaml` validate against JSON Schemas under `schemas/metadata/`. Breaking changes to those schemas (new required fields, removed keys, enum changes) should be:

1. Described in git history and release notes for consumers who copy `schemas/metadata/`.
2. Accompanied by template updates under `templates/metadata/`.

Use **`chassis contract-diff`** to compare two contract files for breaking vs non-breaking manifest changes between revisions.

## Changelog (config / protocol hints)

| Config `version` | Notes |
|------------------|--------|
| 1.0.0 | Initial documented fields; `project.languages` optional array added for polyglot repos. |

## Source-of-truth precedence

The following **source-of-truth precedence** model applies when `CONTRACT.yaml`, tests, and source code disagree about a module's behavior. Chassis tooling and AI agents must use this order:

1. **Runtime behavior** — What the code actually does under test is ground truth.
2. **Passing tests** — Executable assertions about behavior. A test that asserts `f(x) === y` outranks any claim in `CONTRACT.yaml` that says otherwise.
3. **`CONTRACT.yaml` invariants** — Stated constraints, valid as specifications but not verified until linked via `test_linkage`. Treat as specifications-under-review until `status: stable` and `test_linkage` is non-empty.
4. **`DECISIONS.md`** — Design rationale and context. Informational only — never treat `DECISIONS.md` prose as behavioral constraints.

### Architecture topology

For **cross-module boundaries, workflows, slots, and state ownership**, the **architecture intermediate representation (IR)** under [`schemas/architecture/`](../../schemas/architecture/) is the normative, language-agnostic model when present (see [`guides/architecture-ir.md`](guides/architecture-ir.md)). Module manifests may **project** that IR into `purpose`, dependencies, and related fields; local-only manifest prose without IR linkage is **lower authority** than reviewed IR for architectural claims.

### Implications for AI agents

- An AI agent reading `agent-context` output must treat `invariants` as specifications that MAY differ from runtime behavior, not as verified facts.
- When `test_linkage` is present with `confidence: high`, treat the linked invariant as verified.
- When `inference.confidence: low` is present, treat ALL invariants as unverified placeholders.
- `DECISIONS.md` content in the agent-context bundle is labeled as "narrative context" — agents must not derive behavioral constraints from it.

### Conflict resolution

When conflict is detected between test assertions and CONTRACT invariants, the recommended resolution order is:

1. Check whether the test is testing the right thing (tests can be wrong).
2. If the test is correct, update `CONTRACT.yaml` to match observed behavior.
3. If `CONTRACT.yaml` is the intent, fix the code to match the invariant.
4. If the invariant is aspirational (future requirement), move it to `todos` with priority and description.

Never leave a known conflict unresolved in a `status: stable` contract.

### `superseded_by` field

When a module is replaced by another, set `status: superseded` and fill `superseded_by` with the path or name of the replacement. Tooling will warn when a `superseded` contract is referenced by other modules' `depends_on`.

## See also

- **Evolution backlog** — [`ROADMAP.md`](ROADMAP.md) (P0/P1/P2: objectives, stable claim IDs, assurance ladder, schema/support-matrix work).
