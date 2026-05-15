#![forbid(unsafe_code)]

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use ed25519_dalek::VerifyingKey;
use serde_json::{json, Value};

use chassis_core::artifact::{
    validate_cedar_facts_value, validate_drift_report_value, validate_dsse_envelope_value,
    validate_eventcatalog_metadata_value, validate_opa_input_value, validate_policy_input_value,
    validate_release_gate_value, validate_trace_graph_value,
};
use chassis_core::attest::predicate::CommandRun;
use chassis_core::contract::validate_metadata_contract;
use chassis_core::diagnostic::Severity;
use chassis_core::diff;
use chassis_core::drift::report::build_drift_report;
use chassis_core::exempt;
use chassis_core::exempt::Codeowners;
use chassis_core::exports;
use chassis_core::fingerprint;
use chassis_core::trace::{build_trace_graph, render_mermaid};

/// Stable CLI exit codes. This surface is part of the public CLI contract —
/// do not renumber without bumping the CLI's CONTRACT.yaml version.
mod exit {
    pub const OK: i32 = 0;
    pub const VALIDATE_FAILED: i32 = 2;
    pub const EXEMPT_VIOLATION: i32 = 3;
    pub const DIFF_BREAKING: i32 = 4;
    pub const DRIFT_DETECTED: i32 = 5;
    pub const ATTEST_VERIFY_FAILED: i32 = 6;
    pub const MALFORMED_INPUT: i32 = 65;
    pub const MISSING_FILE: i32 = 66;
    pub const INTERNAL: i32 = 70;
}

/// CLI-layer rule IDs surfaced inside `--json` error envelopes.
mod rule {
    pub const MISSING_FILE: &str = "CLI-MISSING-FILE";
    pub const MALFORMED_INPUT: &str = "CLI-MALFORMED-INPUT";
    pub const INTERNAL: &str = "CLI-INTERNAL";
}

#[derive(Parser, Debug)]
#[command(
    name = "chassis",
    version,
    about = "Chassis governance CLI: validate, diff, trace, drift, export, exempt, attest.",
    long_about = LONG_ABOUT,
)]
struct Cli {
    /// Repository root (default: current directory).
    #[arg(long, global = true, default_value = ".")]
    repo: PathBuf,

    /// Emit machine-readable JSON on stdout. Errors are returned as JSON envelopes.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

const LONG_ABOUT: &str = "\
Chassis governance CLI.

Stable subcommands:
  validate <path>             Validate a CONTRACT.yaml against canonical JSON Schemas.
  diff <old> <new>            Diff two contract YAML documents and classify findings.
  trace [--mermaid]           Build the @claim ↔ CONTRACT trace graph for --repo.
  drift                       Compute the drift report (claim vs implementation churn).
  export --format <FORMAT>    Emit facts for external governance systems.
  exempt verify               Statically verify .chassis/exemptions.yaml against CODEOWNERS.
  release-gate                Run validate + trace + drift + exemptions + optional attestation.
  attest sign --out <FILE>    Assemble + sign a DSSE-wrapped release-gate Statement.
  attest verify <FILE>        Verify a DSSE envelope (signature + repo subject digest).

Exit codes:
  0   ok
  2   validate failed (schema-invalid contract)
  3   exempt verify surfaced an error diagnostic
  4   diff detected at least one breaking finding
  5   drift detected (stale, abandoned, or missing claims)
  6   attest verify failed (signature or subject mismatch)
  65  malformed input (parse error)
  66  missing required file
  70  internal error

When --json is set, every command's stdout is a single JSON document and
errors are emitted as `{\"ok\": false, \"error\": {\"code\": \"...\", \"message\": \"...\"}}`.";

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate a CONTRACT.yaml (or metadata contract YAML) against canonical JSON Schemas.
    Validate {
        /// Path to a contract YAML file.
        path: PathBuf,
    },
    /// Diff two contract YAML documents and classify findings (breaking / non-breaking / additive).
    Diff {
        /// Path to the prior contract YAML.
        old: PathBuf,
        /// Path to the new contract YAML.
        new: PathBuf,
    },
    /// Build the trace graph (@claim ↔ CONTRACT join) over --repo.
    Trace {
        /// Render Mermaid text instead of JSON/summary (overrides --json).
        #[arg(long)]
        mermaid: bool,
    },
    /// Compute the drift report using git history.
    Drift,
    /// Emit export-only facts for external governance systems.
    Export {
        /// Export format: chassis, opa, cedar, or eventcatalog.
        #[arg(long, value_enum, default_value_t = ExportFormat::Chassis)]
        format: ExportFormat,
    },
    /// Run the end-to-end release gate over --repo.
    ReleaseGate {
        /// Fail when unsuppressed drift diagnostics remain.
        #[arg(long)]
        fail_on_drift: bool,
        /// Write and verify a DSSE-wrapped in-toto attestation.
        #[arg(long)]
        attest: bool,
        /// Path for the release-gate predicate artifact.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Path for the DSSE envelope when --attest is set.
        #[arg(long)]
        attest_out: Option<PathBuf>,
        /// Path to an Ed25519 signing key (hex, 64 chars). Default: .chassis/keys/release.priv.
        #[arg(long)]
        private_key: Option<PathBuf>,
    },
    /// Exemption-registry verifier.
    #[command(subcommand)]
    Exempt(ExemptCmd),
    /// Release-gate attestation (DSSE envelope over an in-toto Statement).
    #[command(subcommand)]
    Attest(AttestCmd),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ExportFormat {
    /// Canonical Chassis policy-input JSON facts.
    Chassis,
    /// OPA/Rego input wrapper: { "input": <chassis policy input> }.
    Opa,
    /// Cedar-style entity/action/resource fact export.
    Cedar,
    /// EventCatalog-style service/message metadata from current contract fields.
    Eventcatalog,
}

#[derive(Subcommand, Debug)]
enum ExemptCmd {
    /// Statically verify `.chassis/exemptions.yaml` against CODEOWNERS.
    Verify,
}

#[derive(Subcommand, Debug)]
enum AttestCmd {
    /// Assemble, sign, and write a DSSE envelope to --out.
    Sign {
        /// Path to an Ed25519 signing key (hex, 64 chars). Default: .chassis/keys/release.priv.
        #[arg(long)]
        private_key: Option<PathBuf>,
        /// Destination path for the DSSE envelope JSON.
        #[arg(long)]
        out: PathBuf,
    },
    /// Verify a DSSE envelope (signature + repo-subject digest).
    Verify {
        /// Path to an Ed25519 verifying key (hex, 64 chars). Default: .chassis/keys/release.pub.
        #[arg(long)]
        public_key: Option<PathBuf>,
        /// Path to the DSSE envelope JSON file.
        file: PathBuf,
    },
}

/// CLI-layer error. Each variant maps to a deterministic exit code and a stable JSON envelope.
#[derive(Debug)]
struct CliError {
    code: i32,
    rule: &'static str,
    message: String,
    path: Option<PathBuf>,
}

impl CliError {
    fn missing_file(path: &Path, source: impl fmt::Display) -> Self {
        Self {
            code: exit::MISSING_FILE,
            rule: rule::MISSING_FILE,
            message: format!("missing or unreadable file: {source}"),
            path: Some(path.to_path_buf()),
        }
    }

