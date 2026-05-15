/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/cedar-facts.schema.json
 */

/**
 * Cedar-style entity/action/resource facts for external authorization systems. Chassis does not evaluate Cedar policies.
 */
export interface CedarFacts {
  schema_version: 1;
  entities: Entity[];
  actions: Action[];
  resources: Resource[];
}
export interface Entity {
  uid: Uid;
  attrs: {};
  parents: Uid[];
}
export interface Uid {
  type: string;
  id: string;
}
export interface Action {
  name: string;
  applies_to: string[];
}
export interface Resource {
  uid: Uid;
  attrs: {};
}
