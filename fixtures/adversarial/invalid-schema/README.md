# invalid-schema (negative fixture)

Ships a `CONTRACT.yaml` that intentionally violates the metadata JSON Schema:
missing required fields, illegal `kind`, plus extra `additionalProperties`.

The consumer fixture matrix expects `chassis validate` to exit non-zero with
diagnostics when run against this tree. Bootstrap is intentionally skipped
because the fixture preseeds a manifest; we assert the **validator's** negative
path, not scaffolding.
