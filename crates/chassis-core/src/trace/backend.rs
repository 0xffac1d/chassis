//! Trace extraction backend: regex (line-oriented) vs tree-sitter (comment-aware).
#![forbid(unsafe_code)]

/// How `@claim` sites are discovered in Rust/TypeScript sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceExtractBackend {
    /// Historical line scanner (ADR-0023 baseline).
    #[default]
    Regex,
    /// Parse with tree-sitter; extract `line_comment` nodes (ADR-0028).
    TreeSitter,
}
