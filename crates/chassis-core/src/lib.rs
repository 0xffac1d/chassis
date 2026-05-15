//! Chassis core: typed contract / ADR / exemption vocabulary + JSON Schema validators, diff,
//! fingerprint, and drift/trace helpers used by forthcoming Wave tooling.

#![forbid(unsafe_code)]

pub mod artifact;
pub mod attest;
pub mod contract;
pub mod diagnostic;
pub mod diagnostic_registry;
pub mod diff;
pub mod drift;
pub mod exempt;
pub mod exemption;
pub mod exports;
pub mod fingerprint;
pub mod gate;
pub mod supply;
pub mod trace;
pub mod validators;

#[cfg(test)]
mod adr_kernel_rule_binding_tests;