    fn malformed(path: &Path, source: impl fmt::Display) -> Self {
        Self {
            code: exit::MALFORMED_INPUT,
            rule: rule::MALFORMED_INPUT,
            message: format!("malformed input: {source}"),
            path: Some(path.to_path_buf()),
        }
    }

    fn malformed_no_path(source: impl fmt::Display) -> Self {
        Self {
            code: exit::MALFORMED_INPUT,
            rule: rule::MALFORMED_INPUT,
            message: format!("malformed input: {source}"),
            path: None,
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: exit::INTERNAL,
            rule: rule::INTERNAL,
            message: message.into(),
            path: None,
        }
    }

    fn attest_verify(rule_id: &'static str, message: impl Into<String>) -> Self {
        Self {
            code: exit::ATTEST_VERIFY_FAILED,
            rule: rule_id,
            message: message.into(),
            path: None,
        }
    }

    fn render(&self, json: bool) {
        if json {
            let mut env = json!({
                "ok": false,
                "error": {
                    "code": self.rule,
                    "message": self.message,
                }
            });
            if let Some(p) = &self.path {
                env["error"]["path"] = json!(p.display().to_string());
            }
            println!("{env}");
        } else {
            eprintln!("{}: {}", self.rule, self.message);
            if let Some(p) = &self.path {
                eprintln!("  path: {}", p.display());
            }
        }
    }
}

fn read_text(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => CliError::missing_file(path, e),
        _ => CliError::missing_file(path, e),
    })
}

fn parse_yaml_to_json(path: &Path, raw: &str) -> Result<Value, CliError> {
    let y: serde_yaml::Value =
        serde_yaml::from_str(raw).map_err(|e| CliError::malformed(path, e))?;
    serde_json::to_value(y).map_err(|e| CliError::malformed(path, e))
}

fn canon_repo(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    let root = canon_repo(&cli.repo);

    let result = match cli.command {
        Command::Validate { path } => run_validate(&path, json),
        Command::Diff { old, new } => run_diff(&old, &new, json),
        Command::Trace { mermaid } => run_trace(&root, mermaid, json),
        Command::Drift => run_drift(&root, json),
        Command::Export { format } => run_export(&root, format, json),
        Command::ReleaseGate {
            fail_on_drift,
            attest,
            out,
            attest_out,
            private_key,
        } => run_release_gate(
            &root,
            fail_on_drift,
            attest,
            out.as_deref(),
            attest_out.as_deref(),
            private_key.as_deref(),
            json,
        ),
        Command::Exempt(ExemptCmd::Verify) => run_exempt_verify(&root, json),
        Command::Attest(AttestCmd::Sign { private_key, out }) => {
            run_attest_sign(&root, private_key.as_deref(), &out, json)
        }
        Command::Attest(AttestCmd::Verify { public_key, file }) => {
            run_attest_verify(&root, public_key.as_deref(), &file, json)
        }
    };

    let code = match result {
        Ok(c) => c,
        Err(e) => {
            e.render(json);
            e.code
        }
    };
    process::exit(code);
}

