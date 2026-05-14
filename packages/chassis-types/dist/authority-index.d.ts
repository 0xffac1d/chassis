/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/authority-index.schema.json
 */

/**
 * Optional repo-local sidecar listing verification and invalidation rules for manifests and IR. Does not replace JSON Schema validation.
 */
export interface AuthorityIndex {
  chassis_authority_index_version: '1.0.0';
  entries: {
    /**
     * Repo-relative path to CONTRACT.yaml, chassis.unit.yaml, or architecture graph.
     */
    path: string;
    authority_class:
      | 'verified_contract'
      | 'verified_unit'
      | 'architecture_ir'
      | 'inferred_manifest'
      | 'generated_artifact'
      | 'prose_doc';
    freshness_status: 'fresh' | 'stale' | 'unknown';
    /**
     * RFC 3339 timestamp of last human verification.
     */
    verified_at: string;
    /**
     * Actor id or role (opaque string).
     */
    verified_by: string;
    /**
     * Upstream paths or IR ids this summary was derived from.
     */
    derived_from: string[];
    /**
     * Repo-relative globs; if any matching file is newer than verified_at, tooling SHOULD mark stale.
     */
    invalidation_triggers: string[];
  }[];
}
