/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/adr.schema.json
 */

/**
 * Frontmatter for an ADR file. ADRs live under docs/adr/ (or docs/chassis/decisions/) as Markdown with a YAML frontmatter block at the top. The frontmatter validates against this schema; the prose below is free-form. ADR `enforces` entries bind stable rule IDs to this decision; the binding-link gate (Milestone A.5) ensures every diagnostic.ruleId resolves to an ADR whose enforces[].rule matches.
 */
export interface Adr {
  /**
   * Stable identifier. Format: ADR-NNNN with 4+ digits (e.g. ADR-0001, ADR-0123). Referenced by diagnostic.violated.convention.
   */
  id: string;
  /**
   * One-line title of the decision.
   */
  title: string;
  /**
   * Lifecycle. A diagnostic binding to a superseded/deprecated ADR emits a `stale-ADR-binding` warning.
   */
  status: 'proposed' | 'accepted' | 'deprecated' | 'superseded';
  /**
   * ISO-8601 date of status change.
   */
  date: string;
  /**
   * Stable rule IDs authoritatively defined by this ADR. The binding-link gate fails on diagnostic.ruleId values that don't resolve to any ADR's enforces[] entry.
   */
  enforces?: {
    /**
     * Stable ruleId that this ADR authoritatively defines (matches diagnostic.ruleId pattern).
     */
    rule: string;
    /**
     * Optional one-line hint of what the rule checks.
     */
    description?: string;
  }[];
  /**
   * Path globs the decision applies to (e.g. `crates/**`, `schemas/metadata/*.json`). Agents use this to scope ADRs relevant to the file they're editing.
   */
  applies_to?: string[];
  /**
   * ADR IDs this decision replaces. Superseded ADRs automatically flip to status: superseded by the `chassis adr validate` tool.
   */
  supersedes?: string[];
  /**
   * Filled when status=superseded.
   */
  superseded_by?: string;
  tags?: string[];
}
