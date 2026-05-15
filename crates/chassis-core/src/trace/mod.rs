//! Trace graph extraction and composition (ADR-0023).

use std::sync::LazyLock;

use serde_json::Value;

pub mod extract;
pub mod graph;
pub mod render;
pub mod types;

mod backend;
pub use backend::TraceExtractBackend;

pub use graph::{
    build_trace_graph, build_trace_graph_at, build_trace_graph_at_with, RULE_NOT_IN_CONTRACT,
};
pub use render::render_mermaid;
pub use types::{ClaimContractKind, ClaimNode, ClaimSite, SiteKind, TraceError, TraceGraph};

static TRACE_SCHEMA_STR: &str = include_str!("../../../../schemas/trace-graph.schema.json");

static TRACE_COMPILED: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(TRACE_SCHEMA_STR).expect("trace-graph schema JSON");
    jsonschema::validator_for(&schema).expect("compile trace-graph schema")
});

/// Validate a serialized trace graph against `schemas/trace-graph.schema.json`.
pub fn validate_trace_graph(graph: &TraceGraph) -> Result<(), Vec<String>> {
    let v = serde_json::to_value(graph).expect("serde TraceGraph");
    validate_trace_graph_json(&v)
}

/// Validate a JSON value against `schemas/trace-graph.schema.json`.
pub fn validate_trace_graph_json(value: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = TRACE_COMPILED
        .iter_errors(value)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
