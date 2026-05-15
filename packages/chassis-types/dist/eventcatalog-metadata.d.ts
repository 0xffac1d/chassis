/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/eventcatalog-metadata.schema.json
 */

/**
 * Metadata derived only from current Chassis service and event-stream contract fields. This is not an EventCatalog engine or OpenLineage run event emitter.
 */
export interface EventcatalogMetadata {
  schema_version: 1;
  services: Service[];
  messages: Message[];
  metadata: {};
}
export interface Service {
  name: string;
  version: string;
  owner: string;
  contract_path: string;
  protocol: unknown;
  endpoints: unknown[];
  consumes: unknown[];
  produces: unknown[];
}
export interface Message {
  name: string;
  version: string;
  owner: string;
  contract_path: string;
  source: unknown;
  payload: unknown;
  delivery: unknown;
  consumers: unknown[];
}
