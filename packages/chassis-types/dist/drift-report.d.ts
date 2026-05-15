/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/drift-report.schema.json
 */

export interface DriftReport {
  version: 1;
  summary: {
    stale: number;
    abandoned: number;
    missing: number;
  };
  diagnostics: {}[];
}
