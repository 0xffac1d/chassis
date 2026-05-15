/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/contract-kinds/cli.schema.json
 */

/**
 * Schema for kind=cli contracts. Requires entrypoint + argsSummary; allows structured subcommands.
 */
export interface Cli {
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
   * This interface was referenced by `Cli`'s JSON-Schema definition
   * via the `patternProperty` "^x-[A-Za-z0-9_.-]+$".
   */
  [k: string]: {};
}
