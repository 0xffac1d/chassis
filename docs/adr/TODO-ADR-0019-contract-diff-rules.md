---
id: TODO-ADR-0019
title: "Contract-diff rule IDs (CH-DIFF-*) — Wave 2 stub for close-out promotion"
status: stub
date: "2026-05-14"
tags:
  - wave-2
  - diff
  - diagnostics
  - todo
---

## Status

**Stub.** This file enumerates the rule IDs emitted by
`crates/chassis-core/src/diff/` so the diff implementation can land in
Wave 2 Session D without blocking on ADR authorship. Promote to a full
`ADR-0019-contract-diff-rules.md` (with normative `enforces[]` frontmatter,
"Decision", "Consequences", "Relationship to predecessor" sections) during
the Wave 2 close-out review, after Sessions A / C / D have all merged.

Until promotion this file is the authoritative reference for diagnostics
emitted by `chassis-core::diff`. The diagnostics already cite `ADR-0019`
in their `violated.convention` envelope field — promoting this stub keeps
that linkage truthful.

## Context

Wave 2 introduces the contract-diff engine in `chassis-core::diff`. Per
ADR-0011 every emitted rule ID MUST resolve to an accepted ADR's
`enforces[]`. The engine emits 19 distinct rule IDs; this stub registers
them so the implementation, fixtures, and diagnostic envelope conformance
tests are all coherent against a stable surface.

Diagnostic envelope shape is fixed by ADR-0018
(`schemas/diagnostic.schema.json`). Each emitted finding sets:

- `ruleId` = one of the IDs below
- `severity` = `error` | `warning` | `info` (ADR-0018 envelope)
- `source` = `chassis diff`
- `subject` = scoped subject string (e.g. `contract<name>.invariants.<id>`)
- `violated.convention` = `ADR-0019`
- `detail.classification` = `breaking` | `non-breaking` | `additive`
  (diff-specific, distinct from envelope severity)

## Rule IDs

| Rule ID | Envelope severity | Classification | Summary |
|---------|-------------------|----------------|---------|
| `CH-DIFF-KIND-CHANGED` | `error` | breaking | Contract `kind` changed (e.g. `library` → `service`). Kind-specific schemas don't overlap; downstream consumers cannot reuse the old kind's projections. |
| `CH-DIFF-NAME-CHANGED` | `error` | breaking | Contract `name` changed. Consumers cannot track this contract across the rename without out-of-band reconciliation. |
| `CH-DIFF-VERSION-MISSING` | `error` | breaking | New contract lacks `version` (ADR-0008 violation). Reachable only when the caller bypassed schema validation; covered by unit test rather than a fixture. |
| `CH-DIFF-VERSION-NOT-BUMPED` | `error` | breaking | Breaking change detected but `version` unchanged across old/new. |
| `CH-DIFF-VERSION-DOWNGRADED` | `error` | breaking | New `version` is semver-less-than old `version`. |
| `CH-DIFF-VERSION-MAJOR-WITHOUT-BREAKING` | `warning` | non-breaking | Major version bumped but no breaking change detected. Signals possible over-bumping; non-blocking. |
| `CH-DIFF-VERSION-BREAKING-WITHOUT-MAJOR` | `error` | breaking | Breaking change detected but only minor or patch was bumped. |
| `CH-DIFF-CLAIM-REMOVED` | `error` | breaking | An `invariants[].id` or `edge_cases[].id` present in old is absent in new. |
| `CH-DIFF-CLAIM-ID-CHANGED` | `error` | breaking | Reserved. In practice, an id rename surfaces as `CH-DIFF-CLAIM-REMOVED` for the old id plus `CH-DIFF-CLAIM-ADDED` for the new id — the engine does not emit this rule directly. Reserved so future heuristic detection of renames can introduce it without consuming a new identifier. |
| `CH-DIFF-CLAIM-TEXT-CHANGED` | `warning` | non-breaking | Same claim `id`, different `text`. Trace tooling keys on ids; text edits are documentation drift. |
| `CH-DIFF-CLAIM-ADDED` | `info` | additive | New `invariants[].id` or `edge_cases[].id` in new not present in old. |
| `CH-DIFF-INVARIANT-DEMOTED-TO-EDGE-CASE` | `error` | breaking | A claim `id` moved from `invariants` to `edge_cases`. Per ADR-0003 invariants are fail-closed and edge_cases are bounded — demotion is a weaker guarantee. |
| `CH-DIFF-EDGE-CASE-PROMOTED-TO-INVARIANT` | `warning` | non-breaking | Claim id moved from `edge_cases` to `invariants` (stronger guarantee). |
| `CH-DIFF-ASSURANCE-DEMOTED` | `error` | breaking | `assurance_level` moved down the ADR-0002 ladder (`observed > enforced > verified > coherent > declared`). |
| `CH-DIFF-ASSURANCE-PROMOTED` | `warning` | non-breaking | `assurance_level` moved up the ladder. |
| `CH-DIFF-STATUS-CHANGED` | `warning` | non-breaking | `status` field changed. Semantics (e.g. `stable` → `deprecated`) are left to consumers. |
| `CH-DIFF-OWNER-CHANGED` | `warning` | non-breaking | `owner` reassigned. Audit trail signal. |
| `CH-DIFF-REQUIRED-KIND-FIELD-REMOVED` | `error` | breaking | When `kind` is unchanged, a field that the kind's `schemas/contract.schema.json` `oneOf` branch lists as required is present in old but absent in new. Reachable only when the caller bypassed schema validation; covered by unit test rather than a fixture. |
| `CH-DIFF-PARSE-ERROR` | n/a | n/a | Surfaced via `DiffError::Parse` rather than as a `Diagnostic`. Returned when either input is not a recognizable Contract shape (empty object, non-object root). |

