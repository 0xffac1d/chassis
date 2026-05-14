/**
 * AUTO-GENERATED — do not edit.
 *
 * Barrel re-exporting every generated schema type.
 * - Bare names: top-level schema types with a globally unique name
 *   (e.g. Contract, Diagnostic). Use `import { Contract }`.
 * - Namespaced: every schema module is always available under a
 *   domain-qualified namespace (e.g. Contract_Contract) so
 *   collisions (multiple "Policy" schemas) remain reachable.
 * - Generated from schemas at build time; re-run `npm run build`
 *   after schema changes, and verify the fingerprint matches
 *   `node scripts/fingerprint-schemas.mjs`.
 */
// --- Bare top-level re-exports (collision-free names only) ---
export type { Adr } from './adr';
export type { AuthorityIndex } from './authority-index';
export type { CoherenceReport } from './coherence-report';
export type { Contract } from './contract';
export type { Diagnostic } from './diagnostic';
export type { ExemptionRegistry } from './exemption-registry';
export type { FieldDefinition } from './field-definition';
export type { TagOntology } from './tag-ontology';

// --- Namespaced re-exports (every schema, always reachable) ---
export * as Adr_Adr from './adr';
export * as AuthorityIndex_AuthorityIndex from './authority-index';
export * as CoherenceReport_CoherenceReport from './coherence-report';
export * as Contract_Contract from './contract';
export * as Diagnostic_Diagnostic from './diagnostic';
export * as ExemptionRegistry_ExemptionRegistry from './exemption-registry';
export * as FieldDefinition_FieldDefinition from './field-definition';
export * as TagOntology_TagOntology from './tag-ontology';
