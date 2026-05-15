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
    validate_release_gate_value, validate_spec_index_value, validate_trace_graph_value,
};
use chassis_core::attest::predicate::{CommandRun, Verdict};
use chassis_core::attest::GateOutcome;
use chassis_core::contract::validate_metadata_contract;
use chassis_core::diagnostic::Severity;
use chassis_core::diff;
use chassis_core::drift::report::build_drift_report;
use chassis_core::exempt;
use chassis_core::exempt::Codeowners;
use chassis_core::exports;
use chassis_core::fingerprint;
use chassis_core::gate::compute as gate_compute;
use chassis_core::spec_index::{
    digest_sha256_hex, export_from_source_yaml_path, link_spec_index, SpecIndex,
};
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
    /// No release signing key is present and the caller did not opt in to
    /// ephemeral signing. Release-grade attestations are never signed with
    /// implicit throwaway keys.
    pub const ATTEST_KEY_MISSING: &str = "CH-ATTEST-KEY-MISSING";
}

#[derive(Parser, Debug)]
#[command(
    name = "chassis",
    version,
    about = "Chassis governance CLI: validate, diff, trace, drift, export, spec-index, exempt, attest.",
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
  spec-index export           Emit deterministic spec-index.json from YAML source.
  spec-index validate         Schema-validate spec-index.json.
  spec-index link             Link spec requirements to CONTRACT.yaml claims.
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
errors are emitted as `{\"ok\": false, \"error\": {\"code\": \"...\", \"message\": \"...\"}}`.

Attestation key policy:
  - `attest sign` and `release-gate --attest` fail closed when no signing key
    is present. Provide --private-key <path>, or place a hex Ed25519 key at
    .chassis/keys/release.priv, OR opt explicitly into a throwaway keypair
    with --ephemeral-key. Without one of these the command exits with
    CH-ATTEST-KEY-MISSING. Ephemeral signing is for demos/tests only — the
    CLI marks such artifacts as release_grade=false and writes the matching
    public key next to the envelope.";

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
    /// Spec Kit index: export YAML → JSON, validate, or link to contracts.
    #[command(subcommand)]
    SpecIndex(SpecIndexCmd),
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
        /// Path to an Ed25519 signing key (hex, 64 chars). When omitted, the
        /// release gate requires `.chassis/keys/release.priv`; if absent the
        /// command fails with CH-ATTEST-KEY-MISSING. Release-grade.
        #[arg(long)]
        private_key: Option<PathBuf>,
        /// Sign with a freshly generated throwaway keypair for demos/tests.
        /// The resulting attestation is marked NON-release-grade and the
        /// public key is written next to the envelope. Mutually exclusive
        /// with --private-key.
        #[arg(long, conflicts_with = "private_key")]
        ephemeral_key: bool,
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
        /// Path to an Ed25519 signing key (hex, 64 chars). When omitted, the
        /// signer requires `.chassis/keys/release.priv`; if absent the command
        /// fails with CH-ATTEST-KEY-MISSING. Release-grade.
        #[arg(long)]
        private_key: Option<PathBuf>,
        /// Sign with a freshly generated throwaway keypair for demos/tests.
        /// Marks the resulting attestation NON-release-grade and writes the
        /// public key next to the envelope. Mutually exclusive with
        /// --private-key.
        #[arg(long, conflicts_with = "private_key")]
        ephemeral_key: bool,
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

