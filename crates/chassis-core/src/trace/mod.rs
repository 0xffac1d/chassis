//! Trace graph extraction and composition (ADR-0023).

pub mod extract;
pub mod graph;
pub mod render;
pub mod types;

pub use graph::{build_trace_graph, build_trace_graph_at, RULE_NOT_IN_CONTRACT};
pub use types::{ClaimContractKind, ClaimNode, ClaimSite, SiteKind, TraceError, TraceGraph};
