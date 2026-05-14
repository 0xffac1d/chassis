/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/field-definition.schema.json
 */

/**
 * Reusable field definition for entity, event, and store schemas. Referenced via $ref from consuming schemas.
 */
export interface FieldDefinition {
  name: string;
  type: 'string' | 'integer' | 'float' | 'boolean' | 'datetime' | 'uuid' | 'json' | 'enum' | 'text' | 'binary';
  required?: boolean;
  unique?: boolean;
  indexed?: boolean;
  default?: unknown;
  validation?: string;
  description: string;
  enumValues?: string[];
  sensitive?: boolean;
}
