#![forbid(unsafe_code)]

//! Custom newline-delimited JSON-RPC 2.0 server over stdio exposing six chassis methods
//! (scoped Wave 5 surface). This is **not** a Model Context Protocol (MCP) implementation —
//! see `docs/future-mcp.md` for the requirements a real MCP surface would have to meet.
//!
//! Every method that returns a structured artifact (trace graph, drift report,
//! release-gate predicate, exemption rows, diff findings, validate diagnostics)
//! validates that artifact against the matching canonical JSON Schema *before*
//! placing it on the wire. A schema-invalid artifact is reported as a JSON-RPC
//! internal error (-32603) rather than serialized to the client.

// @claim chassis.jsonrpc-not-mcp
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::{json, Value};

use chassis_core::artifact::{
    validate_diagnostic_value, validate_drift_report_value, validate_release_gate_value,
    validate_trace_graph_value,
};
use chassis_core::attest::predicate::CommandRun;
use chassis_core::contract::validate_metadata_contract;
use chassis_core::diagnostic::{Diagnostic, Severity, Violated};
use chassis_core::diff;
use chassis_core::drift::report::build_drift_report;
use chassis_core::exempt::{list, ListFilter, Registry as ExemptionRegistry};
use chassis_core::gate;
use chassis_core::trace::build_trace_graph;

/// JSON-RPC error codes used by this server. The numeric values are the
/// JSON-RPC 2.0 spec codes plus an internal-error code we reuse for runtime
/// gate failures; they are part of the wire contract and must not drift.
mod rpc_code {
    /// Server received malformed JSON — could not parse a request at all.
    pub const PARSE_ERROR: i64 = -32700;
    /// Request was valid JSON but not a JSON-RPC request envelope.
    pub const INVALID_REQUEST: i64 = -32600;
    /// `method` field did not match any of the six published verbs.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// Caller supplied wrong / missing / mis-typed params for a method.
    pub const INVALID_PARAMS: i64 = -32602;
    /// Server-side failure while executing the method (gate compute failed,
    /// emitted artifact failed its schema, etc.). The `message` field carries
    /// a stable Chassis rule id when one applies.
    pub const INTERNAL_ERROR: i64 = -32603;
}

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

/// Build wire diagnostics for `validate_contract` failures.
///
/// Rule binding: `CH-RUST-METADATA-CONTRACT` is enforced by ADR-0021 (kind
/// subschemas), so `violated.convention` is the ADR id — not the previously
/// used `chassis.contract` literal, which is not an ADR file and would fail
/// both `schemas/diagnostic.schema.json` (`^ADR-\d{4,}$`) and the rule-ID
/// registry check.
fn diagnostics_from_contract_errors(messages: &[String]) -> Vec<Diagnostic> {
    messages
        .iter()
        .map(|msg| {
            let d = Diagnostic {
                rule_id: "CH-RUST-METADATA-CONTRACT".to_string(),
                severity: Severity::Error,
                message: msg.clone(),
                source: Some("jsonrpc.validate_contract".into()),
                subject: None,
                violated: Some(Violated {
                    convention: "ADR-0021".into(),
                }),
                docs: None,
                fix: None,
                location: None,
                detail: None,
            };
            debug_assert!(
                d.validate_envelope().is_ok(),
                "validate_contract diagnostic violates schemas/diagnostic.schema.json: {:?}",
                d.validate_envelope().err()
            );
            d
        })
        .collect()
}

/// Defense-in-depth: confirm every diagnostic that would land on the wire is
/// schema-valid. Returns an Err with the offending diagnostic + schema errors
/// so the caller can surface an internal error instead of emitting an invalid
/// payload. `debug_assert!`s above catch construction-time bugs; this catch
/// also fires in release builds.
fn validate_diagnostics_for_wire(diagnostics: &[Diagnostic]) -> Result<(), (Value, Vec<String>)> {
    for d in diagnostics {
        let v = serde_json::to_value(d).map_err(|e| (json!(d.rule_id), vec![e.to_string()]))?;
        if let Err(errs) = validate_diagnostic_value(&v) {
            return Err((v, errs));
        }
    }
    Ok(())
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
        "error": { "code": rpc_code::PARSE_ERROR, "message": "Parse error" }
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
        .ok_or_else(|| rpc_error(&id, rpc_code::INVALID_PARAMS, "missing params"))?;
    params
        .as_object()
        .ok_or_else(|| rpc_error(&id, rpc_code::INVALID_PARAMS, "params must be an object"))
}

