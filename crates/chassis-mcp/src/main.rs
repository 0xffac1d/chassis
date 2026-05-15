#![forbid(unsafe_code)]

//! Exactly six newline-delimited JSON-RPC 2.0 methods over stdio (scoped Wave 5 surface).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::{json, Value};

use chassis_core::attest::CH_ATTEST_NOT_FOUND;
use chassis_core::contract::validate_metadata_contract;
use chassis_core::diagnostic::{Diagnostic, Severity, Violated};
use chassis_core::diff;
use chassis_core::drift::report::build_drift_report;
use chassis_core::exempt::{list, ListFilter, Registry as ExemptionRegistry};
use chassis_core::trace::build_trace_graph;

fn repo_root_from_env_or_cwd() -> PathBuf {
    let fallback = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let p = match std::env::var_os("CHASSIS_REPO_ROOT") {
        Some(s) => PathBuf::from(s),
        None => fallback,
    };
    std::fs::canonicalize(&p).unwrap_or(p)
}

fn load_registry_optional(repo: &Path) -> Option<ExemptionRegistry> {
    let p = repo.join(".chassis/exemptions.yaml");
    let raw = std::fs::read_to_string(p).ok()?;
    let y: serde_yaml::Value = serde_yaml::from_str(&raw).ok()?;
    let j = serde_json::to_value(y).ok()?;
    serde_json::from_value(j).ok()
}

fn diagnostics_from_contract_errors(messages: &[String]) -> Vec<Diagnostic> {
    messages
        .iter()
        .map(|msg| Diagnostic {
            rule_id: "CH-RUST-METADATA-CONTRACT".to_string(),
            severity: Severity::Error,
            message: msg.clone(),
            source: Some("mcp.validate_contract".into()),
            subject: None,
            violated: Some(Violated {
                convention: "chassis.contract".into(),
            }),
            docs: None,
            fix: None,
            location: None,
            detail: None,
        })
        .collect()
}

fn send_line(out: &mut impl Write, obj: &Value) {
    if let Ok(s) = serde_json::to_string(obj) {
        let _ = writeln!(out, "{s}");
        let _ = out.flush();
    }
}

fn rpc_parse_error(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32700, "message": "Parse error" }
    })
}

fn rpc_error(id: &Value, code: i64, msg: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id.clone(), "error": { "code": code, "message": msg.into() }})
}

fn rpc_result(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id.clone(), "result": result})
}

fn require_params(req: &Value) -> Result<&serde_json::Map<String, Value>, Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let params = req
        .get("params")
        .ok_or_else(|| rpc_error(&id, -32602, "missing params"))?;
    params
        .as_object()
        .ok_or_else(|| rpc_error(&id, -32602, "params must be object"))
}

fn dispatch(repo: &Path, req: &Value) -> Value {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = match req.get("method").and_then(Value::as_str) {
        Some(m) => m,
        None => return rpc_error(&id, -32600, "Invalid Request"),
    };

    let run: Result<Value, Value> = (|| -> Result<Value, Value> {
        match method {
            "validate_contract" => {
                let pmap = require_params(req)?;
                let yaml = pmap
                    .get("yaml")
                    .and_then(Value::as_str)
                    .ok_or_else(|| rpc_error(&id, -32602, "params.yaml required"))?;
                let yaml_val: serde_yaml::Value = serde_yaml::from_str(yaml)
                    .map_err(|e| rpc_error(&id, -32602, format!("yaml: {e}")))?;
                let v = serde_json::to_value(yaml_val)
                    .map_err(|e| rpc_error(&id, -32602, format!("to json: {e}")))?;
                match validate_metadata_contract(&v) {
                    Ok(()) => Ok(json!({ "ok": true, "diagnostics": [] })),
                    Err(msgs) => Ok(
                        json!({ "ok": false, "diagnostics": diagnostics_from_contract_errors(&msgs) }),
                    ),
                }
            }
            "diff_contracts" => {
                let pmap = require_params(req)?;
                let old = pmap
                    .get("old")
                    .ok_or_else(|| rpc_error(&id, -32602, "params.old required"))?;
                let new = pmap
                    .get("new")
                    .ok_or_else(|| rpc_error(&id, -32602, "params.new required"))?;
                diff::diff(old, new)
                    .map(|r| serde_json::to_value(r).unwrap())
                    .map_err(|e| rpc_error(&id, -32603, format!("diff: {e:?}")))
            }
            "trace_claim" => {
                let pmap = require_params(req)?;
                let claim_id = pmap
                    .get("claim_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| rpc_error(&id, -32602, "params.claim_id required"))?;
                let g = build_trace_graph(repo)
                    .map_err(|e| rpc_error(&id, -32603, format!("trace: {e}")))?;
                Ok(match g.claims.get(claim_id) {
                    Some(n) => serde_json::to_value(n).unwrap(),
                    None => Value::Null,
                })
            }
            "drift_report" => {
                let trace = build_trace_graph(repo)
                    .map_err(|e| rpc_error(&id, -32603, format!("trace: {e}")))?;
                let r = build_drift_report(repo, &trace, Utc::now())
                    .map_err(|e| rpc_error(&id, -32603, format!("drift: {e}")))?;
                Ok(serde_json::to_value(r).unwrap())
            }
            "release_gate" => {
                let path = repo.join("release-gate.dsse");
                let raw = std::fs::read_to_string(&path)
                    .map_err(|_| rpc_error(&id, -32603, CH_ATTEST_NOT_FOUND))?;
                let v: Value = serde_json::from_str(&raw)
                    .map_err(|_| rpc_error(&id, -32603, CH_ATTEST_NOT_FOUND))?;
                Ok(v)
            }
            "list_exemptions" => {
                let filter_rule = req
                    .get("params")
                    .and_then(|p| p.as_object())
                    .and_then(|m| m.get("rule_id"))
                    .and_then(Value::as_str);
                match load_registry_optional(repo) {
                    None => Ok(Value::Array(vec![])),
                    Some(reg) => {
                        let today = Utc::now().date_naive();
                        let filt = ListFilter {
                            rule_id: filter_rule.map(ToString::to_string),
                            path: None,
                            active_at: Some(today),
                        };
                        let mut rows: Vec<Value> = Vec::new();
                        for e in list(&reg, filt) {
                            rows.push(
                                serde_json::to_value(e).unwrap_or_else(|_| json!({"id": &e.id})),
                            );
                        }
                        Ok(Value::Array(rows))
                    }
                }
            }
            _ => Err(rpc_error(&id, -32601, "Method not found")),
        }
    })();

    match run {
        Ok(v) => rpc_result(&id, v),
        Err(e) => e,
    }
}

fn main() {
    let repo = repo_root_from_env_or_cwd();
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin);

    for line_result in reader.split(b'\n') {
        let line = match line_result {
            Ok(b) => b,
            Err(_) => break,
        };
        if line.is_empty() || line.iter().all(|c| matches!(c, b' ' | b'\t' | b'\r')) {
            continue;
        }

        let id_from_raw = || {
            serde_json::from_slice::<Value>(&line)
                .ok()
                .and_then(|v| v.get("id").cloned())
                .unwrap_or(Value::Null)
        };

        let req: Value = match serde_json::from_slice(&line) {
            Ok(v) => v,
            Err(_) => {
                send_line(&mut stdout, &rpc_parse_error(id_from_raw()));
                continue;
            }
        };
        send_line(&mut stdout, &dispatch(&repo, &req));
    }
}
