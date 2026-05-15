//! Chassis core: typed contract / ADR / exemption / coherence vocabulary + JSON Schema validators.

#![forbid(unsafe_code)]

pub mod adr;
pub mod authority_index;
pub mod coherence_report;
pub mod contract;
pub mod diagnostic;
pub mod diff;
pub mod exempt;
pub mod exemption;
pub mod field_definition;
pub mod fingerprint;
pub mod tag_ontology;
pub mod validators;

#[cfg(test)]
mod adr_kernel_rule_binding_tests;
