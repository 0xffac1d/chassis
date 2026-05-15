//! Chassis core: typed contract / ADR / exemption vocabulary + JSON Schema validators, diff, fingerprint, drift.

#![forbid(unsafe_code)]

pub mod adr;
pub mod contract;
pub mod diagnostic;
pub mod diff;
pub mod drift;
pub mod exempt;
pub mod exemption;
pub mod fingerprint;
pub mod validators;

#[cfg(test)]
mod adr_kernel_rule_binding_tests;
