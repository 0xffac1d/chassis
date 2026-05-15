---
id: ADR-0019
title: "Contract-diff rule IDs (CH-DIFF-*) — stable diagnostics for chassis-core::diff"
status: accepted
date: "2026-05-14"
supersedes: []
enforces:
  - rule: CH-DIFF-KIND-CHANGED
    description: "Breaking. Contract kind changed; kind-specific schemas do not overlap."
  - rule: CH-DIFF-NAME-CHANGED
    description: "Breaking. Contract name changed; consumers cannot track identity across rename."
  - rule: CH-DIFF-VERSION-MISSING
    description: "Breaking. New contract lacks parseable version (ADR-0008); caller bypassed validation."
  - rule: CH-DIFF-VERSION-NOT-BUMPED
    description: "Breaking. Breaking change detected but semver unchanged."
  - rule: CH-DIFF-VERSION-DOWNGRADED
    description: "Breaking. New version is semver-less-than old version."
  - rule: CH-DIFF-VERSION-MAJOR-WITHOUT-BREAKING
    description: "Non-breaking envelope severity (warning). Major bumped without detected breaking delta."
  - rule: CH-DIFF-VERSION-BREAKING-WITHOUT-MAJOR
    description: "Breaking. Breaking delta detected but bump is not major (ADR-0008)."
  - rule: CH-DIFF-CLAIM-REMOVED
    description: "Breaking. Claim id present in old absent in new (ADR-0003 identity)."
  - rule: CH-DIFF-CLAIM-ID-CHANGED
    description: "Reserved. Id renames surface as REMOVED + ADDED; reserved for future rename heuristics."
  - rule: CH-DIFF-CLAIM-TEXT-CHANGED
    description: "Non-breaking. Same claim id, different text (documentation drift)."
  - rule: CH-DIFF-CLAIM-ADDED
    description: "Additive. New claim id in new not present in old."
  - rule: CH-DIFF-INVARIANT-DEMOTED-TO-EDGE-CASE
    description: "Breaking. Claim moved from invariants to edge_cases (weaker guarantee per ADR-0003)."
  - rule: CH-DIFF-EDGE-CASE-PROMOTED-TO-INVARIANT
    description: "Non-breaking. Claim moved from edge_cases to invariants."
  - rule: CH-DIFF-ASSURANCE-DEMOTED
    description: "Breaking. assurance_level moved down the ADR-0002 ladder."
  - rule: CH-DIFF-ASSURANCE-PROMOTED
    description: "Non-breaking. assurance_level moved up the ladder."
  - rule: CH-DIFF-STATUS-CHANGED
    description: "Non-breaking. status field changed; semantics left to consumers."
  - rule: CH-DIFF-OWNER-CHANGED
    description: "Non-breaking. owner reassigned."
  - rule: CH-DIFF-REQUIRED-KIND-FIELD-REMOVED
    description: "Breaking. Kind-required field present in old absent in new when kind matches."
applies_to:
  - "crates/chassis-core/src/diff/**"
  - "fixtures/diff/**"
  - "schemas/contract.schema.json"
tags:
  - wave-2
  - diff
  - diagnostics
---

## Context

Wave 2 Session D landed the contract-diff engine in `crates/chassis-core/src/diff/`. Per ADR-0011, every emitted `ruleId` must resolve to an accepted ADR `enforces[]` entry. Rule IDs were enumerated in a temporary stub during parallel development; this ADR promotes that list to normative form.

The root self-governance claim `chassis.no-silent-assurance-demotion` binds this ADR back to `CONTRACT.yaml`: demoting `assurance_level` must emit `CH-DIFF-ASSURANCE-DEMOTED`, never disappear as prose-only drift.

Finding shape follows ADR-0018 (`schemas/diagnostic.schema.json`): each diagnostic sets `ruleId`, envelope `severity` (`error` \| `warning` \| `info`), `source` (`chassis diff`), `subject`, optional `violated.convention` (`ADR-0019`), and `detail.classification` (`breaking` \| `non-breaking` \| `additive`) as the diff-domain semantic distinct from envelope severity.

## Decision

The eighteen `CH-DIFF-*` rule IDs in `enforces[]` are **immutable** per ADR-0011. Classifications:

- **Breaking** findings use envelope **error** except where noted (e.g. version major-without-breaking uses **warning** with non-breaking classification).
- **Non-breaking** findings use envelope **warning**.
- **Additive** findings use envelope **info**.

Cross-references:

- **ADR-0008** — semver semantics for version-related rules (`CH-DIFF-VERSION-*`).
- **ADR-0002** — assurance ladder ordering for `CH-DIFF-ASSURANCE-*`.
- **ADR-0003** — claim identity and bucket semantics for `CH-DIFF-CLAIM-*` and invariant/edge-case promotion rules.

`CH-DIFF-PARSE-ERROR` is **not** an ADR-bound diagnostic `ruleId` on the wire: parse failures return `DiffError::Parse` instead of a `Diagnostic` row.

## Consequences

Downstream consumers (Wave 3 trace graph, Wave 4 CLI) may route on these stable IDs without parsing `message` text. New diff rules require **new** identifiers and an ADR amendment or superseder — never reuse or redefine an existing token.

## Relationship to predecessor

None. New ADR.
