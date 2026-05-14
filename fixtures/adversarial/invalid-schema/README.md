# invalid-schema (negative fixture)

Ships a `CONTRACT.yaml` that intentionally violates `schemas/contract.schema.json`: missing required fields, illegal `kind`, plus extra `additionalProperties`.

Used to assert the **validator's** negative path. The current verification point is `chassis-core`'s `CanonicalMetadataContractValidator`, which must reject this file with at least one diagnostic. The planned `chassis validate` CLI is expected to exit non-zero against this tree.
