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
  diagnostics: {}[];
  exemptions: {
    registry?: null | {};
    diagnostics: {}[];
  };
  drift_summary: {
    stale: number;
    abandoned: number;
    missing: number;
  };
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
