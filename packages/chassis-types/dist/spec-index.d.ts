/**
 * AUTO-GENERATED — do not edit.
 * Source: schemas/spec-index.schema.json
 */

/**
 * Deterministic index of Spec Kit intent for Chassis trace, policy, and linker surfaces.
 */
export interface SpecIndex {
  version: 1;
  chassis_preset_version: 1;
  feature_id: string;
  title?: string;
  summary?: string;
  constitution_principles: {
    id: string;
    text: string;
  }[];
  non_goals: string[];
  requirements: {
    id: string;
    title: string;
    description: string;
    /**
     * @minItems 1
     */
    acceptance_criteria: [string, ...string[]];
    claim_ids: string[];
    related_task_ids: string[];
    touched_paths: string[];
  }[];
  tasks: {
    id: string;
    title: string;
    description?: string;
    depends_on: string[];
    parallel_group?: string;
    touched_paths: string[];
  }[];
  implementation_constraints: string[];
}
