//! Normalized scanner evidence bundle (`schemas/scanner-summary.schema.json`).

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;

/// Supported static analysis tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScannerTool {
    Semgrep,
    Codeql,
}

impl ScannerTool {
    pub fn source_label(self) -> &'static str {
        match self {
            ScannerTool::Semgrep => "semgrep",
            ScannerTool::Codeql => "codeql",
        }
    }
}

/// Chassis-normalized scanner run summary (ingested from SARIF).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannerSummary {
    pub tool: ScannerTool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
    pub sarif_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl ScannerSummary {
    /// Recompute severity counters from [`Self::diagnostics`] (e.g. after exemption application).
    pub fn recompute_counts(&mut self) {
        self.total = self.diagnostics.len();
        self.errors = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::diagnostic::Severity::Error)
            .count();
        self.warnings = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::diagnostic::Severity::Warning)
            .count();
        self.infos = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::diagnostic::Severity::Info)
            .count();
    }
}
