/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/release-gate.schema.json
 */

export interface ReleaseGate {
  schema_fingerprint: string;
  git_commit: string;
  built_at: string;
  verdict: 'pass' | 'fail';
  fail_on_drift: boolean;
  trace_failed: boolean;
  drift_failed: boolean;
  exemption_failed: boolean;
  attestation_failed: boolean;
  unsuppressed_blocking: number;
  suppressed: number;
  severity_overridden: number;
  final_exit_code: number;
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