// -- validate -----------------------------------------------------------------

fn run_validate(path: &Path, json: bool) -> Result<i32, CliError> {
    let raw = read_text(path)?;
    let value = parse_yaml_to_json(path, &raw)?;
    match validate_metadata_contract(&value) {
        Ok(()) => {
            if json {
                println!(
                    "{}",
                    json!({"ok": true, "path": path.display().to_string()})
                );
            } else {
                println!("ok {}", path.display());
            }
            Ok(exit::OK)
        }
        Err(errs) => {
            if json {
                println!(
                    "{}",
                    json!({
                        "ok": false,
                        "path": path.display().to_string(),
                        "errors": errs,
                    })
                );
            } else {
                eprintln!(
                    "validate failed: {} ({} error(s))",
                    path.display(),
                    errs.len()
                );
                for e in &errs {
                    eprintln!("  - {e}");
                }
            }
            Ok(exit::VALIDATE_FAILED)
        }
    }
}

// -- diff ---------------------------------------------------------------------

fn run_diff(old: &Path, new: &Path, json: bool) -> Result<i32, CliError> {
    let old_raw = read_text(old)?;
    let new_raw = read_text(new)?;
    let old_v = parse_yaml_to_json(old, &old_raw)?;
    let new_v = parse_yaml_to_json(new, &new_raw)?;

    match diff::diff(&old_v, &new_v) {
        Ok(rep) => {
            if json {
                let payload = serde_json::to_value(&rep)
                    .map_err(|e| CliError::internal(format!("serialize diff report: {e}")))?;
                println!("{payload}");
            } else {
                let breaking = rep.count_by_classification(diff::Classification::Breaking);
                let nb = rep.count_by_classification(diff::Classification::NonBreaking);
                let add = rep.count_by_classification(diff::Classification::Additive);
                println!("breaking={breaking} non-breaking={nb} additive={add}");
                for f in &rep.findings {
                    println!("  {} {:?} {}", f.rule_id, f.severity, f.message);
                }
            }
            if rep.has_breaking() {
                Ok(exit::DIFF_BREAKING)
            } else {
                Ok(exit::OK)
            }
        }
        Err(diff::DiffError::Parse(msg)) => {
            Err(CliError::malformed_no_path(format!("diff parse: {msg}")))
        }
    }
}

// -- trace --------------------------------------------------------------------

fn run_trace(root: &Path, mermaid: bool, json: bool) -> Result<i32, CliError> {
    let graph = build_trace_graph(root).map_err(|e| CliError::internal(format!("trace: {e}")))?;
    let payload = serde_json::to_value(&graph)
        .map_err(|e| CliError::internal(format!("serialize trace graph: {e}")))?;
    validate_trace_graph_value(&payload)
        .map_err(|errs| CliError::internal(format!("trace schema invalid: {errs:?}")))?;

    if mermaid {
        // Mermaid is text-only; --mermaid wins over --json.
        println!("{}", render_mermaid(&graph));
    } else if json {
        println!("{payload}");
    } else {
        println!(
            "claims={} orphans={} diagnostics={}",
            graph.claims.len(),
            graph.orphan_sites.len(),
            graph.diagnostics.len()
        );
    }
    Ok(exit::OK)
}

// -- drift --------------------------------------------------------------------

fn run_drift(root: &Path, json: bool) -> Result<i32, CliError> {
    let trace = build_trace_graph(root)
        .map_err(|e| CliError::internal(format!("trace (for drift): {e}")))?;
    let report = build_drift_report(root, &trace, Utc::now())
        .map_err(|e| CliError::internal(format!("drift: {e}")))?;
    let payload = serde_json::to_value(&report)
        .map_err(|e| CliError::internal(format!("serialize drift report: {e}")))?;
    validate_drift_report_value(&payload)
        .map_err(|errs| CliError::internal(format!("drift schema invalid: {errs:?}")))?;

    if json {
        println!("{payload}");
    } else {
        println!(
            "stale={} abandoned={} missing={}",
            report.summary.stale, report.summary.abandoned, report.summary.missing
        );
    }

    let any_drift = report.summary.stale + report.summary.abandoned + report.summary.missing > 0;
    if any_drift {
        Ok(exit::DRIFT_DETECTED)
    } else {
        Ok(exit::OK)
    }
}

// -- export -------------------------------------------------------------------

