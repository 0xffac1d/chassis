#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use chrono::Utc;
use clap::{Parser, Subcommand};
use ed25519_dalek::VerifyingKey;
use serde_json::{json, Value};

use chassis_core::contract::validate_metadata_contract;
use chassis_core::diff;
use chassis_core::drift::report::build_drift_report;
use chassis_core::exempt;
use chassis_core::exempt::Codeowners;
use chassis_core::trace::{build_trace_graph, render_mermaid};

#[derive(Parser, Debug)]
#[command(
    name = "chassis",
    version,
    about = "Chassis kernel CLI (validate, trace, drift, attest)"
)]
struct Cli {
    /// Repository root (default: current directory).
    #[arg(long, global = true, default_value = ".")]
    repo: PathBuf,

    /// Emit machine-readable JSON on stdout (where applicable).
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate a CONTRACT.yaml (or metadata contract YAML) against canonical JSON Schemas.
    Validate { path: PathBuf },
    /// Diff two contract YAML documents (JSON-compatible YAML).
    Diff { old: PathBuf, new: PathBuf },
    #[command(subcommand)]
    Exempt(ExemptCmd),
    /// Build the trace graph (`@claim` / CONTRACT join).
    Trace {
        #[arg(long)]
        mermaid: bool,
    },
    /// Build drift report for claims in the trace graph.
    Drift,
    #[command(subcommand)]
    Attest(AttestCmd),
}

#[derive(Subcommand, Debug)]
enum ExemptCmd {
    /// Run static exemption registry verification.
    Verify,
}

