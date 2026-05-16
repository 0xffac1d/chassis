/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/policy-input.schema.json
 */

/**
 * Export-only Chassis facts for downstream governance systems. This is not a policy language or enforcement engine.
 */
export interface PolicyInput {
  version: 1;
  repo: {
    root: string;
    git_commit?: string;
    schema_fingerprint?: string;
  };
  contracts: ContractFact[];
  claims: ClaimFact[];
  diagnostics: Diagnostic[];
  exemptions: {
    registry?: null | {};
    diagnostics: Diagnostic[];
  };
  drift_summary: {
    stale: number;
    abandoned: number;
    missing: number;
  };
  spec_kit?: {
    spec_index_digest: string;
  };
  scanner_summaries: ScannerSummary[];
  scanner_required: boolean;
}
export interface ContractFact {
  path: string;
  name: string;
  kind: string;
  version: string;
  owner: string;
  assurance_level: string;
  status: string;
  document: {};
}
export interface ClaimFact {
  claim_id: string;
  contract_path: string;
  contract_kind: 'invariant' | 'edge_case';
  claim_record: {};
  impl_sites: ClaimSite[];
  test_sites: ClaimSite[];
  adr_refs: string[];
  active_exemptions: string[];
}
export interface ClaimSite {
  file: string;
  line: number;
  claim_id: string;
  kind: 'impl' | 'test';
}
/**
 * Inline mirror of schemas/diagnostic.schema.json load-bearing fields for policy-input validation (2020-12).
 */
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
  fix?: {
    applicability: 'automatic' | 'suggested' | 'manual-only';
    description?: string;
    patch?: string;
  };
  location?: {
    path: string;
    range?: {
      start: Position;
      end?: Position;
    };
  };
  detail?: {};
}
export interface Position {
  line: number;
  column?: number;
}
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
