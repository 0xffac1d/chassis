/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/scanner-summary.schema.json
 */

export interface ScannerSummary {
  tool: 'semgrep' | 'codeql';
  toolVersion?: string;
  sarifSha256: string;
  runId?: string;
  total: number;
  errors: number;
  warnings: number;
  infos: number;
  diagnostics: Diagnostic[];
}
export interface Diagnostic {
  source?: string;
  subject?: string;
  ruleId: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  violated?: {
    convention: string;
  };
  docs?: string;
  fix?: {};
  location?: {};
  detail?: {};
}
