//! Minimal SARIF 2.1.0 ingestion (Semgrep + CodeQL).

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::diagnostic::{Diagnostic, Severity, Violated};

use super::summary::{ScannerSummary, ScannerTool};
use super::CH_SCANNER_SARIF_MALFORMED;

/// Ingest SARIF JSON bytes into a [`ScannerSummary`] with SHA-256 of the raw bytes.
pub fn ingest_sarif_bytes(tool: ScannerTool, bytes: &[u8]) -> Result<ScannerSummary, String> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let sarif_sha256 = format!("{:x}", hasher.finalize());
    let v: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("{CH_SCANNER_SARIF_MALFORMED}: {e}"))?;
    ingest_sarif_value(tool, &v, sarif_sha256)
}

fn ingest_sarif_value(
    tool: ScannerTool,
    root: &Value,
    sarif_sha256: String,
) -> Result<ScannerSummary, String> {
    let runs = root
        .get("runs")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{CH_SCANNER_SARIF_MALFORMED}: missing runs[]"))?;
    let run0 = runs
        .first()
        .ok_or_else(|| format!("{CH_SCANNER_SARIF_MALFORMED}: empty runs[]"))?;

    let tool_version = run0
        .get("tool")
        .and_then(|t| t.get("driver"))
        .and_then(|d| d.get("version"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let rules = run0
        .get("tool")
        .and_then(|t| t.get("driver"))
        .and_then(|d| d.get("rules"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let results = run0
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{CH_SCANNER_SARIF_MALFORMED}: missing results[]"))?;

    let mut diagnostics = Vec::new();
    for r in results {
        if let Some(d) = result_to_diagnostic(tool, r, &rules)? {
            diagnostics.push(d);
        }
    }

    let mut summary = ScannerSummary {
        tool,
        tool_version,
        sarif_sha256,
        run_id: None,
        total: diagnostics.len(),
        errors: 0,
        warnings: 0,
        infos: 0,
        diagnostics,
    };
    summary.recompute_counts();
    Ok(summary)
}

fn result_to_diagnostic(
    tool: ScannerTool,
    result: &Value,
    rules: &[Value],
) -> Result<Option<Diagnostic>, String> {
    let level = result
        .get("level")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let severity = match level.as_deref() {
        Some("error") => Severity::Error,
        Some("warning") => Severity::Warning,
        Some("note") | Some("none") | Some("notapplicable") | None => Severity::Info,
        Some(_) => Severity::Info,
    };

    let rule_id_raw = result
        .get("ruleId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| rule_id_from_index(result, rules))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let rule_id = super::CH_SCANNER_FINDING.to_string();

    let message = result
        .get("message")
        .and_then(|m| m.get("text"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut msg = s.to_string();
            if msg.chars().count() > 160 {
                msg = msg.chars().take(157).collect::<String>() + "...";
            }
            msg
        })
        .unwrap_or_else(|| "scanner finding".to_string());

    let (subject, location) = primary_location(result);

    let source = tool.source_label().to_string();
    let detail = serde_json::json!({
        "tool": source,
        "sarifRuleId": rule_id_raw,
    });
    Ok(Some(Diagnostic {
        rule_id,
        severity,
        message,
        source: Some(source),
        subject,
        violated: Some(Violated {
            convention: "ADR-0033".to_string(),
        }),
        docs: None,
        fix: None,
        location,
        detail: Some(detail),
    }))
}

fn rule_id_from_index(result: &Value, rules: &[Value]) -> Option<String> {
    let idx = result.get("ruleIndex")?.as_u64()? as usize;
    let rule = rules.get(idx)?;
    rule.get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| rule.get("name").and_then(Value::as_str).map(str::to_string))
}

fn primary_location(result: &Value) -> (Option<String>, Option<Value>) {
    let Some(locs) = result.get("locations").and_then(Value::as_array) else {
        return (None, None);
    };
    let Some(first) = locs.first() else {
        return (None, None);
    };
    let Some(phys) = first.get("physicalLocation") else {
        return (None, None);
    };
    let uri = phys
        .get("artifactLocation")
        .and_then(|a| a.get("uri"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(uri_ref) = uri.clone() else {
        return (None, None);
    };
    let mut loc_obj = serde_json::Map::new();
    loc_obj.insert("path".to_string(), Value::String(uri_ref.clone()));
    if let Some(reg) = phys.get("region") {
        if let Some(range_val) = range_object_from_region(reg) {
            loc_obj.insert("range".to_string(), range_val);
        }
    }
    (Some(uri_ref), Some(Value::Object(loc_obj)))
}

fn range_object_from_region(reg: &Value) -> Option<Value> {
    let start_line = reg.get("startLine")?.as_u64()? as i64;
    let start_col = reg
        .get("startColumn")
        .and_then(|v| v.as_u64())
        .map(|u| u as i64);
    let end_line = reg
        .get("endLine")
        .and_then(|v| v.as_u64())
        .map(|u| u as i64);
    let end_col = reg
        .get("endColumn")
        .and_then(|v| v.as_u64())
        .map(|u| u as i64);

    let mut start = serde_json::Map::new();
    start.insert("line".to_string(), json_i64(start_line));
    if let Some(c) = start_col {
        start.insert("column".to_string(), json_i64(c));
    }
    let start_v = Value::Object(start);

    let range = if let Some(el) = end_line {
        let mut end = serde_json::Map::new();
        end.insert("line".to_string(), json_i64(el));
        if let Some(c) = end_col {
            end.insert("column".to_string(), json_i64(c));
        }
        let mut r = serde_json::Map::new();
        r.insert("start".to_string(), start_v);
        r.insert("end".to_string(), Value::Object(end));
        r
    } else {
        let mut r = serde_json::Map::new();
        r.insert("start".to_string(), start_v);
        r
    };
    Some(Value::Object(range))
}

fn json_i64(n: i64) -> Value {
    serde_json::Number::from(n).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> Vec<u8> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/scanner")
            .join(name);
        std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
        // nosemgrep: chassis-no-panic-runtime-core
    }

    #[test]
    fn semgrep_clean_roundtrip_counts() {
        let bytes = fixture("semgrep-clean.sarif");
        let s = ingest_sarif_bytes(ScannerTool::Semgrep, &bytes).expect("ingest");
        assert_eq!(s.errors, 0);
        assert_eq!(s.total, s.diagnostics.len());
    }

    #[test]
    fn semgrep_error_finding_has_error_severity() {
        let bytes = fixture("semgrep-error.sarif");
        let s = ingest_sarif_bytes(ScannerTool::Semgrep, &bytes).expect("ingest");
        assert!(s.errors >= 1);
        assert!(s.diagnostics.iter().any(|d| d.severity == Severity::Error));
    }

    #[test]
    fn malformed_sarif_errors() {
        let bytes = fixture("malformed.sarif");
        assert!(ingest_sarif_bytes(ScannerTool::Semgrep, &bytes).is_err());
    }
}
