//! SARIF ingestion + normalized scanner summaries for governance exports.

mod sarif;
pub mod summary;

pub use sarif::ingest_sarif_bytes;
pub use summary::{ScannerSummary, ScannerTool};

use serde_json::Value;

use crate::artifact::validate_scanner_summary_value;

/// Stable rule id for SARIF-normalized findings (ADR-0033).
pub const CH_SCANNER_FINDING: &str = "CH-SCANNER-FINDING";

/// Diagnostic rule id when SARIF cannot be parsed.
pub const CH_SCANNER_SARIF_MALFORMED: &str = "CH-SCANNER-SARIF-MALFORMED";

/// Load `dist/scanner-{semgrep,codeql}.json` under `base` (repo root or
/// evidence download root) when present; validate against schema.
pub fn load_scanner_summaries_from_repo(
    repo: &std::path::Path,
) -> Result<Vec<ScannerSummary>, String> {
    load_scanner_summaries_from_dist_parent(repo)
}

/// Same layout as [`load_scanner_summaries_from_repo`]: `base/dist/scanner-*.json`.
pub fn load_scanner_summaries_from_dist_parent(
    base: &std::path::Path,
) -> Result<Vec<ScannerSummary>, String> {
    let mut out = Vec::new();
    for (tool, filename) in [
        (ScannerTool::Semgrep, "scanner-semgrep.json"),
        (ScannerTool::Codeql, "scanner-codeql.json"),
    ] {
        let p = base.join("dist").join(filename);
        if !p.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
        let v: Value =
            serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", p.display()))?;
        validate_scanner_summary_value(&v)
            .map_err(|errs| format!("schema {}: {errs:?}", p.display()))?;
        let s: ScannerSummary =
            serde_json::from_value(v).map_err(|e| format!("decode {}: {e}", p.display()))?;
        if s.tool != tool {
            return Err(format!(
                "{}: tool mismatch (expected {:?}, got {:?})",
                p.display(),
                tool,
                s.tool
            ));
        }
        out.push(s);
    }
    Ok(out)
}

/// Fail closed unless both normalized scanner summaries exist under `base/dist/`.
pub fn verify_scanner_evidence_dir(base: &std::path::Path) -> Result<(), String> {
    let dist = base.join("dist");
    for fname in ["scanner-semgrep.json", "scanner-codeql.json"] {
        let p = dist.join(fname);
        if !p.is_file() {
            return Err(format!(
                "missing required scanner evidence: {}",
                p.display()
            ));
        }
    }
    let loaded = load_scanner_summaries_from_dist_parent(base)?;
    if loaded.len() != 2 {
        return Err(format!(
            "expected two scanner summaries under {}, found {}",
            dist.display(),
            loaded.len()
        ));
    }
    Ok(())
}
