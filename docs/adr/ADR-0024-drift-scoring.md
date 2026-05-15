---
id: ADR-0024
title: "Drift scoring rubric (claim vs implementation staleness)"
status: accepted
date: "2026-05-14"
supersedes: []
enforces:
  - rule: CH-DRIFT-CHURN-INFO
    description: "Drift score in (0, 5] — light implementation churn versus claim staleness (info)."
  - rule: CH-DRIFT-CLAIM-STALE
    description: "Drift score in the (5, 20] band — claim text/reality likely diverging (warning)."
  - rule: CH-DRIFT-CLAIM-ABANDONED
    description: "Drift score above 20 — treat as abandoned relative to implementation churn (error)."
  - rule: CH-DRIFT-IMPL-MISSING
    description: "No implementation file can be resolved for a claim site (error)."
---

# ADR-0024 — Drift scoring

## Context

Assurance beyond `declared` needs a cheap, git-grounded signal when implementation churn outpaces claim edits.

## Decision

- **Inputs (per claim):** last edit time of the claim in the canonical contract file, last edit time of the bound implementation artifact, commit churn on that implementation since the claim edit, and a deterministic `now` injected by callers (tests/CLI), not sampled inside pure scoring functions.
- **Per-claim raw score:**

  \[
    \textit{score} = \textit{impl\_commits\_since\_last\_claim\_edit} \times \ln\bigl(1 + \textit{days\_since\_last\_claim\_edit}\bigr)
  \]

  where “days” is the real-valued difference in UTC between `now` and the last claim edit timestamp.

- **Bands:**
  - `score = 0` → no diagnostic from this rubric.
  - `0 < score ≤ 5` → info `CH-DRIFT-CHURN-INFO`.
  - `5 < score ≤ 20` → warning `CH-DRIFT-CLAIM-STALE`.
  - `score > 20` → error `CH-DRIFT-CLAIM-ABANDONED`.

  Exact numeric cutoffs are intentional for v1; making them configurable is deferred.
- **Missing implementation artifact** for a traced site → error `CH-DRIFT-IMPL-MISSING` regardless of score.
- Git integration (timestamps, churn counts) lives in `chassis-core::drift::git`; the numeric rubric stays pure in `chassis-core::drift::score` (no IO, no `Utc::now` inside the scorer).

## Consequences

CI and CLI must thread an explicit clock for deterministic tests; local runs may pass wall-clock `now`.
