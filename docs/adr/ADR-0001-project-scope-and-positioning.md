---
id: ADR-0001
title: "Project scope, positioning, and salvage boundary"
status: accepted
date: "2026-05-14"
enforces:
  - rule: SCOPE-LANGUAGE-PAIR
    description: "First-class support is limited to Rust and TypeScript."
  - rule: SCOPE-POSITION-COMPLEMENT
    description: "This project positions as a complement to GitHub Spec Kit, not a competitor. The differentiator is verifiable adherence (trace graph, drift detection, attestation), not intent capture."
  - rule: SCOPE-NO-RUNTIME-CLAIMS
    description: "Until a runtime crate enforces something concrete (admission control or equivalent), the project does not use the word 'runtime' in user-facing copy."
applies_to:
  - "crates/**"
  - "packages/**"
  - "schemas/**"
  - "README.md"
  - "CLAUDE.md"
supersedes: []
tags:
  - foundation
  - scope
---

## Context

This project was extracted from a prior codebase ("Chassis") whose audit concluded that ~85% of its surface either competed unfavorably with GitHub Spec Kit or was unimplemented scaffolding. The salvageable kernel — a typed metadata vocabulary, a stable-ID discipline, an assurance ladder, an exemption registry pattern, and a JSON-Schema → typed-Rust codegen — is the basis for this rebuild.

The audit also identified Spec Kit (github/spec-kit) as the dominant adjacent project: MIT-licensed, GitHub-backed, ~90k stars at audit time, 30+ AI agent integrations, 70+ community extensions. Competing with Spec Kit on intent capture is a losing position.

## Decision

1. **Scope is Rust + TypeScript only.** Python/Go/C# codegen surfaces from the original are out of scope.
2. **Position as a complement to Spec Kit, not a competitor.** The differentiator is what happens *after* the spec is written: verifiable adherence — trace graph (spec ↔ code ↔ test), drift detection over git history, signed attestation per release, breaking-change diff on contracts, exemption registry with hard expiry, and an MCP server for direct agent integration.
3. **Preserve the load-bearing vocabulary verbatim.** Stable IDs (rule IDs, claim IDs, ADR IDs, exemption IDs), the five-rung assurance ladder (`declared → coherent → verified → enforced → observed`), and the attestation artifact shape are inherited intentionally from the original Chassis. See `docs/STABLE-IDS.md` and `docs/ASSURANCE-LADDER.md`.
4. **No runtime claims without a runtime.** The original's "Rust governance runtime" was scaffolding. This project does not use the word "runtime" in user-facing copy until an enforcement point exists.

## Consequences

- Distribution targets: `chassis-core` to crates.io, `@chassis/core-types` (renamed from `@chassis/types`) to npm, plus a Spec Kit extension package on day one.
- The 32 historical ADRs in `reference/adrs-original/` are reference material only. Re-author any that still bind this project as new ADRs starting from ADR-0002.
- The five-rung assurance ladder MVP ships with `declared` and `verified` only; the other three rungs require additional infrastructure and are deferred.
- Any future scope expansion to additional languages requires a superseding ADR.
