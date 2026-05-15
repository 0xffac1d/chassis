/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/release-gate.schema.json
 */

export interface ReleaseGate {
  schema_fingerprint: string;
  git_commit: string;
  built_at: string;
  trace_summary: {
    claims: number;
    orphan_sites: number;
  };
  drift_summary: {
    stale: number;
    abandoned: number;
    missing: number;
  };
  exempt_summary: {
    active: number;
    expired_present: number;
  };
  commands_run: {
    argv: string[];
    exit_code: number;
  }[];
}