fn run_export(root: &Path, format: ExportFormat, json: bool) -> Result<i32, CliError> {
    let input = build_export_policy_input(root)?;
    let payload = match format {
        ExportFormat::Chassis => {
            let v = serde_json::to_value(&input)
                .map_err(|e| CliError::internal(format!("serialize policy input export: {e}")))?;
            validate_policy_input_value(&v).map_err(|errs| {
                CliError::internal(format!("policy input schema invalid: {errs:?}"))
            })?;
            v
        }
        ExportFormat::Opa => {
            let v = serde_json::to_value(exports::opa_input(input))
                .map_err(|e| CliError::internal(format!("serialize OPA input export: {e}")))?;
            validate_opa_input_value(&v).map_err(|errs| {
                CliError::internal(format!("OPA input schema invalid: {errs:?}"))
            })?;
            v
        }
        ExportFormat::Cedar => {
            let v = serde_json::to_value(exports::cedar_facts(&input)).map_err(|e| {
                CliError::internal(format!("serialize Cedar-style facts export: {e}"))
            })?;
            validate_cedar_facts_value(&v).map_err(|errs| {
                CliError::internal(format!("Cedar-style facts schema invalid: {errs:?}"))
            })?;
            v
        }
        ExportFormat::Eventcatalog => {
            let v = serde_json::to_value(exports::eventcatalog_metadata(&input)).map_err(|e| {
                CliError::internal(format!("serialize EventCatalog metadata export: {e}"))
            })?;
            validate_eventcatalog_metadata_value(&v).map_err(|errs| {
                CliError::internal(format!("EventCatalog metadata schema invalid: {errs:?}"))
            })?;
            v
        }
    };

    if json {
        println!("{payload}");
    } else {
        let pretty = serde_json::to_string_pretty(&payload)
            .map_err(|e| CliError::internal(format!("pretty-print export JSON: {e}")))?;
        println!("{pretty}");
    }
    Ok(exit::OK)
}

// -- exempt verify ------------------------------------------------------------

fn run_exempt_verify(root: &Path, json: bool) -> Result<i32, CliError> {
    let reg_path = root.join(".chassis/exemptions.yaml");
    let raw = read_text(&reg_path)?;
    let json_val = parse_yaml_to_json(&reg_path, &raw)?;
    let registry: exempt::Registry = serde_json::from_value(json_val)
        .map_err(|e| CliError::malformed(&reg_path, format!("registry: {e}")))?;

    let co_path = root.join("CODEOWNERS");
    let co_raw = fs::read_to_string(&co_path).unwrap_or_default();
    let codeowners = Codeowners::parse(&co_raw)
        .map_err(|e| CliError::malformed(&co_path, format!("CODEOWNERS: {e}")))?;

    let diagnostics = exempt::verify(&registry, Utc::now(), &codeowners);
    let has_error = diagnostics.iter().any(|d| d.severity == Severity::Error);

    if json {
        println!(
            "{}",
            json!({
                "ok": !has_error,
                "diagnostics": diagnostics,
            })
        );
    } else {
        for d in &diagnostics {
            println!("{} {:?} {}", d.rule_id, d.severity, d.message);
        }
        if !has_error {
            println!("exempt verify: clean ({} diagnostic(s))", diagnostics.len());
        }
    }

    if has_error {
        Ok(exit::EXEMPT_VIOLATION)
    } else {
        Ok(exit::OK)
    }
}

// -- release gate --------------------------------------------------------------

