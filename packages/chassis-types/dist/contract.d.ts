/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/contract.schema.json
 */

/**
 * Machine-readable metadata for a code module (CONTRACT.yaml). Kind-discriminated contracts align with reference/schemas-extended/ slices.
 */
export type Contract = {} & (
  | {
      kind: 'library';
      name?: string;
      purpose?: string;
      status?:
        | 'draft'
        | 'experimental'
        | 'stable'
        | 'deprecated'
        | 'active'
        | 'archived'
        | 'inferred'
        | 'superseded'
        | 'pre-alpha';
      since?: string;
      version?: string;
      assurance_level?: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
      owner?: string;
      /**
       * @minItems 1
       */
      invariants?: [
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
      edge_cases?: {
        id: string;
        text: string;
        test_linkage?: string[];
      }[];
      superseded_by?: string;
      linked_objectives?: string[];
      ring?: number;
      /**
       * Structured declared inputs (name/description rows); optional schemaRef ties rows to JSON Schema.
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
      exports: string[];
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
       * Namespaced extensions; keys must not collide with chassis top-level reserved names.
       */
      extensions?: {
        [k: string]: {};
      };
      /**
       * Vendor extension namespace. Values must be objects.
       *
       * This interface was referenced by `undefined`'s JSON-Schema definition
       * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
       */
      [k: string]: {};
    }
  | {
      kind: 'cli';
      name?: string;
      purpose?: string;
      status?:
        | 'draft'
        | 'experimental'
        | 'stable'
        | 'deprecated'
        | 'active'
        | 'archived'
        | 'inferred'
        | 'superseded'
        | 'pre-alpha';
      since?: string;
      version?: string;
      assurance_level?: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
      owner?: string;
      /**
       * @minItems 1
       */
      invariants?: [
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
      edge_cases?: {
        id: string;
        text: string;
        test_linkage?: string[];
      }[];
      superseded_by?: string;
      linked_objectives?: string[];
      ring?: number;
      /**
       * Structured declared inputs (name/description rows); optional schemaRef ties rows to JSON Schema.
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
      exports?: string[];
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
       * Namespaced extensions; keys must not collide with chassis top-level reserved names.
       */
      extensions?: {
        [k: string]: {};
      };
      /**
       * Primary executable entry (path from repo root or package root).
       */
      entrypoint: string;
      /**
       * Human-readable synopsis of CLI arguments.
       */
      argsSummary: string;
      /**
       * Vendor extension namespace. Values must be objects.
       *
       * This interface was referenced by `undefined`'s JSON-Schema definition
       * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
       */
      [k: string]: {};
    }
  | {
      kind: 'component';
      name?: string;
      purpose?: string;
      status?:
        | 'draft'
        | 'experimental'
        | 'stable'
        | 'deprecated'
        | 'active'
        | 'archived'
        | 'inferred'
        | 'superseded'
        | 'pre-alpha';
      since?: string;
      version?: string;
      assurance_level?: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
      owner?: string;
      /**
       * @minItems 1
       */
      invariants?: [
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
      edge_cases?: {
        id: string;
        text: string;
        test_linkage?: string[];
      }[];
      superseded_by?: string;
      linked_objectives?: string[];
      ring?: number;
      /**
       * Structured declared inputs (name/description rows); optional schemaRef ties rows to JSON Schema.
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
      exports?: string[];
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
       * Namespaced extensions; keys must not collide with chassis top-level reserved names.
       */
      extensions?: {
        [k: string]: {};
      };
      /**
       * UI taxonomy for component contracts (distinct from CONTRACT.kind).
       */
      ui_taxonomy:
        | 'presentational'
        | 'container'
        | 'layout'
        | 'form'
        | 'navigation'
        | 'feedback'
        | 'data-display'
        | 'overlay';
      props: {
        description: string;
        [k: string]: unknown;
      }[];
      events: {
        description: string;
        [k: string]: unknown;
      }[];
      slots: {
        description: string;
        [k: string]: unknown;
      }[];
      states: {
        loading: {
          supported: boolean;
          strategy: string;
        };
        error: {
          supported: boolean;
          strategy: string;
        };
        empty: {
          supported: boolean;
          strategy: string;
        };
        disabled?: {
          supported: boolean;
        };
      };
      accessibility: {
        role?: string;
        keyboardShortcuts?: {
          key: string;
          action: string;
        }[];
        screenReaderNotes?: string;
      };
      dependencies: string[];
      responsive?: {
        description: string;
        [k: string]: unknown;
      }[];
      theme?: {
        tokens?: string[];
      };
      /**
       * Vendor extension namespace. Values must be objects.
       *
       * This interface was referenced by `undefined`'s JSON-Schema definition
       * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
       */
      [k: string]: {};
    }
  | {
      kind: 'endpoint';
      name?: string;
      purpose?: string;
      status?:
        | 'draft'
        | 'experimental'
        | 'stable'
        | 'deprecated'
        | 'active'
        | 'archived'
        | 'inferred'
        | 'superseded'
        | 'pre-alpha';
      since?: string;
      version?: string;
      assurance_level?: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
      owner?: string;
      /**
       * @minItems 1
       */
      invariants?: [
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
      edge_cases?: {
        id: string;
        text: string;
        test_linkage?: string[];
      }[];
      superseded_by?: string;
      linked_objectives?: string[];
      ring?: number;
      /**
       * Structured declared inputs (name/description rows); optional schemaRef ties rows to JSON Schema.
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
      exports?: string[];
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
       * Namespaced extensions; keys must not collide with chassis top-level reserved names.
       */
      extensions?: {
        [k: string]: {};
      };
      method: string;
      path: string;
      auth: {
        required: boolean;
        methods: string[];
        scopes?: string[];
      };
      request: {
        headers: {
          name: string;
          type: string;
          description: string;
          required?: boolean;
        }[];
        pathParams: {
          name: string;
          type: string;
          description: string;
          required?: boolean;
        }[];
        queryParams: {
          name: string;
          type: string;
          description: string;
          required?: boolean;
        }[];
        body?: {
          contentType?: string;
          schemaRef?: string;
        };
      };
      response: {
        success: {
          statusCode: number;
          description: string;
          schemaRef?: string;
        };
        errors: {
          code: string;
          description: string;
        }[];
      };
      /**
       * Vendor extension namespace. Values must be objects.
       *
       * This interface was referenced by `undefined`'s JSON-Schema definition
       * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
       */
      [k: string]: {};
    }
  | {
      kind: 'entity';
      name?: string;
      purpose?: string;
      status?:
        | 'draft'
        | 'experimental'
        | 'stable'
        | 'deprecated'
        | 'active'
        | 'archived'
        | 'inferred'
        | 'superseded'
        | 'pre-alpha';
      since?: string;
      version?: string;
      assurance_level?: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
      owner?: string;
      /**
       * @minItems 1
       */
      invariants?: [
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
      edge_cases?: {
        id: string;
        text: string;
        test_linkage?: string[];
      }[];
      superseded_by?: string;
      linked_objectives?: string[];
      ring?: number;
      /**
       * Structured declared inputs (name/description rows); optional schemaRef ties rows to JSON Schema.
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
      exports?: string[];
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
       * Namespaced extensions; keys must not collide with chassis top-level reserved names.
       */
      extensions?: {
        [k: string]: {};
      };
      fields: FieldDefinition[];
      relationships: {
        name: string;
        kind: 'has_one' | 'has_many' | 'belongs_to' | 'many_to_many';
        target: string;
        foreignKey?: string;
        cascade?: 'none' | 'delete' | 'nullify' | 'restrict';
      }[];
      indexes: {
        /**
         * @minItems 1
         */
        fields: [string, ...string[]];
        unique?: boolean;
        name?: string;
      }[];
      timestamps: {
        createdAt: boolean;
        updatedAt: boolean;
        deletedAt: boolean;
      };
      versioning?: {
        strategy?: 'none' | 'optimistic' | 'event-sourced';
      };
      /**
       * Vendor extension namespace. Values must be objects.
       *
       * This interface was referenced by `undefined`'s JSON-Schema definition
       * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
       */
      [k: string]: {};
    }
  | {
      kind: 'service';
      name?: string;
      purpose?: string;
      status?:
        | 'draft'
        | 'experimental'
        | 'stable'
        | 'deprecated'
        | 'active'
        | 'archived'
        | 'inferred'
        | 'superseded'
        | 'pre-alpha';
      since?: string;
      version?: string;
      assurance_level?: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
      owner?: string;
      /**
       * @minItems 1
       */
      invariants?: [
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
      edge_cases?: {
        id: string;
        text: string;
        test_linkage?: string[];
      }[];
      superseded_by?: string;
      linked_objectives?: string[];
      ring?: number;
      /**
       * Structured declared inputs (name/description rows); optional schemaRef ties rows to JSON Schema.
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
      exports?: string[];
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
       * Namespaced extensions; keys must not collide with chassis top-level reserved names.
       */
      extensions?: {
        [k: string]: {};
      };
      protocol: 'rest' | 'grpc' | 'graphql' | 'websocket' | 'message-queue' | 'internal';
      endpoints: string[];
      consumes: string[];
      produces: string[];
      resilience: {
        timeout: string;
        retry?: {
          maxAttempts: number;
        };
        notes?: string;
      };
      /**
       * Vendor extension namespace. Values must be objects.
       *
       * This interface was referenced by `undefined`'s JSON-Schema definition
       * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
       */
      [k: string]: {};
    }
  | {
      kind: 'event-stream';
      name?: string;
      purpose?: string;
      status?:
        | 'draft'
        | 'experimental'
        | 'stable'
        | 'deprecated'
        | 'active'
        | 'archived'
        | 'inferred'
        | 'superseded'
        | 'pre-alpha';
      since?: string;
      version?: string;
      assurance_level?: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
      owner?: string;
      /**
       * @minItems 1
       */
      invariants?: [
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
      edge_cases?: {
        id: string;
        text: string;
        test_linkage?: string[];
      }[];
      superseded_by?: string;
      linked_objectives?: string[];
      ring?: number;
      /**
       * Structured declared inputs (name/description rows); optional schemaRef ties rows to JSON Schema.
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
      exports?: string[];
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
       * Namespaced extensions; keys must not collide with chassis top-level reserved names.
       */
      extensions?: {
        [k: string]: {};
      };
      source: string;
      payload: {
        fields: FieldDefinition[];
      };
      delivery: {
        guarantee: 'at-most-once' | 'at-least-once' | 'exactly-once';
        ordering: 'none' | 'per-key' | 'total';
        partitionKey?: string;
      };
      consumers: string[];
      /**
       * Vendor extension namespace. Values must be objects.
       *
       * This interface was referenced by `undefined`'s JSON-Schema definition
       * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
       */
      [k: string]: {};
    }
  | {
      kind: 'feature-flag';
      name?: string;
      purpose?: string;
      status?:
        | 'draft'
        | 'experimental'
        | 'stable'
        | 'deprecated'
        | 'active'
        | 'archived'
        | 'inferred'
        | 'superseded'
        | 'pre-alpha';
      since?: string;
      version?: string;
      assurance_level?: 'declared' | 'coherent' | 'verified' | 'enforced' | 'observed';
      owner?: string;
      /**
       * @minItems 1
       */
      invariants?: [
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
      edge_cases?: {
        id: string;
        text: string;
        test_linkage?: string[];
      }[];
      superseded_by?: string;
      linked_objectives?: string[];
      ring?: number;
      /**
       * Structured declared inputs (name/description rows); optional schemaRef ties rows to JSON Schema.
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
      exports?: string[];
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
       * Namespaced extensions; keys must not collide with chassis top-level reserved names.
       */
      extensions?: {
        [k: string]: {};
      };
      /**
       * Flag value type.
       */
      type: 'boolean' | 'string' | 'number' | 'json';
      /**
       * Default flag value when targeting misses (any JSON type).
       */
      defaultValue: {
        [k: string]: unknown;
      };
      targeting: {
        description: string;
        conditions?: {
          attribute: string;
          operator: string;
          value?: unknown;
        }[];
        value?: unknown;
        percentage?: number;
      }[];
      metrics: string[];
      expiration?: string;
      /**
       * Vendor extension namespace. Values must be objects.
       *
       * This interface was referenced by `undefined`'s JSON-Schema definition
       * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
       */
      [k: string]: {};
    }
);

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
