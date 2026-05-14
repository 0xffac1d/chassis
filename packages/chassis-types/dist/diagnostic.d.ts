/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/diagnostic.schema.json
 */

/**
 * Structured, agent-routable diagnostic emitted by Chassis gates, validators, and drift checks. Every finding surfaced via --json conforms to this schema. The ruleId + violated.convention pair binds the diagnostic to an ADR; the fix.applicability enum tells an agent whether to auto-apply.
 */
export interface Diagnostic {
  /**
   * Stable identifier for the rule this diagnostic violates. Format: DOMAIN-NNN (e.g. CHASSIS-001, GATE-EXPORT-03). Must resolve to an ADR whose enforces[].rule == ruleId.
   */
  ruleId: string;
  /**
   * error = hard-fail in non-advisory mode; warning = visible but non-blocking; info = observation only.
   */
  severity: 'error' | 'warning' | 'info';
  /**
   * Human-readable one-line summary. Keep under 160 characters; detail belongs in detail.
   */
  message: string;
  /**
   * Link to the convention (ADR) this diagnostic enforces. Required when ruleId is bound to an ADR.
   */
  violated?: {
    /**
     * ADR id (e.g. ADR-0007). The ADR's enforces[] array must include the ruleId.
     */
    convention: string;
  };
  /**
   * Stable anchor URL pointing at prose explaining the rule. Typically a fragment-linked docs/chassis/guides/*.md or an ADR file.
   */
  docs?: string;
  /**
   * Optional remediation hint. When applicability='automatic', the patch field should be populated; this is validated separately by the binding-link gate rather than in JSON Schema (Draft-07 if/then/else is forbidden by schemas/portability rules for cross-language compatibility).
   */
  fix?: {
    /**
     * automatic = tool can apply without human review; suggested = tool proposes, human confirms; manual-only = requires human judgment.
     */
    applicability: 'automatic' | 'suggested' | 'manual-only';
    /**
     * Short natural-language description of the fix.
     */
    description?: string;
    /**
     * Optional unified-diff or script fragment implementing the fix. Should be populated when applicability=automatic.
     */
    patch?: string;
  };
  /**
   * Where the violation lives. Path is always required; range is optional.
   */
  location?: {
    /**
     * Repo-relative POSIX path.
     */
    path: string;
    /**
     * Optional byte/line range. start <= end.
     */
    range?: {
      start: Position;
      end?: Position;
    };
  };
  /**
   * Free-form structured extension. Gate-specific fields live here (e.g. baseline counts, affected item lists).
   */
  detail?: {};
}
export interface Position {
  /**
   * 1-based line number.
   */
  line: number;
  /**
   * 1-based column number.
   */
  column?: number;
}
