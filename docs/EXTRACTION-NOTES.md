Phase4: cargo check exit and first errors recorded below
    Checking socket2 v0.6.3
    Checking http-body-util v0.1.3
    Checking num-bigint v0.4.6
    Checking num-iter v0.1.45
    Checking parking_lot v0.12.5
    Checking tokio v1.52.3
    Checking regex-automata v0.4.14
    Checking num-rational v0.4.2
    Checking num v0.4.3
    Checking fraction v0.15.4
   Compiling synstructure v0.13.2
   Compiling zerovec-derive v0.11.3
   Compiling displaydoc v0.2.5
   Compiling serde_derive v1.0.228
   Compiling ref-cast-impl v1.0.25
   Compiling zerofrom-derive v0.1.7
   Compiling yoke-derive v0.8.2
    Checking zerofrom v0.1.8
    Checking yoke v0.8.2
    Checking zerotrie v0.2.4
    Checking hyper v1.9.0
    Checking tower v0.5.3
    Checking zerovec v0.11.6
    Checking fancy-regex v0.14.0
    Checking regex v1.12.3
    Checking tinystr v0.8.3
    Checking potential_utf v0.1.5
    Checking icu_collections v2.2.0
    Checking icu_locale_core v2.2.0
    Checking hyper-util v0.1.20
    Checking icu_provider v2.2.0
    Checking icu_properties v2.2.0
    Checking icu_normalizer v2.2.0
    Checking serde_urlencoded v0.7.1
    Checking fluent-uri v0.3.2
    Checking email_address v0.2.9
    Checking referencing v0.30.0
    Checking idna_adapter v1.2.2
    Checking idna v1.1.0
    Checking url v2.5.8
    Checking tower-http v0.6.10
    Checking reqwest v0.12.28
    Checking jsonschema v0.30.0
    Checking chassis-core v0.1.0 (/mnt/C/chassis/crates/chassis-core)
error[E0433]: cannot find `metadata` in `crate`
  --> chassis-core/src/contract.rs:31:33
   |
31 |     pub debt: Option<Vec<crate::metadata::debt_item::DebtItem>>,
   |                                 ^^^^^^^^ could not find `metadata` in the crate root

error[E0433]: cannot find module or crate `chassis_runtime_api` in this scope
  --> chassis-core/src/validators.rs:49:6
   |
49 | impl chassis_runtime_api::Validator for CanonicalMetadataContractValidator {
   |      ^^^^^^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `chassis_runtime_api`
   |
   = help: if you wanted to use a crate named `chassis_runtime_api`, use `cargo add chassis_runtime_api` to add it to your `Cargo.toml`

For more information about this error, try `rustc --explain E0433`.
error: could not compile `chassis-core` (lib) due to 2 previous errors
Phase7: spec line for rust-minimal/invalid-schema fixtures was malformed in source brief; reconstructed mapping rust-minimal->happy-path/rust-minimal, invalid-schema->adversarial/invalid-schema.
Phase7: missing fixtures/happy-path/rust-minimal/CONTRACT.yaml
Phase7: rust-minimal fixture has no CONTRACT.yaml (only fixture.yaml — a fixture descriptor). The validators.rs test path 'fixtures/happy-path/rust-minimal/CONTRACT.yaml' does NOT resolve. Leaving #[ignore] in place (deviates from Phase 7 instruction to remove it). User must add CONTRACT.yaml to the fixture during the rewrite before enabling that test.
Phase8: .gitignore (as specified) contains 'dist/' which will exclude packages/chassis-types/dist from git — but that dir was deliberately copied. If git-tracking dist is desired, add '!packages/chassis-types/dist/' override or remove 'dist/' from .gitignore.
