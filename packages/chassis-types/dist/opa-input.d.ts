/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/opa-input.schema.json
 */

/**
 * OPA adapter wrapping Chassis policy-input facts under the standard input key. Chassis does not evaluate Rego.
 */
export interface OpaInput {
  input: {
    version: 1;
    repo: {};
    contracts: unknown[];
    claims: unknown[];
    diagnostics: unknown[];
    exemptions: {};
    drift_summary: {};
  };
}