fn run_release_gate(
    root: &Path,
    fail_on_drift: bool,
    attest: bool,
    out: Option<&Path>,
    attest_out: Option<&Path>,
    private_key: Option<&Path>,
    json: bool,
) -> Result<i32, CliError> {
    let now = Utc::now();
    let artifact_path = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("release-gate.json"));
    let attestation_path = attest_out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("release-gate.dsse"));

    let contract_summary = validate_repo_contracts(root)?;
    if contract_summary.invalid > 0 {
        let output = json!({
            "ok": false,
            "verdict": "fail",
            "schema_fingerprint": Value::Null,
            "git_commit": Value::Null,
            "contract_validation": contract_summary.to_json(),
            "trace_summary": Value::Null,
            "drift_summary": Value::Null,
            "exemption_summary": Value::Null,
            "artifact_path": Value::Null,
            "attestation_path": Value::Null,
        });
        print_release_gate_output(&output, json);
        return Ok(exit::VALIDATE_FAILED);
    }

    let trace = build_trace_graph(root)
        .map_err(|e| CliError::internal(format!("trace (for release gate): {e}")))?;
    let trace_v = serde_json::to_value(&trace)
        .map_err(|e| CliError::internal(format!("serialize trace graph: {e}")))?;
    validate_trace_graph_value(&trace_v)
        .map_err(|errs| CliError::internal(format!("trace schema invalid: {errs:?}")))?;

    let drift = build_drift_report(root, &trace, now)
        .map_err(|e| CliError::internal(format!("drift (for release gate): {e}")))?;
    let drift_v = serde_json::to_value(&drift)
        .map_err(|e| CliError::internal(format!("serialize drift report: {e}")))?;
    validate_drift_report_value(&drift_v)
        .map_err(|errs| CliError::internal(format!("drift schema invalid: {errs:?}")))?;

    let exemption_gate = load_and_apply_exemptions(root, drift.diagnostics.clone(), now)?;
    let schema_fingerprint =
        fingerprint::compute(root).map_err(|e| CliError::internal(format!("fingerprint: {e}")))?;
    let git_commit = git_head(root)?;

    let commands_run = vec![CommandRun {
        argv: vec![
            "chassis".to_string(),
            "release-gate".to_string(),
            "--repo".to_string(),
            root.display().to_string(),
            if fail_on_drift {
                "--fail-on-drift".to_string()
            } else {
                "--no-fail-on-drift".to_string()
            },
            if attest {
                "--attest".to_string()
            } else {
                "--no-attest".to_string()
            },
        ],
        exit_code: 0,
    }];

    let statement = chassis_core::attest::assemble(
        root,
        &trace,
        &drift,
        exemption_gate.registry.as_ref(),
        commands_run,
        now,
    )
    .map_err(|e| CliError::internal(format!("assemble release gate: {e}")))?;
    let predicate_v = serde_json::to_value(&statement.predicate)
        .map_err(|e| CliError::internal(format!("serialize release-gate predicate: {e}")))?;
    validate_release_gate_value(&predicate_v)
        .map_err(|errs| CliError::internal(format!("release-gate schema invalid: {errs:?}")))?;
    write_json_file(&artifact_path, &predicate_v)?;

    let trace_summary = trace_summary_json(&trace);
    let unsuppressed_drift_blocking = exemption_gate
        .unsuppressed_drift
        .iter()
        .any(|d| matches!(d.severity, Severity::Error | Severity::Warning));
    let drift_failed = fail_on_drift && unsuppressed_drift_blocking;
    let trace_failed = trace.orphan_sites.len()
        + trace
            .claims
            .values()
            .filter(|n| n.impl_sites.is_empty())
            .count()
        + trace
            .claims
            .values()
            .filter(|n| n.test_sites.is_empty())
            .count()
        > 0;
    let exemption_failed = exemption_gate.error_count > 0;

    let mut attestation_written: Option<PathBuf> = None;
    let mut attestation_error: Option<String> = None;
    if attest {
        match sign_and_verify_statement(root, &statement, private_key, &attestation_path) {
            Ok(()) => attestation_written = Some(attestation_path.clone()),
            Err(e) => attestation_error = Some(e.message),
        }
    }

    let passed = !trace_failed && !drift_failed && !exemption_failed && attestation_error.is_none();
    let output = json!({
        "ok": passed,
        "verdict": if passed { "pass" } else { "fail" },
        "schema_fingerprint": schema_fingerprint,
        "git_commit": git_commit,
        "contract_validation": contract_summary.to_json(),
        "trace_summary": trace_summary,
        "drift_summary": {
            "stale": drift.summary.stale,
            "abandoned": drift.summary.abandoned,
            "missing": drift.summary.missing,
            "unsuppressed_blocking": exemption_gate.unsuppressed_blocking_count(),
        },
        "exemption_summary": exemption_gate.to_summary_json(),
        "artifact_path": artifact_path.display().to_string(),
        "attestation_path": attestation_written
            .as_ref()
            .map(|p| p.display().to_string()),
        "attestation_error": attestation_error,
    });
    print_release_gate_output(&output, json);

    if passed {
        Ok(exit::OK)
    } else if attestation_error.is_some() {
        Ok(exit::ATTEST_VERIFY_FAILED)
    } else if exemption_failed {
        Ok(exit::EXEMPT_VIOLATION)
    } else {
        Ok(exit::DRIFT_DETECTED)
    }
}

#[derive(Debug)]
struct ContractValidationSummary {
    checked: usize,
    valid: usize,
    invalid: usize,
    errors: Vec<Value>,
}

impl ContractValidationSummary {
    fn to_json(&self) -> Value {
        json!({
            "checked": self.checked,
            "valid": self.valid,
            "invalid": self.invalid,
            "errors": self.errors,
        })
    }
}

fn validate_repo_contracts(root: &Path) -> Result<ContractValidationSummary, CliError> {
    let contracts = discover_contract_files(root)?;
    let mut summary = ContractValidationSummary {
        checked: contracts.len(),
        valid: 0,
        invalid: 0,
        errors: Vec::new(),
    };

    for path in contracts {
        let rel = path
            .strip_prefix(root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.clone());
        match read_text(&path)
            .and_then(|raw| parse_yaml_to_json(&path, &raw))
            .and_then(|value| {
                validate_metadata_contract(&value).map_err(|errs| CliError {
                    code: exit::VALIDATE_FAILED,
                    rule: "CH-RUST-METADATA-CONTRACT",
                    message: errs.join("; "),
                    path: Some(path.clone()),
                })
            }) {
            Ok(()) => summary.valid += 1,
            Err(e) => {
                summary.invalid += 1;
                summary.errors.push(json!({
                    "path": rel.display().to_string(),
                    "code": e.rule,
                    "message": e.message,
                }));
            }
        }
    }

    Ok(summary)
}

