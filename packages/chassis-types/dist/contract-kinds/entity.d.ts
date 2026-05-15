/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/contract-kinds/entity.schema.json
 */

/**
 * Schema for kind=entity contracts. Requires structured fields and relationships; indexes/timestamps optional.
 */
export interface Entity {
  kind: 'entity';
  name: string;
  purpose: string;
  status:
    | 'draft'
    | 'experimental'
    | 'stable'
    | 'deprecated'
    | 'active'
    | 'archived'
    | 'inferred'
    | 'superseded'
    | 'pre-alpha';
  since: string;
  version: string;
  assurance_level: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
  owner: string;
  /**
   * @minItems 1
   */
  invariants: [
    {
      id: string;
      text: string;
      test_linkage?: string[];
    },
    ...{
      id: string;
      text: string;
      test_linkage?: string[];
    }[]
  ];
  edge_cases: {
    id: string;
    text: string;
    test_linkage?: string[];
  }[];
  superseded_by?: string;
  linked_objectives?: string[];
  ring?: number;
  /**
   * Structured declared inputs.
   */
  inputs?: {
    name: string;
    description: string;
    schemaRef?: string;
  }[];
  /**
   * Structured declared outputs.
   */
  outputs?: {
    name: string;
    description: string;
    schemaRef?: string;
  }[];
  drift?: {
    skip_exports?: boolean;
    language?: 'auto' | 'typescript' | 'rust' | 'none';
    ignore_paths?: string[];
    package_layer?: string;
    tier?: string;
  };
  debt?: {
    id: string;
    description: string;
    severity?: 'low' | 'medium' | 'high' | 'critical';
    owner?: string;
    remediation?: string;
  }[];
  generated?: boolean;
  rationale?: string[];
  test_linkage?: {
    claim_id: string;
    test_file: string;
    validation_method?: 'test' | 'runtime-assertion' | 'monitor' | 'manual';
    confidence?: 'low' | 'medium' | 'high';
  }[];
  caveats?: string[];
  depends_on?: string[];
  depended_by?: string[];
  tags?: string[];
  architecture_system?: string;
  /**
   * Namespaced extensions.
   */
  extensions?: {
    [k: string]: {};
  };
  /**
   * @minItems 1
   */
  fields: [
    {
      name: string;
      type: string;
      nullable?: boolean;
      indexed?: boolean;
      description?: string;
    },
    ...{
      name: string;
      type: string;
      nullable?: boolean;
      indexed?: boolean;
      description?: string;
    }[]
  ];
  relationships: {
    name: string;
    kind: 'one-to-one' | 'one-to-many' | 'many-to-many';
    target: string;
  }[];
  indexes?: {
    /**
     * @minItems 1
     */
    fields: [string, ...string[]];
    unique?: boolean;
    name?: string;
  }[];
  timestamps?: {
    createdAt?: boolean;
    updatedAt?: boolean;
    deletedAt?: boolean;
  };
  /**
   * Vendor extension namespace; values must be objects.
   *
   * This interface was referenced by `Entity`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