#[derive(Subcommand, Debug)]
enum SpecIndexCmd {
    /// Write canonical spec-index.json from YAML (Chassis Spec Kit preset).
    Export {
        /// Path to the YAML source (for example `.chassis/spec-index-source.yaml`).
        #[arg(long)]
        from: PathBuf,
        /// Output JSON path (for example `artifacts/spec-index.json`).
        #[arg(long)]
        out: PathBuf,
    },
    /// Validate spec-index.json against the canonical JSON Schema.
    Validate {
        /// Path to a spec-index.json file.
        path: PathBuf,
    },
    /// Map requirements to CONTRACT.yaml claims; print diagnostics.
    Link {
        /// Path to spec-index.json.
        #[arg(long)]
        index: PathBuf,
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

    fn attest_key_missing(path: &Path) -> Self {
        Self {
            code: exit::MISSING_FILE,
            rule: rule::ATTEST_KEY_MISSING,
            message: format!(
                "no release signing key at {}; pass --private-key <path> or --ephemeral-key (demos/tests only)",
                path.display()
            ),
            path: Some(path.to_path_buf()),
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
        Command::SpecIndex(cmd) => match cmd {
            SpecIndexCmd::Export { from, out } => run_spec_index_export(&from, &out, json),
            SpecIndexCmd::Validate { path } => run_spec_index_validate(&path, json),
            SpecIndexCmd::Link { index } => run_spec_index_link(&root, &index, json),
        },
        Command::ReleaseGate {
            fail_on_drift,
            attest,
            out,
            attest_out,
            private_key,
            ephemeral_key,
        } => run_release_gate(
            &root,
            fail_on_drift,
            attest,
            out.as_deref(),
            attest_out.as_deref(),
            private_key.as_deref(),
            ephemeral_key,
            json,
        ),
        Command::Exempt(ExemptCmd::Verify) => run_exempt_verify(&root, json),
        Command::Attest(AttestCmd::Sign {
            private_key,
            ephemeral_key,
            out,
        }) => run_attest_sign(&root, private_key.as_deref(), ephemeral_key, &out, json),
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

// -- spec-index ---------------------------------------------------------------

fn run_spec_index_export(from: &Path, out: &Path, json: bool) -> Result<i32, CliError> {
    let idx = export_from_source_yaml_path(from).map_err(|e| CliError {
        code: exit::MALFORMED_INPUT,
        rule: e.rule_id,
        message: e.message,
        path: Some(from.to_path_buf()),
    })?;
    let v = serde_json::to_value(&idx)
        .map_err(|e| CliError::internal(format!("serialize spec index: {e}")))?;
    validate_spec_index_value(&v).map_err(|errs| {
        CliError::internal(format!("spec index schema invalid after export: {errs:?}"))
    })?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| CliError::internal(format!("create_dir {}: {e}", parent.display())))?;
    }
    let pretty = serde_json::to_string_pretty(&v)
        .map_err(|e| CliError::internal(format!("pretty spec index json: {e}")))?;
    fs::write(out, pretty + "\n").map_err(|e| CliError::missing_file(out, e))?;

    let digest = digest_sha256_hex(&idx).map_err(CliError::internal)?;
    if json {
        println!(
            "{}",
            json!({
                "ok": true,
                "out": out.display().to_string(),
                "digest": digest,
            })
        );
    } else {
        println!("wrote {} (spec_index digest {digest})", out.display());
    }
    Ok(exit::OK)
}

fn run_spec_index_validate(path: &Path, json: bool) -> Result<i32, CliError> {
    let raw = read_text(path)?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| CliError::malformed(path, e))?;
    match validate_spec_index_value(&v) {
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

fn run_spec_index_link(root: &Path, index_path: &Path, json: bool) -> Result<i32, CliError> {
    let raw = read_text(index_path)?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| CliError::malformed(index_path, e))?;
    validate_spec_index_value(&v)
        .map_err(|errs| CliError::internal(format!("spec index schema: {errs:?}")))?;
    let spec: SpecIndex = serde_json::from_value(v)
        .map_err(|e| CliError::malformed(index_path, format!("spec index serde: {e}")))?;
    let trace = build_trace_graph(root)
        .map_err(|e| CliError::internal(format!("trace (for spec-index link): {e}")))?;
    let diags = link_spec_index(&spec, root, &trace);
    let has_err = diags.iter().any(|d| d.severity == Severity::Error);

    if json {
        println!(
            "{}",
            json!({
                "ok": !has_err,
                "diagnostics": diags,
            })
        );
    } else {
        for d in &diags {
            println!("{} {:?} {}", d.rule_id, d.severity, d.message);
        }
        if !has_err {
            println!("spec-index link: clean ({} diagnostic(s))", diags.len());
        }
    }

    if has_err {
        Ok(exit::VALIDATE_FAILED)
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

#[allow(clippy::too_many_arguments)]
fn run_release_gate(
    root: &Path,
    fail_on_drift: bool,
    attest: bool,
    out: Option<&Path>,
    attest_out: Option<&Path>,
    private_key: Option<&Path>,
    ephemeral_key: bool,
    json: bool,
) -> Result<i32, CliError> {
    let now = Utc::now();
    let artifact_path = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("release-gate.json"));
    let attestation_path = attest_out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("release-gate.dsse"));

    let mut argv = vec![
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
    ];
    if ephemeral_key {
        argv.push("--ephemeral-key".to_string());
    }

    let contract_summary = validate_repo_contracts(root)?;
    if contract_summary.invalid > 0 {
        let final_exit_code = exit::VALIDATE_FAILED;
        let output = json!({
            "ok": false,
            "verdict": "fail",
            "fail_on_drift": fail_on_drift,
            "trace_failed": false,
            "drift_failed": false,
            "exemption_failed": false,
            "attestation_failed": false,
            "spec_index_present": false,
            "spec_index_digest": Value::Null,
            "spec_link_failed": false,
            "spec_link_diagnostics": Value::Array(vec![]),
            "unsuppressed_blocking": 0,
            "suppressed": 0,
            "severity_overridden": 0,
            "final_exit_code": final_exit_code,
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
        return Ok(final_exit_code);
    }

    let run = gate_compute(root, now, fail_on_drift)
        .map_err(|e| CliError::internal(format!("release gate: {}: {}", e.rule_id, e.message)))?;
    let trace = &run.trace;
    let drift = &run.drift;

    let spec_index_present = run.spec.present;
    let spec_index_digest = run.spec.digest.clone();
    let spec_link_failed = run.spec.failed();
    let spec_link_diagnostics = run.spec.diagnostics.clone();

    let schema_fingerprint = run.schema_fingerprint.clone();
    let git_commit = run.git_commit.clone();

    // Fail closed on attestation: when --attest is requested, resolve the
    // signing key BEFORE any predicate is assembled. If there is no explicit
    // --private-key, no .chassis/keys/release.priv, and no --ephemeral-key
    // opt-in, abort so the gate cannot produce an implicitly throwaway-signed
    // attestation that looks release-grade on inspection.
    let sign_ctx = if attest {
        Some(resolve_signing_key(
            root,
            private_key,
            ephemeral_key,
            &attestation_path,
        )?)
    } else {
        None
    };

    // Pre-attestation outcome from shared gate semantics (must match JSON-RPC).
    let pre_outcome = run.outcome(false);
    let pre_commands = vec![CommandRun {
        argv: argv.clone(),
        exit_code: pre_outcome.final_exit_code,
    }];
    let pre_statement = chassis_core::attest::assemble(
        root,
        trace,
        drift,
        run.exempt_registry.as_ref(),
        pre_commands,
        pre_outcome.clone(),
        now,
    )
    .map_err(|e| CliError::internal(format!("assemble release gate: {e}")))?;

    let mut attestation_written: Option<PathBuf> = None;
    let mut attestation_error: Option<String> = None;
    let mut release_grade: Option<bool> = None;
    let mut public_key_path_out: Option<String> = None;
    let mut public_key_fingerprint_out: Option<String> = None;
    if let Some(ctx) = sign_ctx.as_ref() {
        match sign_envelope_with_context(root, &pre_statement, ctx, &attestation_path) {
            Ok(()) => {
                attestation_written = Some(attestation_path.clone());
                release_grade = Some(ctx.release_grade);
                public_key_path_out = ctx
                    .public_key_path
                    .as_ref()
                    .map(|p| p.display().to_string());
                public_key_fingerprint_out = Some(ctx.public_key_fingerprint.clone());
                if !ctx.release_grade {
                    let pk_desc = ctx
                        .public_key_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| ctx.public_key_fingerprint.clone());
                    eprintln!(
                        "WARNING: ephemeral signing — this attestation is NOT release-grade. Public key written to {pk_desc} (fingerprint {})",
                        ctx.public_key_fingerprint
                    );
                }
            }
            Err(e) => attestation_error = Some(e.message),
        }
    }
    let attestation_failed = attestation_error.is_some();

    // Final outcome: if attestation failed, the verdict the CLI prints and the
    // unsigned artifact records flips to fail with the correct exit code. The
    // signed envelope (if any) carried `pre_outcome`, which by construction
    // saw `attestation_failed=false`, so it cannot lie about its own success.
    let final_outcome = if attestation_failed {
        GateOutcome {
            verdict: Verdict::Fail,
            attestation_failed: true,
            final_exit_code: exit::ATTEST_VERIFY_FAILED,
            ..pre_outcome.clone()
        }
    } else {
        pre_outcome.clone()
    };

    let final_statement = if final_outcome == pre_outcome {
        pre_statement
    } else {
        let final_commands = vec![CommandRun {
            argv: argv.clone(),
            exit_code: final_outcome.final_exit_code,
        }];
        chassis_core::attest::assemble(
            root,
            trace,
            drift,
            run.exempt_registry.as_ref(),
            final_commands,
            final_outcome.clone(),
            now,
        )
        .map_err(|e| CliError::internal(format!("assemble release gate: {e}")))?
    };

    let predicate_v = serde_json::to_value(&final_statement.predicate)
        .map_err(|e| CliError::internal(format!("serialize release-gate predicate: {e}")))?;
    validate_release_gate_value(&predicate_v)
        .map_err(|errs| CliError::internal(format!("release-gate schema invalid: {errs:?}")))?;
    write_json_file(&artifact_path, &predicate_v)?;

    let trace_summary = trace_summary_json(trace);
    let passed = final_outcome.verdict == Verdict::Pass;
    let spec_link_diagnostics_v = serde_json::to_value(&spec_link_diagnostics)
        .map_err(|e| CliError::internal(format!("serialize spec link diagnostics: {e}")))?;
    let output = json!({
        "ok": passed,
        "verdict": if passed { "pass" } else { "fail" },
        "fail_on_drift": fail_on_drift,
        "trace_failed": run.trace_failed(),
        "drift_failed": run.drift_failed(),
        "exemption_failed": run.exemption_failed(),
        "attestation_failed": attestation_failed,
        "spec_index_present": spec_index_present,
        "spec_index_digest": spec_index_digest,
        "spec_link_failed": spec_link_failed,
        "spec_link_diagnostics": spec_link_diagnostics_v,
        "unsuppressed_blocking": run.unsuppressed_blocking(),
        "suppressed": run.suppressed,
        "severity_overridden": run.overridden,
        "final_exit_code": final_outcome.final_exit_code,
        "schema_fingerprint": schema_fingerprint,
        "git_commit": git_commit,
        "contract_validation": contract_summary.to_json(),
        "trace_summary": trace_summary,
        "drift_summary": {
            "stale": drift.summary.stale,
            "abandoned": drift.summary.abandoned,
            "missing": drift.summary.missing,
            "unsuppressed_blocking": run.unsuppressed_blocking(),
        },
        "exemption_summary": exemption_summary_from_gate(&run),
        "artifact_path": artifact_path.display().to_string(),
        "attestation_path": attestation_written
            .as_ref()
            .map(|p| p.display().to_string()),
        "attestation_error": attestation_error,
        "attestation_release_grade": release_grade,
        "attestation_public_key_path": public_key_path_out,
        "attestation_public_key_fingerprint": public_key_fingerprint_out,
    });
    print_release_gate_output(&output, json);

    Ok(final_outcome.final_exit_code)
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
    let (spec_kit, spec_link_diags) = load_optional_spec_kit(root, &trace)?;
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
        spec_kit,
        spec_link_diags,
    ))
}

fn load_optional_spec_kit(
    root: &Path,
    trace: &chassis_core::trace::TraceGraph,
) -> Result<
    (
        Option<exports::SpecKitExtension>,
        Vec<chassis_core::diagnostic::Diagnostic>,
    ),
    CliError,
> {
    let p = root.join("artifacts/spec-index.json");
    if !p.is_file() {
        return Ok((None, Vec::new()));
    }
    let raw = read_text(&p)?;
    let v: Value = serde_json::from_str(&raw).map_err(|e| CliError::malformed(&p, e))?;
    validate_spec_index_value(&v).map_err(|errs| {
        CliError::malformed(
            &p,
            format!("artifacts/spec-index.json: {}", errs.join("; ")),
        )
    })?;
    let spec: SpecIndex = serde_json::from_value(v)
        .map_err(|e| CliError::malformed(&p, format!("spec index typed parse: {e}")))?;
    let digest = digest_sha256_hex(&spec).map_err(CliError::internal)?;
    let diags = link_spec_index(&spec, root, trace);
    Ok((
        Some(exports::SpecKitExtension {
            spec_index_digest: digest,
        }),
        diags,
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

fn exemption_summary_from_gate(run: &chassis_core::gate::GateRun) -> Value {
    let error_count = run
        .exempt_diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    json!({
        "registry_present": run.exempt_registry.is_some(),
        "active": run.exempt_active(),
        "diagnostics": run.exempt_diagnostics.len(),
        "errors": error_count,
        "suppressed": run.suppressed,
        "overridden": run.overridden,
        "audit": run.audit,
    })
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

/// Resolved Ed25519 signing material plus enough context for the CLI to
/// describe the resulting attestation honestly: whether it is release-grade,
/// where the corresponding public key landed on disk (if anywhere), and the
/// public-key fingerprint (hex-encoded 32-byte verifying key) so the verifier
/// side can confirm identity without trusting the signed envelope alone.
struct SignContext {
    signing_key: ed25519_dalek::SigningKey,
    release_grade: bool,
    public_key_fingerprint: String,
    public_key_path: Option<PathBuf>,
    ephemeral: bool,
}

fn hex_of_bytes(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn ephemeral_pubkey_path(envelope_out: &Path) -> PathBuf {
    let s = envelope_out.as_os_str().to_string_lossy().into_owned();
    PathBuf::from(format!("{s}.ephemeral.pub"))
}

/// If `priv_path` ends in `.priv`, return the sibling `.pub` path when that
/// file exists on disk. Used purely to surface a deterministic public-key
/// location in CLI output when one is conventionally present.
fn derived_pub_path(priv_path: &Path) -> Option<PathBuf> {
    let s = priv_path.to_string_lossy().into_owned();
    if let Some(stem) = s.strip_suffix(".priv") {
        let pub_path = PathBuf::from(format!("{stem}.pub"));
        if pub_path.exists() {
            return Some(pub_path);
        }
    }
    None
}

/// Resolve the signing key for an attest/release-gate run. The policy:
/// - `--ephemeral-key` opt-in: generate a fresh keypair, write its public key
///   next to the envelope, and mark the attestation as non-release-grade.
/// - explicit `--private-key`: read from that path.
/// - neither flag: require `.chassis/keys/release.priv`; if absent, fail
///   closed with CH-ATTEST-KEY-MISSING.
// @claim chassis.attest-key-policy-fail-closed
fn resolve_signing_key(
    root: &Path,
    private_key: Option<&Path>,
    ephemeral: bool,
    envelope_out: &Path,
) -> Result<SignContext, CliError> {
    use chassis_core::attest::sign::{generate_keypair, signing_key_from_hex, verifying_key_for};

    if ephemeral {
        let sk = generate_keypair();
        let vk = verifying_key_for(&sk);
        let pub_hex = hex_of_bytes(&vk.to_bytes());
        let pub_path = ephemeral_pubkey_path(envelope_out);
        if let Some(parent) = pub_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| CliError::internal(format!("mkdir {}: {e}", parent.display())))?;
        }
        fs::write(&pub_path, format!("{pub_hex}\n"))
            .map_err(|e| CliError::internal(format!("write {}: {e}", pub_path.display())))?;
        return Ok(SignContext {
            signing_key: sk,
            release_grade: false,
            public_key_fingerprint: pub_hex,
            public_key_path: Some(pub_path),
            ephemeral: true,
        });
    }

    let pk_path: PathBuf = match private_key {
        Some(p) => p.to_path_buf(),
        None => {
            let default = root.join(".chassis/keys/release.priv");
            if !default.exists() {
                return Err(CliError::attest_key_missing(&default));
            }
            default
        }
    };

    let sk_hex = read_text(&pk_path)?;
    let sk = signing_key_from_hex(sk_hex.trim())
        .map_err(|e| CliError::malformed(&pk_path, format!("private key: {e}")))?;
    let vk = verifying_key_for(&sk);
    let pub_hex = hex_of_bytes(&vk.to_bytes());
    let derived_pub = derived_pub_path(&pk_path);

    Ok(SignContext {
        signing_key: sk,
        release_grade: true,
        public_key_fingerprint: pub_hex,
        public_key_path: derived_pub,
        ephemeral: false,
    })
}

fn sign_envelope_with_context(
    root: &Path,
    statement: &chassis_core::attest::Statement,
    ctx: &SignContext,
    out: &Path,
) -> Result<(), CliError> {
    use chassis_core::attest::sign::{
        sign_statement, verify_envelope, verify_subject_matches_repo, verifying_key_for,
    };
    use chassis_core::attest::{CH_ATTEST_SUBJECT_MISMATCH, CH_ATTEST_VERIFY_FAILED};

    let mut envelope = sign_statement(statement, &ctx.signing_key)
        .map_err(|e| CliError::internal(format!("sign: {e}")))?;
    // DSSE `keyid` is informational and not part of the signed PAE bytes, but
    // it lets a verifier match the envelope to a public key without trusting
    // the payload first. `ephemeral:` is a stable marker that downstream code
    // and humans can use to refuse a release-grade gate.
    if let Some(sig) = envelope.signatures.get_mut(0) {
        sig.keyid = Some(if ctx.ephemeral {
            format!("ephemeral:{}", ctx.public_key_fingerprint)
        } else {
            ctx.public_key_fingerprint.clone()
        });
    }
    let env_v = serde_json::to_value(&envelope)
        .map_err(|e| CliError::internal(format!("serialize envelope: {e}")))?;
    validate_dsse_envelope_value(&env_v).map_err(|errs| {
        CliError::attest_verify(
            CH_ATTEST_VERIFY_FAILED,
            format!("DSSE envelope schema invalid: {errs:?}"),
        )
    })?;
    write_json_file(out, &env_v)?;

    let vk = verifying_key_for(&ctx.signing_key);
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
    println!(
        "blocking_reasons: trace={} drift={} exemption={} attestation={}",
        output["trace_failed"],
        output["drift_failed"],
        output["exemption_failed"],
        output["attestation_failed"],
    );
    println!(
        "exemptions: unsuppressed_blocking={} suppressed={} severity_overridden={}",
        output["unsuppressed_blocking"], output["suppressed"], output["severity_overridden"],
    );
    println!("final_exit_code={}", output["final_exit_code"]);
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
    ephemeral_key: bool,
    out: &Path,
    json: bool,
) -> Result<i32, CliError> {
    use chassis_core::attest::assemble;

    let now = Utc::now();
    let run = gate_compute(root, now, true).map_err(|e| {
        CliError::internal(format!("attest sign (gate): {}: {}", e.rule_id, e.message))
    })?;

    let ctx = resolve_signing_key(root, private_key, ephemeral_key, out)?;

    let outcome = run.outcome(false);
    let exit_code = outcome.final_exit_code;
    let mut argv = vec![
        "chassis".to_string(),
        "attest".to_string(),
        "sign".to_string(),
        "--repo".to_string(),
        root.display().to_string(),
    ];
    if ephemeral_key {
        argv.push("--ephemeral-key".to_string());
    }
    let commands = vec![CommandRun { argv, exit_code }];

    let stmt = assemble(
        root,
        &run.trace,
        &run.drift,
        run.exempt_registry.as_ref(),
        commands,
        outcome,
        now,
    )
    .map_err(|e| CliError::internal(format!("assemble: {e}")))?;

    sign_envelope_with_context(root, &stmt, &ctx, out)?;

    let sha256 = stmt
        .subject
        .first()
        .map(|s| s.digest.sha256.clone())
        .unwrap_or_default();

    if !ctx.release_grade {
        let pk_desc = ctx
            .public_key_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ctx.public_key_fingerprint.clone());
        eprintln!(
            "WARNING: ephemeral signing — this attestation is NOT release-grade. Public key written to {pk_desc} (fingerprint {})",
            ctx.public_key_fingerprint
        );
    }

    if json {
        let mut payload = json!({
            "ok": true,
            "out": out.display().to_string(),
            "sha256": sha256,
            "predicateType": stmt.predicate_type,
            "release_grade": ctx.release_grade,
            "public_key_fingerprint": ctx.public_key_fingerprint,
        });
        if let Some(p) = &ctx.public_key_path {
            payload["public_key_path"] = json!(p.display().to_string());
        }
        println!("{payload}");
    } else {
        println!("wrote {}", out.display());
        println!("release_grade={}", ctx.release_grade);
        println!("public_key_fingerprint={}", ctx.public_key_fingerprint);
        if let Some(p) = &ctx.public_key_path {
            println!("public_key_path={}", p.display());
        }
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
