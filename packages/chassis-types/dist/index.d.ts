/**
 * AUTO-GENERATED — do not edit.
 *
 * Barrel re-exporting every generated schema type.
 * - Bare names: top-level schema types with a globally unique name
 *   (e.g. AgentAction, ApiResponse). Use `import { AgentAction }`.
 * - Namespaced: every schema module is always available under a
 *   domain-qualified namespace (e.g. Agent_AgentAction) so
 *   collisions (multiple "Policy" schemas) remain reachable.
 * - Generated from schemas at build time; re-run `npm run build`
 *   after schema changes, and verify the fingerprint matches
 *   chassis `fingerprint_schemas.py`.
 */
// --- Bare top-level re-exports (collision-free names only) ---
export type { Adr } from './adr';
export type { AuthorityIndex } from './authority-index';
export type { Cli } from './contract-kinds/cli';
export type { CoherenceReport } from './coherence-report';
export type { Component } from './contract-kinds/component';
export type { Contract } from './contract';
export type { Diagnostic } from './diagnostic';
export type { Endpoint } from './contract-kinds/endpoint';
export type { Entity } from './contract-kinds/entity';
export type { EventStream } from './contract-kinds/event-stream';
export type { ExemptionRegistry } from './exemption-registry';
export type { FeatureFlag } from './contract-kinds/feature-flag';
export type { FieldDefinition } from './field-definition';
export type { Library } from './contract-kinds/library';
export type { Service } from './contract-kinds/service';
export type { TagOntology } from './tag-ontology';

// --- Namespaced re-exports (every schema, always reachable) ---
export * as Adr_Adr from './adr';
export * as AuthorityIndex_AuthorityIndex from './authority-index';
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
export * as ExemptionRegistry_ExemptionRegistry from './exemption-registry';
export * as FieldDefinition_FieldDefinition from './field-definition';
export * as TagOntology_TagOntology from './tag-ontology';
