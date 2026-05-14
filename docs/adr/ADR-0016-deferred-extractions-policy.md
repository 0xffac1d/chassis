---
id: ADR-0016
title: "Deferred extractions — reference-only staging with promotion criteria"
status: accepted
date: "2026-05-14"
enforces:
  - rule: DEFERRAL-LIVES-IN-REFERENCE
    description: "Non-shipping deferred artifacts remain under reference/ with explanatory README pointers."
  - rule: DEFERRAL-HAS-PROMOTION-CRITERIA
    description: "Each deferred bucket documents predecessor context + criteria for promotion into supported fixtures/tooling."
applies_to:
  - "reference/**"
  - "fixtures/**"
tags:
  - foundation
  - process
---

## Context

The salvage intentionally parked unsupported fixtures, Python CLIs, and extended schemas outside the supported kernel. `reference/adrs-original/ADR-0016-deferred-extractions.md` tracked module extraction from a monolithic workspace — informative historically but mismatched to this repository’s smaller blast radius. We still need an explicit **deferral policy** so contributors know where unfinished work belongs.

## Decision

### Reference-only deferrals

Items that intentionally lack validators/consumers MUST live under `reference/` with:

1. A **README** naming the predecessor artifact (path + purpose).
2. Forward pointers to the ADR(s) or waves responsible for promotion.
3. Clear language that paths are **non-canonical** until moved.

Promoting something to `fixtures/` or production crates/packages requires implementing the corresponding verifier or marking the fixture `happy-path`.

### Exemption registry linkage

When deferrals imply governance debt (e.g., illegal layouts tolerated temporarily), authors MUST file exemptions per ADR-0004 once tooling exists; Wave 1 only documents intent inside README references.

### Promotion criteria template

Each deferred directory SHOULD enumerate:

- **Gate:** what validator or CLI command must exist.
- **Owner:** team/person responsible.
- **Exit:** observable condition for graduation (e.g., layout validator merged + fixture migrated).

## Audit (2026-05-14)

| Path | Status | Notes |
|------|--------|-------|
| `reference/fixtures-deferred/` (`illegal-layout`, `brownfield-messy`) | Deferred | README explains missing layout/bootstrap tooling — keep until validators ship. |
| `reference/python-cli/` (incl. `mcp_server.py`) | Reference semantic spec | Python runtime not supported per ADR-0001 — TS MCP wave must supersede. |
| `reference/schemas-extended/` | Design input | Kind slices informed contract tightening; not emitted into dist schemas wholesale. |
| `reference/docs-original/` + `reference/adrs-original/` | Historical | Process docs + legacy ADRs — never authoritative without re-authorship in `docs/adr/`. |
| Assurance rungs `coherent`–`observed` | Deferred | Documented in ADR-0002 ordering; no verifier committed yet. |

## Consequences

- Prevents “phantom fixtures” from polluting supported matrices.
- Makes roadmap ordering explicit for reviewers.

## Relationship to predecessor

Replaced extraction tables with repository-local deferrals consistent with salvage boundaries.
