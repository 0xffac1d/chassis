//! Re-validate any governance artifact against its canonical schema.
//!
//! Usage: `validate_artifact <kind> <path>` where `<kind>` is one of:
//! `trace-graph`, `drift-report`, `diagnostic`, `release-gate`,
//! `in-toto-statement`, `dsse-envelope`.
//!
//! Used by `scripts/self-attest.sh` to belt-and-suspenders every emitted
//! artifact, so a regression in any single emitter still fails the script.

use std::process::ExitCode;

use chassis_core::artifact::{
    validate_diagnostic_value, validate_drift_report_value, validate_dsse_envelope_value,
    validate_in_toto_statement_value, validate_release_gate_value, validate_trace_graph_value,
};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let kind = match args.next() {
        Some(k) => k,
        None => {
            eprintln!("usage: validate_artifact <kind> <path>");
            return ExitCode::from(2);
        }
    };
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: validate_artifact <kind> <path>");
            return ExitCode::from(2);
        }
    };

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse {path}: {e}");
            return ExitCode::from(2);
        }
    };

    let result = match kind.as_str() {
        "trace-graph" => validate_trace_graph_value(&value),
        "drift-report" => validate_drift_report_value(&value),
        "diagnostic" => validate_diagnostic_value(&value),
        "release-gate" => validate_release_gate_value(&value),
        "in-toto-statement" => validate_in_toto_statement_value(&value),
        "dsse-envelope" => validate_dsse_envelope_value(&value),
        other => {
            eprintln!("unknown artifact kind: {other}");
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => {
            println!("ok {kind} {path}");
            ExitCode::SUCCESS
        }
        Err(errs) => {
            eprintln!("schema validation failed for {kind} {path}:");
            for e in errs {
                eprintln!("  - {e}");
            }
            ExitCode::from(1)
        }
    }
}