fn build_export_policy_input(root: &Path) -> Result<exports::PolicyInput, CliError> {
    let contracts = load_contract_facts(root)?;
    let trace = build_trace_graph(root)
        .map_err(|e| CliError::internal(format!("trace (for export): {e}")))?;
    let trace_v = serde_json::to_value(&trace)
        .map_err(|e| CliError::internal(format!("serialize trace graph: {e}")))?;
    validate_trace_graph_value(&trace_v)
        .map_err(|errs| CliError::internal(format!("trace schema invalid: {errs:?}")))?;

    let drift = build_drift_report(root, &trace, Utc::now())
        .map_err(|e| CliError::internal(format!("drift (for export): {e}")))?;
    let drift_v = serde_json::to_value(&drift)
        .map_err(|e| CliError::internal(format!("serialize drift report: {e}")))?;
    validate_drift_report_value(&drift_v)
        .map_err(|errs| CliError::internal(format!("drift schema invalid: {errs:?}")))?;

    let exemptions = load_export_exemption_facts(root)?;
    let schema_fingerprint =
        fingerprint::compute(root).map_err(|e| CliError::internal(format!("fingerprint: {e}")))?;
    let repo = exports::RepoFacts {
        root: root.display().to_string(),
        git_commit: Some(git_head(root)?),
        schema_fingerprint: Some(schema_fingerprint),
    };

    Ok(exports::build_policy_input(
        repo,
        contracts,
        &trace,
        drift.summary.clone(),
        drift.diagnostics,
        exemptions,
    ))
}

fn load_contract_facts(root: &Path) -> Result<Vec<exports::ContractFact>, CliError> {
    discover_contract_files(root)?
        .into_iter()
        .map(|path| {
            let rel = path
                .strip_prefix(root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.clone());
            let raw = read_text(&path)?;
            let value = parse_yaml_to_json(&path, &raw)?;
            validate_metadata_contract(&value).map_err(|errs| CliError {
                code: exit::VALIDATE_FAILED,
                rule: "CH-RUST-METADATA-CONTRACT",
                message: errs.join("; "),
                path: Some(path.clone()),
            })?;
            exports::contract_fact(rel, value)
                .map_err(|e| CliError::internal(format!("contract export fact: {e}")))
        })
        .collect()
}

fn load_export_exemption_facts(root: &Path) -> Result<exports::ExemptionFacts, CliError> {
    let registry = load_exemptions_strict(root)?;
    let diagnostics = match &registry {
        Some(registry) => {
            let co_path = root.join("CODEOWNERS");
            let co_raw = fs::read_to_string(&co_path).unwrap_or_default();
            let codeowners = Codeowners::parse(&co_raw)
                .map_err(|e| CliError::malformed(&co_path, format!("CODEOWNERS: {e}")))?;
            exempt::verify(registry, Utc::now(), &codeowners)
        }
        None => Vec::new(),
    };

    Ok(exports::ExemptionFacts {
        registry,
        diagnostics,
    })
}

fn discover_contract_files(root: &Path) -> Result<Vec<PathBuf>, CliError> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), CliError> {
        let entries = fs::read_dir(dir).map_err(|e| CliError::missing_file(dir, e))?;
        for ent in entries {
            let ent = ent.map_err(|e| CliError::missing_file(dir, e))?;
            let p = ent.path();
            if p.is_dir() {
                if matches!(
                    p.file_name().and_then(|n| n.to_str()),
                    Some("target")
                        | Some("node_modules")
                        | Some(".git")
                        | Some("fixtures")
                        | Some("reference")
                ) {
                    continue;
                }
                walk(&p, out)?;
            } else if p.file_name().and_then(|n| n.to_str()) == Some("CONTRACT.yaml") {
                out.push(p);
            }
        }
        Ok(())
    }

    let mut out = Vec::new();
    walk(root, &mut out)?;
    out.sort();
    Ok(out)
}

struct ExemptionGate {
    registry: Option<exempt::Registry>,
    diagnostics: Vec<chassis_core::diagnostic::Diagnostic>,
    error_count: usize,
    active_count: usize,
    unsuppressed_drift: Vec<chassis_core::diagnostic::Diagnostic>,
    suppressed_count: usize,
    overridden_count: usize,
    audit_count: usize,
}

impl ExemptionGate {
    fn unsuppressed_blocking_count(&self) -> usize {
        self.unsuppressed_drift
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
            .count()
    }

    fn to_summary_json(&self) -> Value {
        json!({
            "registry_present": self.registry.is_some(),
            "active": self.active_count,
            "diagnostics": self.diagnostics.len(),
            "errors": self.error_count,
            "suppressed": self.suppressed_count,
            "overridden": self.overridden_count,
            "audit": self.audit_count,
        })
    }
}