#[derive(Subcommand, Debug)]
enum AttestCmd {
    /// Assemble, sign, and write a DSSE envelope.
    Sign {
        #[arg(long)]
        private_key: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Verify a DSSE envelope.
    Verify {
        #[arg(long)]
        public_key: Option<PathBuf>,
        file: PathBuf,
    },
}

fn canon_repo(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn read_yaml_value(path: &Path) -> Value {
    let raw =
        fs::read_to_string(path).unwrap_or_else(|e| fail(&format!("read {}: {e}", path.display())));
    let y: serde_yaml::Value = serde_yaml::from_str(&raw)
        .unwrap_or_else(|e| fail(&format!("yaml {}: {e}", path.display())));
    serde_json::to_value(y).unwrap_or_else(|e| fail(&format!("json {}: {e}", path.display())))
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    process::exit(1)
}

fn main() {
    let cli = Cli::parse();
    let root = canon_repo(&cli.repo);

    match cli.command {
        Command::Validate { path } => {
            let v = read_yaml_value(&path);
            match validate_metadata_contract(&v) {
                Ok(()) => {
                    if cli.json {
                        println!("{}", json!({"ok": true}));
                    } else {
                        println!("ok {}", path.display());
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", json!({"ok": false, "errors": e}));
                    } else {
                        eprintln!("validation failed: {e:?}");
                    }
                    process::exit(2);
                }
            }
        }
        Command::Diff { old, new } => {
            let o = read_yaml_value(&old);
            let n = read_yaml_value(&new);
            match diff::diff(&o, &n) {
                Ok(rep) => {
                    if cli.json {
                        println!("{}", serde_json::to_string(&rep).unwrap());
                    } else {
                        println!("{rep:#?}");
                    }
                }
                Err(e) => fail(&format!("diff: {e:?}")),
            }
        }
        Command::Exempt(ExemptCmd::Verify) => {
            let p = root.join(".chassis/exemptions.yaml");
            let raw = fs::read_to_string(&p)
                .unwrap_or_else(|e| fail(&format!("read {}: {e}", p.display())));
            let y: serde_yaml::Value =
                serde_yaml::from_str(&raw).unwrap_or_else(|e| fail(&format!("yaml: {e}")));
            let j = serde_json::to_value(y).unwrap_or_else(|e| fail(&format!("json: {e}")));
            let reg: exempt::Registry =
                serde_json::from_value(j).unwrap_or_else(|e| fail(&format!("registry: {e}")));
            let co_path = root.join("CODEOWNERS");
            let co_raw = fs::read_to_string(&co_path).unwrap_or_default();
            let codeowners = Codeowners::parse(&co_raw).unwrap_or_else(|e| fail(&format!("{e}")));
            let diags = exempt::verify(&reg, Utc::now(), &codeowners);
            if cli.json {
                println!("{}", serde_json::json!({ "diagnostics": diags }));
            } else {
                for d in &diags {
                    println!("{d:?}");
                }
            }
            if diags
                .iter()
                .any(|d| d.severity == chassis_core::diagnostic::Severity::Error)
            {
                process::exit(3);
            }
        }
        Command::Trace { mermaid } => match build_trace_graph(&root) {
            Ok(g) => {
                if mermaid {
                    println!("{}", render_mermaid(&g));
                } else if cli.json {
                    println!("{}", serde_json::to_string(&g).unwrap());
                } else {
                    println!("claims={} orphans={}", g.claims.len(), g.orphan_sites.len());
                }
            }
            Err(e) => fail(&format!("trace: {e}")),
        },
        Command::Drift => {
            let trace = build_trace_graph(&root).unwrap_or_else(|e| fail(&format!("trace: {e}")));
            match build_drift_report(&root, &trace, Utc::now()) {
                Ok(r) => {
                    if cli.json {
                        println!("{}", serde_json::to_string(&r).unwrap());
                    } else {
                        println!(
                            "stale={} abandoned={} missing={}",
                            r.summary.stale, r.summary.abandoned, r.summary.missing
                        );
                    }
                }
                Err(e) => fail(&format!("drift: {e}")),
            }
        }
        Command::Attest(sub) => match sub {
            AttestCmd::Sign { private_key, out } => {
                use chassis_core::attest::sign::signing_key_from_hex;
                use chassis_core::attest::{assemble, sign_statement};

                let pk_path =
                    private_key.unwrap_or_else(|| root.join(".chassis/keys/release.priv"));
                let sk_hex = fs::read_to_string(&pk_path)
                    .unwrap_or_else(|e| fail(&format!("read priv: {e}")));
                let sk = signing_key_from_hex(&sk_hex).unwrap_or_else(|e| {
                    fail(&format!(
                        "{}: {e}",
                        chassis_core::attest::CH_ATTEST_SIGN_FAILED
                    ))
                });
                let trace =
                    build_trace_graph(&root).unwrap_or_else(|e| fail(&format!("trace: {e}")));
                let drift = build_drift_report(&root, &trace, Utc::now())
                    .unwrap_or_else(|e| fail(&format!("drift: {e}")));
                let ex = load_exemptions(&root);
                let stmt = assemble(&root, &trace, &drift, ex.as_ref(), vec![], Utc::now())
                    .unwrap_or_else(|e| fail(&format!("assemble: {e}")));
                let env =
                    sign_statement(&stmt, &sk).unwrap_or_else(|e| fail(&format!("sign: {e}")));
                let txt = serde_json::to_string_pretty(&env)
                    .unwrap_or_else(|e| fail(&format!("json: {e}")));
                fs::write(&out, txt).unwrap_or_else(|e| fail(&format!("write: {e}")));
                if !cli.json {
                    println!("wrote {}", out.display());
                }
            }
            AttestCmd::Verify { public_key, file } => {
                use chassis_core::attest::sign::{
                    verify_envelope, verify_subject_matches_repo, verifying_key_from_hex,
                };

                let pub_path = public_key.unwrap_or_else(|| root.join(".chassis/keys/release.pub"));
                let pub_hex = fs::read_to_string(&pub_path)
                    .unwrap_or_else(|e| fail(&format!("read pub: {e}")));
                let vk: VerifyingKey = verifying_key_from_hex(&pub_hex).unwrap_or_else(|e| {
                    fail(&format!(
                        "{}: {e}",
                        chassis_core::attest::CH_ATTEST_VERIFY_FAILED
                    ))
                });
                let raw = fs::read_to_string(&file).unwrap_or_else(|e| fail(&format!("read: {e}")));
                let env: chassis_core::attest::DsseEnvelope =
                    serde_json::from_str(&raw).unwrap_or_else(|e| fail(&format!("parse: {e}")));
                let stmt =
                    verify_envelope(&env, &vk).unwrap_or_else(|e| fail(&format!("verify: {e}")));
                verify_subject_matches_repo(&stmt, &root).unwrap_or_else(|e| fail(&format!("{e}")));
                if cli.json {
                    println!("{}", serde_json::to_string(&stmt).unwrap());
                } else {
                    println!("ok {}", file.display());
                }
            }
        },
    }
}

fn load_exemptions(root: &Path) -> Option<exempt::Registry> {
    let p = root.join(".chassis/exemptions.yaml");
    let raw = fs::read_to_string(p).ok()?;
    let y: serde_yaml::Value = serde_yaml::from_str(&raw).ok()?;
    let j = serde_json::to_value(y).ok()?;
    serde_json::from_value(j).ok()
}
