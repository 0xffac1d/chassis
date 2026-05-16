---
id: ADR-0033
title: "Scanner SARIF evidence normalization"
status: accepted
date: "2026-05-15"
enforces:
  - rule: CH-SCANNER-FINDING
  - rule: CH-SCANNER-SARIF-MALFORMED
  - rule: CH-GATE-SCANNER-EVIDENCE-REQUIRED
applies_to:
  - crates/chassis-core/src/scanner
tags:
  - supply-chain
  - evidence
---

# ADR-0033 — Normalized scanner findings in Chassis diagnostics

## Decision

- Semgrep and CodeQL emit SARIF; Chassis ingests SARIF into `ScannerSummary` artifacts (`schemas/scanner-summary.schema.json`) under `dist/scanner-{semgrep,codeql}.json`.
- Every SARIF result maps to a wire diagnostic with stable `ruleId` **`CH-SCANNER-FINDING`**. The original SARIF rule id is preserved under `detail.sarifRuleId`; `source` is `semgrep` or `codeql`.
- Parse failures use **`CH-SCANNER-SARIF-MALFORMED`** (typed error / ingest path), not a permissive envelope.
- `violated.convention` for normalized findings references **this ADR** (`ADR-0033`).

## Consequences

- OPA and release-gate can treat scanner blocking like other diagnostics without an unbounded rule-id vocabulary in ADR frontmatter.
- Specific tool rules remain auditable via `detail.sarifRuleId` and the retained SARIF blob digest on `ScannerSummary`.