## Fixture coverage

Fixtures under `fixtures/diff/` cover 17 of the 19 rule IDs:

- Each `breaking-*`, `nonbreaking-*`, `additive-*`, `warning-*` directory
  exercises one primary rule. The `breaking-claim-id-renamed/` fixture
  encodes the documented decomposition: it asserts a renamed id surfaces
  as REMOVED + ADDED rather than as `CH-DIFF-CLAIM-ID-CHANGED`.
- `parse-error/` exercises `CH-DIFF-PARSE-ERROR` via `DiffError::Parse`.
- `CH-DIFF-VERSION-MISSING` and `CH-DIFF-REQUIRED-KIND-FIELD-REMOVED`
  cannot be authored as fixtures: both rules describe input shapes that
  the canonical contract schema rejects, and the fixture sweep verifies
  each `old.yaml` / `new.yaml` is independently schema-valid. They are
  covered by unit tests inside `crates/chassis-core/src/diff/tests.rs`.

## Out of scope (deferred / not emitted)

- `CH-DIFF-ENUM-NARROWED` — operates on schemas, not instances. Deferred
  to a future schema-vs-schema diff pass (likely paired with ADR-0008
  CI enforcement).
- Recursive diff into kind-specific subtrees (`endpoint.request`,
  `entity.fields`, etc.). Scoped out for this session; Wave 2.5 / deeper
  kind subschemas will introduce additional rule IDs under the same
  `CH-DIFF-*` prefix.

## Promotion checklist (for close-out)

When promoting this stub to `ADR-0019-contract-diff-rules.md`:

1. Rename file to `ADR-0019-contract-diff-rules.md`.
2. Replace `status: stub` with `status: accepted`.
3. Add an `enforces[]` array citing each rule ID with its description
   (mirror the table above).
4. Verify every `ruleId` emitted by `chassis-core::diff` resolves to an
   `enforces[].rule` entry.
5. Re-run `cargo test -p chassis-core` — `violated.convention` envelope
   field already references `ADR-0019`, so this becomes valid once the
   ADR is accepted.
