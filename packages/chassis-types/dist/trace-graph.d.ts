/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/trace-graph.schema.json
 */

export interface TraceGraph {
  claims: {
    [k: string]: {
      claim_id: string;
      contract_path: string;
      contract_kind: 'invariant' | 'edge_case';
      claim_record: {};
      impl_sites: ClaimSite[];
      test_sites: ClaimSite[];
      adr_refs: string[];
      active_exemptions: string[];
    };
  };
  orphan_sites: ClaimSite[];
  diagnostics: {}[];
}
export interface ClaimSite {
  file: string;
  line: number;
  claim_id: string;
  kind: 'impl' | 'test';
}