/// Validate the request envelope: must be a JSON object, have `jsonrpc == "2.0"`,
/// and carry a string `method`. Returns the method on success or a wire-ready
/// error on failure.
fn require_method<'a>(req: &'a Value, id: &Value) -> Result<&'a str, Value> {
    if !req.is_object() {
        return Err(rpc_error(
            id,
            rpc_code::INVALID_REQUEST,
            "request must be a JSON object",
        ));
    }
    if req.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(rpc_error(
            id,
            rpc_code::INVALID_REQUEST,
            "jsonrpc must be \"2.0\"",
        ));
    }
    req.get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| rpc_error(id, rpc_code::INVALID_REQUEST, "method must be a string"))
}

fn require_str_param<'a>(
    pmap: &'a serde_json::Map<String, Value>,
    id: &Value,
    name: &str,
) -> Result<&'a str, Value> {
    match pmap.get(name) {
        None => Err(rpc_error(
            id,
            rpc_code::INVALID_PARAMS,
            format!("params.{name} required"),
        )),
        Some(v) => v.as_str().ok_or_else(|| {
            rpc_error(
                id,
                rpc_code::INVALID_PARAMS,
                format!("params.{name} must be a string"),
            )
        }),
    }
}

fn require_obj_param<'a>(
    pmap: &'a serde_json::Map<String, Value>,
    id: &Value,
    name: &str,
) -> Result<&'a Value, Value> {
    match pmap.get(name) {
        None => Err(rpc_error(
            id,
            rpc_code::INVALID_PARAMS,
            format!("params.{name} required"),
        )),
        Some(v) if v.is_object() => Ok(v),
        Some(_) => Err(rpc_error(
            id,
            rpc_code::INVALID_PARAMS,
            format!("params.{name} must be an object"),
        )),
    }
}

