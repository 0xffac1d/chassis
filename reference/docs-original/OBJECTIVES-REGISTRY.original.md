# Product objectives registry

Repo-wide **product objectives** (hard vs soft constraints, acceptance criteria, traceability) live in [`config/chassis.objectives.yaml`](../../../config/chassis.objectives.yaml).

- **Schema:** [`schemas/objective/objective-registry.schema.json`](../../../schemas/objective/objective-registry.schema.json)
- **Validation:** `python3 scripts/chassis/validate_objectives.py` (also runs as part of **`chassis validate-all`**)

## Linking module contracts

In any `CONTRACT.yaml`, set:

```yaml
linked_objectives:
  - myapp.objective.example-id
```

Each id must exist in the registry. The validator warns when an objective lists `links.contracts` for a path that does not declare a back-link via `linked_objectives`.

## Module assurance level

Optional epistemic tier on `CONTRACT.yaml`:

```yaml
assurance_level: declared   # declared | coherent | verified | enforced | observed
```

See [`../ROADMAP.md`](../ROADMAP.md) for the intended ladder semantics.

## Stable claim IDs

For durable `test_linkage`, add **`id`** on object-form invariants and edge cases, then reference that id in `test_linkage[].claim_id`. Plain-string claims continue to work; excerpt-based `claim_id` values are legacy. Details: [`contract-testing.md`](contract-testing.md).
