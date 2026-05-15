/**
 * AUTO-GENERATED — do not edit.
 *
 * Barrel re-exporting every generated schema type.
 * - Bare names: collision-free schema title names exported at the barrel root.
 * - Namespaced: each file is reachable under `<DomainSchema>_<TypeName>` imports.
 * - Regenerate via `npm run build` after schema edits; fingerprints must agree
 *   with `fingerprint.sha256` / `manifest.json` (ADR-0015 / ADR-0017 parity).
 */
// --- Bare top-level re-exports (collision-free names only) ---
export type { Adr } from './adr';
export type { AuthorityIndex } from './authority-index';
export type { CedarFacts } from './cedar-facts';
export type { Cli } from './contract-kinds/cli';
export type { CoherenceReport } from './coherence-report';
export type { Component } from './contract-kinds/component';
export type { Contract } from './contract';
export type { Diagnostic } from './diagnostic';
export type { DriftReport } from './drift-report';
export type { DsseEnvelope } from './dsse-envelope';
export type { Endpoint } from './contract-kinds/endpoint';
export type { Entity } from './contract-kinds/entity';
export type { EventStream } from './contract-kinds/event-stream';
export type { EventcatalogMetadata } from './eventcatalog-metadata';
export type { ExemptionRegistry } from './exemption-registry';
export type { FeatureFlag } from './contract-kinds/feature-flag';
export type { FieldDefinition } from './field-definition';
export type { InTotoStatementV1 } from './in-toto-statement-v1';
export type { Library } from './contract-kinds/library';
export type { OpaInput } from './opa-input';
export type { PolicyInput } from './policy-input';
export type { ReleaseGate } from './release-gate';
export type { Service } from './contract-kinds/service';
export type { TagOntology } from './tag-ontology';
export type { TraceGraph } from './trace-graph';

// --- Namespaced re-exports (every schema, always reachable) ---
export * as Adr_Adr from './adr';
export * as AuthorityIndex_AuthorityIndex from './authority-index';
export * as CedarFacts_CedarFacts from './cedar-facts';
export * as CoherenceReport_CoherenceReport from './coherence-report';
export * as ContractKinds_Cli from './contract-kinds/cli';
export * as ContractKinds_Component from './contract-kinds/component';
export * as ContractKinds_Endpoint from './contract-kinds/endpoint';
export * as ContractKinds_Entity from './contract-kinds/entity';
export * as ContractKinds_EventStream from './contract-kinds/event-stream';
export * as ContractKinds_FeatureFlag from './contract-kinds/feature-flag';
export * as ContractKinds_Library from './contract-kinds/library';
export * as ContractKinds_Service from './contract-kinds/service';
export * as Contract_Contract from './contract';
export * as Diagnostic_Diagnostic from './diagnostic';
export * as DriftReport_DriftReport from './drift-report';
export * as DsseEnvelope_DsseEnvelope from './dsse-envelope';
export * as EventcatalogMetadata_EventcatalogMetadata from './eventcatalog-metadata';
export * as ExemptionRegistry_ExemptionRegistry from './exemption-registry';
export * as FieldDefinition_FieldDefinition from './field-definition';
export * as InTotoStatementV1_InTotoStatementV1 from './in-toto-statement-v1';
export * as OpaInput_OpaInput from './opa-input';
export * as PolicyInput_PolicyInput from './policy-input';
export * as ReleaseGate_ReleaseGate from './release-gate';
export * as TagOntology_TagOntology from './tag-ontology';
export * as TraceGraph_TraceGraph from './trace-graph';
