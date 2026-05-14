---
id: ADR-0010
title: Posture model — security-sensitive defaults, posture.toml, exemption escapes
status: accepted
date: "2026-04-19"
enforces:
  - rule: POSTURE-FIELD-MISSING
    description: "A field declared in config/chassis/posture.toml is absent from the configuration file the posture rule applies to."
  - rule: POSTURE-VALUE-INVALID
    description: "A field's value violates its posture rule (e.g. tls_enforce = false where posture demands true)."
  - rule: POSTURE-EXEMPTION-MISSING
    description: "A field violates its posture rule but no matching .exemptions/registry.yaml entry exists."
  - rule: POSTURE-AUDIT-FIELD-MISSING-DEFAULT
    description: "A posture entry omits the `default` value, so adopters have no fallback when the field is absent."
  - rule: POSTURE-AUDIT-FIELD-MISSING-POSTURE
    description: "A posture entry omits the `posture` (required value) field."
  - rule: POSTURE-AUDIT-FIELD-INVALID-POSTURE
    description: "A posture entry's posture value does not match the field's declared type."
  - rule: POSTURE-AUDIT-FIELD-MISSING-EVIDENCE
    description: "A posture entry omits the `evidence` (link to ADR or threat model) field."
  - rule: POSTURE-AUDIT-EXEMPTION-MISSING
    description: "A posture violation lacks a paired exemption registry entry."
  - rule: AGENT-HYGIENE-TRAILER-MISSING
    description: "Commit trailer (e.g. Co-Authored-By, Reviewed-by) required by repo policy is absent."
  - rule: AGENT-HYGIENE-TRAILER-MALFORMED
    description: "Commit trailer present but malformed (wrong format, missing email)."
  - rule: AGENT-HYGIENE-SCOPE-VIOLATION
    description: "A commit modifies files outside the .claude/scope-lock declared scope."
  - rule: AGENT-HYGIENE-FILE-COUNT-EXCESSIVE
    description: "A single commit touches more files than the per-commit cap baseline."
  - rule: AGENT-HYGIENE-DIFF-SIZE-EXCESSIVE
    description: "A single commit's diff exceeds the per-commit line-count cap baseline."
applies_to:
  - "config/chassis/posture.toml"
  - "scripts/chassis/gates/posture_audit.py"
  - "scripts/chassis/gates/agent_hygiene.py"
tags:
  - chassis
  - governance
  - posture
  - security
---

# ADR-0010: Posture model

## Context

`config/chassis/posture.toml` declares security-sensitive default
postures for fields in other config files (e.g. `tls_enforce: must be
true unless explicitly waived`). The mechanism existed since Phase E
but was never written down as a decision record; this ADR pins the
model and registers the rule IDs for the `posture_audit` gate.

This ADR also registers rule IDs for `agent_hygiene` because that
gate enforces process discipline (commit trailers, scope locks, diff
caps) — a posture-adjacent topic.

## Decision

1. **Posture is declarative.** Each entry in `posture.toml` declares
   a field path, its required value (`posture`), the default the
   ecosystem ships (`default`), and an `evidence` link (typically an
   ADR or threat model).
2. **Drift requires either alignment or exemption.** When a target
   config file's value diverges from `posture.toml::posture`, either
   the config must be brought into alignment OR an
   `.exemptions/registry.yaml` entry must reference the rule (per
   ADR-0004's exemption policy).
3. **Posture entries have a 1:1 mapping to ADRs.** The `evidence`
   field MUST point to an ADR (or downstream-equivalent governance
   artifact). This ensures every posture rule is grounded in an
   authoritative decision.
4. **Posture rules are gate-checked, not runtime-enforced.** The
   `posture_audit` gate runs in CI; the runtime is not modified by
   Chassis. Operators who want runtime enforcement can ship their own
   process-supervisor or admission controller separately.

## Consequences

- Security defaults are visible in one place (`posture.toml`) and
  drift surfaces as a structured diagnostic, not a CVE.
- Exemptions force conscious deviation; "default-on, deviation
  requires owner sign-off + 90-day expiration" replaces silent
  override.
- The `posture_audit` and `agent_hygiene` gates now have specific
  rule IDs that route to this ADR for context.

## Status

Accepted. posture.toml shape and posture_audit gate shipped in Phase
E; this ADR formalizes the model.
