/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/contract.schema.json
 */

/**
 * Machine-readable metadata for a code module (CONTRACT.yaml). Kind-discriminated contracts defer payload constraints to per-kind subschemas under schemas/contract-kinds/.
 */
export type Contract = {} & (
  | LibraryContractPayload
  | CLIContractPayload
  | UIComponentContractPayload
  | APIEndpointContractPayload
  | DataEntityContractPayload
  | ServiceBoundaryContractPayload
  | EventStreamContractPayload
  | FeatureFlagContractPayload
);

/**
 * Schema for kind=library contracts. Deepens `exports` from string list to structured path+kind objects.
 */
export interface LibraryContractPayload {
  kind: 'library';
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
   * Declared public surface of the library (modules, types, functions, etc).
   */
  exports: {
    path: string;
    kind: 'function' | 'type' | 'module' | 'macro' | 'trait' | 'constant';
    description?: string;
  }[];
  /**
   * Vendor extension namespace; values must be objects.
   *
   * This interface was referenced by `LibraryContractPayload`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
/**
 * Schema for kind=cli contracts. Requires entrypoint + argsSummary; allows structured subcommands.
 */
export interface CLIContractPayload {
  kind: 'cli';
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
   * Primary executable name (e.g. `chassis`).
   */
  entrypoint: string;
  /**
   * Human-readable synopsis of top-level CLI arguments.
   */
  argsSummary: string;
  /**
   * Declared subcommands. Optional; absence means CLI is monolithic.
   */
  subcommands?: {
    name: string;
    description: string;
    required_args?: string[];
    optional_args?: string[];
  }[];
  /**
   * Vendor extension namespace; values must be objects.
   *
   * This interface was referenced by `CLIContractPayload`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
/**
 * Schema for kind=component contracts. Requires structured props/events/slots/states arrays.
 */
export interface UIComponentContractPayload {
  kind: 'component';
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
  props: {
    name: string;
    type: string;
    required: boolean;
    description?: string;
    default?: unknown;
  }[];
  events: {
    name: string;
    payload_schema_ref?: string;
    description?: string;
  }[];
  slots: {
    name: string;
    description: string;
    required?: boolean;
  }[];
  states: {
    name: string;
    /**
     * Names of states reachable from this state.
     */
    transitions?: string[];
    description?: string;
  }[];
  /**
   * Optional accessibility annotations.
   */
  accessibility?: {
    [k: string]: unknown;
  };
  dependencies?: string[];
  ui_taxonomy?:
    | 'presentational'
    | 'container'
    | 'layout'
    | 'form'
    | 'navigation'
    | 'feedback'
    | 'data-display'
    | 'overlay';
  /**
   * Vendor extension namespace; values must be objects.
   *
   * This interface was referenced by `UIComponentContractPayload`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
/**
 * Schema for kind=endpoint contracts. Requires HTTP method, path, auth, structured request/response.
 */
export interface APIEndpointContractPayload {
  kind: 'endpoint';
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
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE' | 'HEAD' | 'OPTIONS';
  path: string;
  auth:
    | string
    | {
        type: string;
        [k: string]: unknown;
      };
  request: {
    content_type: string;
    schema_ref?: string;
  };
  response: {
    content_type: string;
    schema_ref?: string;
    status_code?: number;
  };
  request_examples?: {
    name: string;
    description?: string;
    value?: unknown;
  }[];
  response_examples?: {
    name: string;
    description?: string;
    value?: unknown;
  }[];
  /**
   * Vendor extension namespace; values must be objects.
   *
   * This interface was referenced by `APIEndpointContractPayload`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
/**
 * Schema for kind=entity contracts. Requires structured fields and relationships; indexes/timestamps optional.
 */
export interface DataEntityContractPayload {
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
   * This interface was referenced by `DataEntityContractPayload`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
/**
 * Schema for kind=service contracts. Requires protocol + endpoints + consumes + produces.
 */
export interface ServiceBoundaryContractPayload {
  kind: 'service';
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
  protocol: string;
  /**
   * Endpoint names or contract refs exposed by this service.
   */
  endpoints: string[];
  /**
   * Event/topic names this service consumes.
   */
  consumes: string[];
  /**
   * Event/topic names this service produces.
   */
  produces: string[];
  /**
   * Optional resilience knobs (timeouts, retries). Open object for now.
   */
  resilience?: {
    [k: string]: unknown;
  };
  /**
   * Vendor extension namespace; values must be objects.
   *
   * This interface was referenced by `ServiceBoundaryContractPayload`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
/**
 * Schema for kind=event-stream contracts. Requires source, payload (format-tagged), delivery guarantee, and consumers.
 */
export interface EventStreamContractPayload {
  kind: 'event-stream';
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
  source: string;
  payload: {
    format: 'json' | 'avro' | 'protobuf' | 'raw';
    schema_ref?: string;
  };
  delivery: 'at-least-once' | 'at-most-once' | 'exactly-once' | 'unknown';
  consumers: string[];
  /**
   * Vendor extension namespace; values must be objects.
   *
   * This interface was referenced by `EventStreamContractPayload`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
/**
 * Schema for kind=feature-flag contracts. Requires typed value + targeting (rules + default_variation) + metrics.
 */
export interface FeatureFlagContractPayload {
  kind: 'feature-flag';
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
  type: 'bool' | 'string' | 'number' | 'json';
  /**
   * Default flag value (any JSON type).
   */
  defaultValue: {
    [k: string]: unknown;
  };
  targeting: {
    rules: {
      description: string;
      /**
       * Value served when this rule matches.
       */
      variation: {
        [k: string]: unknown;
      };
      conditions?: {
        attribute: string;
        operator: string;
        value?: unknown;
      }[];
      percentage?: number;
    }[];
    /**
     * Value when no rule matches.
     */
    default_variation: {
      [k: string]: unknown;
    };
  };
  metrics: string[];
  expiration?: string;
  /**
   * Vendor extension namespace; values must be objects.
   *
   * This interface was referenced by `FeatureFlagContractPayload`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