fn load_and_apply_exemptions(
    root: &Path,
    drift_diagnostics: Vec<chassis_core::diagnostic::Diagnostic>,
    now: chrono::DateTime<Utc>,
) -> Result<ExemptionGate, CliError> {
    let registry = load_exemptions_strict(root)?;
    let Some(registry) = registry else {
        return Ok(ExemptionGate {
            registry: None,
            diagnostics: Vec::new(),
            error_count: 0,
            active_count: 0,
            unsuppressed_drift: drift_diagnostics,
            suppressed_count: 0,
            overridden_count: 0,
            audit_count: 0,
        });
    };

    let co_path = root.join("CODEOWNERS");
    let co_raw = fs::read_to_string(&co_path).unwrap_or_default();
    let codeowners = Codeowners::parse(&co_raw)
        .map_err(|e| CliError::malformed(&co_path, format!("CODEOWNERS: {e}")))?;
    let diagnostics = exempt::verify(&registry, now, &codeowners);
    let error_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let active_count = exempt::list(
        &registry,
        exempt::ListFilter {
            rule_id: None,
            path: None,
            active_at: Some(now.date_naive()),
        },
    )
    .len();
    let applied = exempt::apply::apply_exemptions(drift_diagnostics, &registry, now);

    Ok(ExemptionGate {
        registry: Some(registry),
        diagnostics,
        error_count,
        active_count,
        unsuppressed_drift: applied.unsuppressed,
        suppressed_count: applied.suppressed.len(),
        overridden_count: applied.overridden.len(),
        audit_count: applied.audit.len(),
    })
}

fn load_exemptions_strict(root: &Path) -> Result<Option<exempt::Registry>, CliError> {
    let p = root.join(".chassis/exemptions.yaml");
    if !p.exists() {
        return Ok(None);
    }
    let raw = read_text(&p)?;
    let y: serde_yaml::Value =
        serde_yaml::from_str(&raw).map_err(|e| CliError::malformed(&p, e))?;
    let j = serde_json::to_value(y).map_err(|e| CliError::malformed(&p, e))?;
    serde_json::from_value(j)
        .map(Some)
        .map_err(|e| CliError::malformed(&p, format!("registry: {e}")))
}

fn trace_summary_json(trace: &chassis_core::trace::TraceGraph) -> Value {
    let missing_impl = trace
        .claims
        .values()
        .filter(|node| node.impl_sites.is_empty())
        .count();
    let missing_tests = trace
        .claims
        .values()
        .filter(|node| node.test_sites.is_empty())
        .count();
    json!({
        "claims": trace.claims.len(),
        "orphan_sites": trace.orphan_sites.len(),
        "missing_impl": missing_impl,
        "missing_tests": missing_tests,
        "diagnostics": trace.diagnostics.len(),
    })
}

fn sign_and_verify_statement(
    root: &Path,
    statement: &chassis_core::attest::Statement,
    private_key: Option<&Path>,
    out: &Path,
) -> Result<(), CliError> {
    use chassis_core::attest::sign::{
        generate_keypair, sign_statement, signing_key_from_hex, verify_envelope,
        verify_subject_matches_repo, verifying_key_for,
    };
    use chassis_core::attest::{CH_ATTEST_SUBJECT_MISMATCH, CH_ATTEST_VERIFY_FAILED};

    let default_pk = root.join(".chassis/keys/release.priv");
    let pk_path: &Path = private_key.unwrap_or(&default_pk);
    let sk = if private_key.is_none() && !default_pk.exists() {
        generate_keypair()
    } else {
        let sk_hex = read_text(pk_path)?;
        signing_key_from_hex(sk_hex.trim())
            .map_err(|e| CliError::malformed(pk_path, format!("private key: {e}")))?
    };
    let envelope =
        sign_statement(statement, &sk).map_err(|e| CliError::internal(format!("sign: {e}")))?;
    let env_v = serde_json::to_value(&envelope)
        .map_err(|e| CliError::internal(format!("serialize envelope: {e}")))?;
    validate_dsse_envelope_value(&env_v).map_err(|errs| {
        CliError::attest_verify(
            CH_ATTEST_VERIFY_FAILED,
            format!("DSSE envelope schema invalid: {errs:?}"),
        )
    })?;
    write_json_file(out, &env_v)?;

    let vk = verifying_key_for(&sk);
    let verified = verify_envelope(&envelope, &vk)
        .map_err(|e| CliError::attest_verify(CH_ATTEST_VERIFY_FAILED, format!("{e}")))?;
    verify_subject_matches_repo(&verified, root)
        .map_err(|e| CliError::attest_verify(CH_ATTEST_SUBJECT_MISMATCH, format!("{e}")))?;
    Ok(())
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| CliError::internal(format!("mkdir {}: {e}", parent.display())))?;
    }
    let txt = serde_json::to_string_pretty(value)
        .map_err(|e| CliError::internal(format!("pretty-print JSON: {e}")))?;
    fs::write(path, format!("{txt}\n"))
        .map_err(|e| CliError::internal(format!("write {}: {e}", path.display())))
}

