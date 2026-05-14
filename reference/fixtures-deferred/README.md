# Deferred fixtures

These fixtures presuppose machinery this project does not (yet) have:

- `illegal-layout/` — needs a directory-layout validator. Its `config/chassis.layout.yaml` describes a policy schema with no current enforcer. Move back to `fixtures/adversarial/` if/when a layout validator lands.
- `brownfield-messy/` — needs a `chassis bootstrap` CLI in `metadata-only` mode (deliberately dropped during salvage). Move back to `fixtures/` if/when brownfield-onboarding is reimplemented.

**Not driven by any test.** Design reference for future work.
