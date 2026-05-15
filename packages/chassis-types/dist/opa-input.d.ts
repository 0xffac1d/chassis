/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/opa-input.schema.json
 */

/**
 * OPA adapter wrapping Chassis policy-input facts under the standard `input` key. Mirrors `schemas/policy-input.schema.json` field-for-field so `opa eval --schema schemas/opa-input.schema.json` type-checks Rego policies against the canonical Chassis fact shape. Chassis does not evaluate Rego.
 */
export interface OpaInput {
  input: {
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
    spec_kit?: {
      spec_index_digest: string;
    };
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