fn git_head(root: &Path) -> Result<String, CliError> {
    let out = process::Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| CliError::internal(format!("git rev-parse HEAD: {e}")))?;
    if !out.status.success() {
        return Err(CliError::internal(format!(
            "git rev-parse HEAD failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn print_release_gate_output(output: &Value, json: bool) {
    if json {
        println!("{output}");
        return;
    }
    println!(
        "release-gate: {}",
        output["verdict"].as_str().unwrap_or("fail")
    );
    println!(
        "schema_fingerprint={}",
        output["schema_fingerprint"].as_str().unwrap_or("")
    );
    println!("git_commit={}", output["git_commit"].as_str().unwrap_or(""));
    println!("contract_validation={}", output["contract_validation"]);
    println!("trace_summary={}", output["trace_summary"]);
    println!("drift_summary={}", output["drift_summary"]);
    println!("exemption_summary={}", output["exemption_summary"]);
    if let Some(path) = output["artifact_path"].as_str() {
        println!("artifact_path={path}");
    }
    if let Some(path) = output["attestation_path"].as_str() {
        println!("attestation_path={path}");
    }
}

// -- attest sign --------------------------------------------------------------

fn run_attest_sign(
    root: &Path,
    private_key: Option<&Path>,
    out: &Path,
    json: bool,
) -> Result<i32, CliError> {
    use chassis_core::attest::sign::signing_key_from_hex;
    use chassis_core::attest::{assemble, sign_statement};

    let default_pk = root.join(".chassis/keys/release.priv");
    let pk_path: &Path = private_key.unwrap_or(&default_pk);
    let sk_hex = read_text(pk_path)?;
    let sk = signing_key_from_hex(sk_hex.trim())
        .map_err(|e| CliError::malformed(pk_path, format!("private key: {e}")))?;

    let trace = build_trace_graph(root)
        .map_err(|e| CliError::internal(format!("trace (for attest): {e}")))?;
    let drift = build_drift_report(root, &trace, Utc::now())
        .map_err(|e| CliError::internal(format!("drift (for attest): {e}")))?;
    let ex = load_exemptions(root);

    let stmt = assemble(root, &trace, &drift, ex.as_ref(), vec![], Utc::now())
        .map_err(|e| CliError::internal(format!("assemble: {e}")))?;
    let envelope =
        sign_statement(&stmt, &sk).map_err(|e| CliError::internal(format!("sign: {e}")))?;
    let env_v = serde_json::to_value(&envelope)
        .map_err(|e| CliError::internal(format!("serialize envelope: {e}")))?;
    validate_dsse_envelope_value(&env_v)
        .map_err(|errs| CliError::internal(format!("DSSE envelope schema invalid: {errs:?}")))?;

    let txt = serde_json::to_string_pretty(&env_v)
        .map_err(|e| CliError::internal(format!("pretty-print envelope: {e}")))?;
    fs::write(out, txt).map_err(|e| CliError::internal(format!("write {}: {e}", out.display())))?;

    let sha256 = stmt
        .subject
        .first()
        .map(|s| s.digest.sha256.clone())
        .unwrap_or_default();

    if json {
        println!(
            "{}",
            json!({
                "ok": true,
                "out": out.display().to_string(),
                "sha256": sha256,
                "predicateType": stmt.predicate_type,
            })
        );
    } else {
        println!("wrote {}", out.display());
    }
    Ok(exit::OK)
}

// -- attest verify ------------------------------------------------------------

fn run_attest_verify(
    root: &Path,
    public_key: Option<&Path>,
    file: &Path,
    json: bool,
) -> Result<i32, CliError> {
    use chassis_core::attest::sign::{
        verify_envelope, verify_subject_matches_repo, verifying_key_from_hex,
    };
    use chassis_core::attest::{CH_ATTEST_SUBJECT_MISMATCH, CH_ATTEST_VERIFY_FAILED};

    let default_pk = root.join(".chassis/keys/release.pub");
    let pk_path: &Path = public_key.unwrap_or(&default_pk);
    let pk_hex = read_text(pk_path)?;
    let vk: VerifyingKey = verifying_key_from_hex(pk_hex.trim())
        .map_err(|e| CliError::malformed(pk_path, format!("public key: {e}")))?;

    let raw = read_text(file)?;
    let env_v: Value = serde_json::from_str(&raw).map_err(|e| CliError::malformed(file, e))?;
    validate_dsse_envelope_value(&env_v).map_err(|errs| {
        CliError::attest_verify(
            CH_ATTEST_VERIFY_FAILED,
            format!("DSSE envelope schema invalid: {errs:?}"),
        )
    })?;
    let envelope: chassis_core::attest::DsseEnvelope = serde_json::from_value(env_v)
        .map_err(|e| CliError::malformed(file, format!("envelope: {e}")))?;

    let stmt = verify_envelope(&envelope, &vk)
        .map_err(|e| CliError::attest_verify(CH_ATTEST_VERIFY_FAILED, format!("{e}")))?;

    verify_subject_matches_repo(&stmt, root)
        .map_err(|e| CliError::attest_verify(CH_ATTEST_SUBJECT_MISMATCH, format!("{e}")))?;

    if json {
        let payload = serde_json::to_value(&stmt)
            .map_err(|e| CliError::internal(format!("serialize statement: {e}")))?;
        println!("{payload}");
    } else {
        println!("ok {}", file.display());
    }
    Ok(exit::OK)
}

fn load_exemptions(root: &Path) -> Option<exempt::Registry> {
    let p = root.join(".chassis/exemptions.yaml");
    let raw = fs::read_to_string(p).ok()?;
    let y: serde_yaml::Value = serde_yaml::from_str(&raw).ok()?;
    let j = serde_json::to_value(y).ok()?;
    serde_json::from_value(j).ok()
}
