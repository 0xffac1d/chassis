/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/exemption-registry.schema.json
 */

export type Entry = {
  [k: string]: unknown;
} & {
  /**
   * Format EX-YYYY-NNNN. Must be unique within the registry.
   */
  id: string;
  /**
   * Legacy alias for `rule_id` (v1 field name).
   */
  rule?: string;
  /**
   * Rule identifier this exemption suppresses. Either `rule_id` or `finding_id` must be set.
   */
  rule_id?: string;
  /**
   * Specific finding instance id (preferred when targeting one finding). Either `rule_id` or `finding_id` must be set.
   */
  finding_id?: string;
  /**
   * Repo-relative file path or glob (legacy alias for `path`).
   */
  scope?: string | [string, ...string[]];
  /**
   * Repo-relative file path or glob (preferred). Either `path` or `scope` must be set.
   */
  path?: string | [string, ...string[]];
  /**
   * Why this waiver is needed. Minimum 40 chars prevents drive-by 'todo later' suppressions.
   */
  reason: string;
  /**
   * Email or handle accountable for resolving the waiver.
   */
  owner: string;
  /**
   * Legacy alias for `linked_issue`.
   */
  ticket?: string;
  /**
   * Optional issue tracker reference (URL or 'PROJ-123'). Strongly recommended.
   */
  linked_issue?: string;
  /**
   * Optional ADR id that defines the rule being exempted.
   */
  adr?: string;
  /**
   * Legacy alias for `created_at`.
   */
  created?: string;
  /**
   * ISO date the exemption was opened. Either `created_at` or `created` must be set.
   */
  created_at?: string;
  /**
   * Legacy alias for `expires_at`.
   */
  expires?: string;
  /**
   * ISO date after which the exemption no longer suppresses findings. Either `expires_at` or `expires` must be set. Maximum: created+90 days (enforced by `chassis exemptions add`).
   */
  expires_at?: string;
  /**
   * Optional. When set, the suppressed finding is downgraded to this severity instead of dropped, so it remains visible in reports without failing.
   */
  severity_override?: 'info' | 'warning' | 'error';
  /**
   * Lifecycle state. `active` participates in suppression; `expired` is informational only and surfaces as an `expired_exemptions` audit entry; `revoked` is retained as historical evidence and never suppresses.
   */
  status?: 'active' | 'expired' | 'revoked';
  /**
   * Per-entry opt-in for wildcard / global scopes. Required when path/scope contains `*`, `**`, or matches the repository root. Strict profile additionally requires the registry-level `allow_global: true`.
   */
  allow_global?: boolean;
};

/**
 * Time-bounded waiver registry for Chassis findings. Each entry grants a narrowly-scoped, owner-accountable, expiring exemption for a specific rule (or finding id). Audit suppresses a finding only when an entry is **active**, scope-matched, and not expired. Expired or invalid entries become audit findings of their own. Wildcard / global scopes require an explicit `allow_global: true` and fail strict mode unless the registry-level `allow_global: true` is set. Schema bumped to 1.1 for lifecycle fields (`status`, `severity_override`, `linked_issue`, `created_at`/`expires_at`, `path`, `rule_id`, `finding_id`).
 */
export interface ExemptionRegistry {
  /**
   * Registry schema major. 1 = legacy field names (rule, scope, created, expires). 2 = lifecycle fields (rule_id, path/scope, created_at, expires_at, status). Both are accepted; v2 is recommended for new entries.
   */
  version: 1 | 2;
  quota?: {
    total_max?: number;
    per_file_max?: number;
  };
  /**
   * When true, individual entries may use scope wildcards (`*`, `**`, repository-root '/'). Strict profile still rejects global entries unless this flag and the per-entry `allow_global: true` are both set.
   */
  allow_global?: boolean;
  entries: Entry[];
}