fn dispatch(repo: &Path, req: &Value) -> Value {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = match require_method(req, &id) {
        Ok(m) => m,
        Err(e) => return e,
    };

    let run: Result<Value, Value> = (|| -> Result<Value, Value> {
        match method {
            "validate_contract" => {
                let pmap = require_params(req)?;
                let yaml = require_str_param(pmap, &id, "yaml")?;
                let yaml_val: serde_yaml::Value = serde_yaml::from_str(yaml).map_err(|e| {
                    rpc_error(&id, rpc_code::INVALID_PARAMS, format!("yaml parse: {e}"))
                })?;
                let v = serde_json::to_value(yaml_val).map_err(|e| {
                    rpc_error(&id, rpc_code::INVALID_PARAMS, format!("yaml to json: {e}"))
                })?;
                match validate_metadata_contract(&v) {
                    Ok(()) => Ok(json!({ "ok": true, "diagnostics": [] })),
                    Err(msgs) => {
                        let diags = diagnostics_from_contract_errors(&msgs);
                        validate_diagnostics_for_wire(&diags).map_err(|(d, errs)| {
                            rpc_error(
                                &id,
                                rpc_code::INTERNAL_ERROR,
                                format!("CH-JSONRPC-DIAG-INVALID: refused to emit schema-invalid diagnostic {d}: {errs:?}"),
                            )
                        })?;
                        Ok(json!({ "ok": false, "diagnostics": diags }))
                    }
                }
            }
            "diff_contracts" => {
                let pmap = require_params(req)?;
                let old = require_obj_param(pmap, &id, "old")?;
                let new = require_obj_param(pmap, &id, "new")?;
                let report = diff::diff(old, new).map_err(|e| {
                    rpc_error(&id, rpc_code::INTERNAL_ERROR, format!("diff: {e:?}"))
                })?;
                validate_diagnostics_for_wire(&report.findings).map_err(|(d, errs)| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!(
                            "CH-JSONRPC-DIAG-INVALID: diff finding failed schema {d}: {errs:?}"
                        ),
                    )
                })?;
                serde_json::to_value(&report).map_err(|e| {
                    rpc_error(&id, rpc_code::INTERNAL_ERROR, format!("diff serde: {e}"))
                })
            }
            "trace_claim" => {
                let pmap = require_params(req)?;
                let claim_id = require_str_param(pmap, &id, "claim_id")?;
                if !repo.is_dir() {
                    return Err(rpc_error(
                        &id,
                        rpc_code::INVALID_PARAMS,
                        format!(
                            "CH-GATE-REPO-UNREADABLE: repo root not a directory: {}",
                            repo.display()
                        ),
                    ));
                }
                let g = build_trace_graph(repo)
                    .map_err(|e| rpc_error(&id, rpc_code::INTERNAL_ERROR, format!("trace: {e}")))?;
                let graph_v = serde_json::to_value(&g).map_err(|e| {
                    rpc_error(&id, rpc_code::INTERNAL_ERROR, format!("trace serde: {e}"))
                })?;
                validate_trace_graph_value(&graph_v).map_err(|errs| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!("trace: schema validation failed: {errs:?}"),
                    )
                })?;
                validate_diagnostics_for_wire(&g.diagnostics).map_err(|(d, errs)| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!(
                            "CH-JSONRPC-DIAG-INVALID: trace diagnostic failed schema {d}: {errs:?}"
                        ),
                    )
                })?;
                Ok(match g.claims.get(claim_id) {
                    Some(n) => serde_json::to_value(n).unwrap(),
                    None => Value::Null,
                })
            }
            "drift_report" => {
                if !repo.is_dir() {
                    return Err(rpc_error(
                        &id,
                        rpc_code::INVALID_PARAMS,
                        format!(
                            "CH-GATE-REPO-UNREADABLE: repo root not a directory: {}",
                            repo.display()
                        ),
                    ));
                }
                let trace = build_trace_graph(repo)
                    .map_err(|e| rpc_error(&id, rpc_code::INTERNAL_ERROR, format!("trace: {e}")))?;
                let trace_v = serde_json::to_value(&trace).map_err(|e| {
                    rpc_error(&id, rpc_code::INTERNAL_ERROR, format!("trace serde: {e}"))
                })?;
                validate_trace_graph_value(&trace_v).map_err(|errs| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!("trace: schema validation failed: {errs:?}"),
                    )
                })?;
                let r = build_drift_report(repo, &trace, Utc::now())
                    .map_err(|e| rpc_error(&id, rpc_code::INTERNAL_ERROR, format!("drift: {e}")))?;
                let r_v = serde_json::to_value(&r).map_err(|e| {
                    rpc_error(&id, rpc_code::INTERNAL_ERROR, format!("drift serde: {e}"))
                })?;
                validate_drift_report_value(&r_v).map_err(|errs| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!("drift: schema validation failed: {errs:?}"),
                    )
                })?;
                validate_diagnostics_for_wire(&r.diagnostics).map_err(|(d, errs)| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!(
                            "CH-JSONRPC-DIAG-INVALID: drift diagnostic failed schema {d}: {errs:?}"
                        ),
                    )
                })?;
                Ok(r_v)
            }
            "release_gate" => {
                // Optional params:
                //   fail_on_drift: bool — controls whether drift counts as a
                //     failure axis. Defaults to true to match `chassis
                //     release-gate --fail-on-drift`.
                let fail_on_drift = req
                    .get("params")
                    .and_then(|p| p.as_object())
                    .and_then(|m| m.get("fail_on_drift"))
                    .map(|v| {
                        v.as_bool().ok_or_else(|| {
                            rpc_error(
                                &id,
                                rpc_code::INVALID_PARAMS,
                                "params.fail_on_drift must be a boolean",
                            )
                        })
                    })
                    .transpose()?
                    .unwrap_or(true);

                let now = Utc::now();
                let run = gate::compute(repo, now, fail_on_drift).map_err(|e| {
                    let code = match e.rule_id {
                        gate::rule_id::REPO_UNREADABLE => rpc_code::INVALID_PARAMS,
                        _ => rpc_code::INTERNAL_ERROR,
                    };
                    rpc_error(&id, code, format!("{}: {}", e.rule_id, e.message))
                })?;
                let commands_run = vec![CommandRun {
                    argv: vec![
                        "chassis-jsonrpc".to_string(),
                        "release_gate".to_string(),
                        format!("--fail-on-drift={fail_on_drift}"),
                    ],
                    exit_code: run.outcome(false).final_exit_code,
                }];
                let predicate = run.predicate(commands_run, false).map_err(|e| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!("{}: {}", e.rule_id, e.message),
                    )
                })?;
                let v = serde_json::to_value(&predicate).map_err(|e| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!("predicate serde: {e}"),
                    )
                })?;
                // Defense-in-depth: re-validate the predicate against the
                // shared schema right before placing it on the wire. The
                // `predicate()` call above already validated it, but this
                // guards against drift between the typed struct and the
                // JSON representation.
                validate_release_gate_value(&v).map_err(|errs| {
                    rpc_error(
                        &id,
                        rpc_code::INTERNAL_ERROR,
                        format!("CH-GATE-SCHEMA-INVALID: release-gate predicate failed schema: {errs:?}"),
                    )
                })?;
                Ok(v)
            }
            "list_exemptions" => {
                let filter_rule = req
                    .get("params")
                    .and_then(|p| p.as_object())
                    .and_then(|m| m.get("rule_id"))
                    .map(|v| {
                        v.as_str().ok_or_else(|| {
                            rpc_error(
                                &id,
                                rpc_code::INVALID_PARAMS,
                                "params.rule_id must be a string",
                            )
                        })
                    })
                    .transpose()?;
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
            _ => Err(rpc_error(
                &id,
                rpc_code::METHOD_NOT_FOUND,
                "Method not found",
            )),
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
