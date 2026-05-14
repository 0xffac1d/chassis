/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/tag-ontology.schema.json
 */

/**
 * Registry-backed tags (P1.1): stable ids, optional namespaces, owners, allowed targets, synonyms, and deprecation. Used as a sidecar or embedded slice; not required for all manifests.
 */
export interface TagOntology {
  version?: string;
  /**
   * Default namespace for tags in this file when a tag omits one.
   */
  namespace?: string;
  /**
   * @minItems 1
   */
  tags: [TagDefinition, ...TagDefinition[]];
}
export interface TagDefinition {
  id: string;
  title: string;
  namespace?: string;
  owner?: string;
  allowedTargets?: string[];
  synonyms?: string[];
  deprecated?: boolean;
  replacedBy?: string;
}
