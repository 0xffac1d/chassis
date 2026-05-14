# Historical ADRs from the original Chassis

These 32 ADRs document decisions made in the original Chassis codebase. They are reference material for the rebuild — not canonical decisions for this project.

Several are about features we deliberately dropped:
- ADR-0006 (codegen policy) — multi-language codegen, out of scope
- ADR-0013 (runtime library split) — runtime crate deleted
- ADR-0018, ADR-0019 (private registries) — out of scope
- ADR-0021 (capability marker) — proc macro deleted
- ADR-0024, ADR-0025, ADR-0029 — self-governance ratchets specific to the original
- ADR-0030, ADR-0031, ADR-0032 — about CLI surfaces we dropped

A few are worth re-authoring as new ADRs in `docs/adr/` once we make the equivalent decisions for this project:
- ADR-0002 (assurance ladder)
- ADR-0003 (claims model)
- ADR-0004 (exemption quota)
- ADR-0008 (schema versioning)
- ADR-0011 (rule ID stability)

Do not assume any decision here binds the new project until it has been re-adopted as a new ADR.
