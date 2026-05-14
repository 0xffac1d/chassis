/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/coherence-report.schema.json
 */

/**
 * Machine-readable longitudinal coherence signals: authority alignment, stale metadata, schema validation, and trust ladder for agent context ranking. Produced by `chassis coherence`.
 */
export interface CoherenceReport {
  schema_version: '1.0.0';
  /**
   * UTC RFC 3339 timestamp when the report was generated.
   */
  generated_at: string;
  /**
   * Canonical list of repo-relative paths to runtime evidence batch JSON files that coherence loaded (may be empty). Mirrors under summary.runtime_evidence_sources and summary.observed_truth.runtime_evidence_sources are identical.
   */
  runtime_evidence_sources: string[];
  summary: {
    errors: number;
    warnings: number;
    infos: number;
    by_category?: {
      [k: string]: number;
    };
    /**
     * Aggregate CONTRACT debt[] counts from summarize_contract_debt.
     */
    debt?: {
      [k: string]: unknown;
    };
    semantic_contradiction_count?: number;
    projection_drift_count?: number;
    runtime_evidence_observation_count?: number;
    /**
     * Deprecated mirror of top-level runtime_evidence_sources. Prefer the root field; this list is always identical for backward-compatible consumers.
     */
    runtime_evidence_sources?: string[];
    /**
     * Findings for missing reviewer acknowledgments or verified+blocking debt.
     */
    promotion_policy_gap_count?: number;
    /**
     * Rollup of CONTRACT.yaml assurance_level fields across the tree. Counts per tier + the lowest observed tier. CI can gate on advancement (e.g. fail if lowest < verified).
     */
    assurance_ladder?: {
      declared: number;
      coherent: number;
      verified: number;
      enforced: number;
      observed: number;
      /**
       * Contracts that declare no assurance_level.
       */
      unset: number;
      /**
       * Lowest observed tier across all contracts. `unset` means at least one contract declares no level.
       */
      lowest: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed' | 'unset';
    };
    /**
     * Runtime evidence rollup: declared_unobserved refs, coverage, and observation counts. Loaded batch file paths are mirrored here from top-level runtime_evidence_sources (identical list).
     */
    observed_truth?: {
      /**
       * Mirror of top-level runtime_evidence_sources; always the same list for consumers that read observed_truth as a unit.
       */
      runtime_evidence_sources?: string[];
      /**
       * Total observation rows counted across loaded batch files (not the same as summary.runtime_evidence_observation_count, which counts runtime_evidence_signal findings).
       */
      observation_count?: number;
      declared_unobserved_ref_count?: number;
      declared_unobserved_refs?: string[];
      /**
       * Fraction of contracts with matching runtime observations.
       */
      observed_coverage_ratio?: number;
      /**
       * Count of contracts with runtime evidence backing.
       */
      contracts_with_evidence?: number;
      /**
       * Overall impact of runtime evidence on confidence: high (>=50% coverage), medium (>=20%), low (>0 observations), none.
       */
      confidence_impact?: 'high' | 'medium' | 'low' | 'none';
      [k: string]: unknown;
    };
  };
  /**
   * Epistemic ranking for context artifacts (same source as schemas/coherence/trust-ladder.data.json).
   */
  trust_ladder: {
    rank: number;
    id: string;
    label: string;
    description: string;
    applies_to: string[];
    guidance: string;
  }[];
  /**
   * Optional per-path authority/freshness snapshot for agent ranking (verified vs inferred vs generated).
   */
  authority_ledger?: {
    path: string;
    authority_class:
      | 'verified_contract'
      | 'verified_unit'
      | 'architecture_ir'
      | 'inferred_manifest'
      | 'generated_artifact'
      | 'prose_doc';
    freshness_status: 'fresh' | 'stale' | 'unknown';
    derived_from?: string[];
    /**
     * Explicit classification: canonical (verified truth), debt (inferred, needs promotion), explanatory (prose/generated, not authoritative).
     */
    truth_classification?: 'canonical' | 'debt' | 'explanatory' | 'unknown';
  }[];
  /**
   * Repository-native policy suggestions keyed to finding severities (not enforced — integrators map to gates).
   */
  autonomy_hints?: {
    /**
     * Condition (e.g. any_error, metadata_stale_warning).
     */
    when: string;
    hint: string;
  }[];
  findings: {
    id: string;
    category:
      | 'authority_alignment'
      | 'authority_missing_parent'
      | 'metadata_stale'
      | 'schema_invalid'
      | 'yaml_invalid'
      | 'semantic_warning'
      | 'missing_decisions'
      | 'semantic_contradiction'
      | 'missing_architecture_link'
      | 'ir_graph_integrity'
      | 'inferred_truth_gap'
      | 'drift_finding'
      | 'projection_drift'
      | 'runtime_evidence_signal'
      | 'structured_debt';
    severity: 'error' | 'warning' | 'info';
    message: string;
    /**
     * Repo-relative path to primary file or directory.
     */
    path?: string;
    related_paths?: string[];
    /**
     * Stable sub-code, e.g. authority classification key.
     */
    code?: string;
    /**
     * JSON Pointer or validator location when applicable.
     */
    location?: string;
    /**
     * Trust ladder rank most affected (see trust_ladder[].rank).
     */
    trust_rank_hint?: number;
    /**
     * Machine-readable stale triggers for metadata_stale findings (e.g. implementation_newer_than_manifest, architecture_ir_newer_than_manifest).
     */
    stale_reasons?: string[];
    /**
     * Drift engine adapter summary when this row is a drift_finding.
     */
    drift_adapter?: {
      [k: string]: unknown;
    };
  }[];
}
