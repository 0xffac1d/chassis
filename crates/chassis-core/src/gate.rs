#![forbid(unsafe_code)]

//! Shared release-gate computation.
//!
//! The CLI's `chassis release-gate` and the JSON-RPC `release_gate` method
//! must produce the **same** predicate/verdict shape — otherwise an agent
//! using the JSON-RPC kernel surface and a human using the CLI would
//! disagree about whether a given repo is ready to ship. This module is the
//! one place that walks the repo, builds the trace + drift + exemption
//! state, and condenses it into a [`GateOutcome`] + [`ReleaseGatePredicate`]
//! pair that both surfaces serialize.
//!
//! Signing is out of scope here. Callers that want a signed DSSE envelope
//! pass the predicate to [`crate::attest::assemble`] + sign it themselves.
//!
//! **Git checkout required:** [`compute`] fails fast with
//! [`rule_id::GIT_METADATA_REQUIRED`] when the repo root has no openable `.git`
//! (for example an extracted source archive). Drift scoring and
//! [`GateRun::git_commit`] depend on Git history and `HEAD`.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use git2::Repository;
use serde::{Deserialize, Serialize};

use crate::attest::assemble::GateOutcome;
use crate::attest::predicate::{
    CommandRun, DriftSummary, ExemptSummary, ReleaseGatePredicate, ScannerPredicateSummary,
    ScannerSarifDigests, TraceSummary, Verdict,
};
use crate::contract::validate_metadata_contract;
use crate::diagnostic::{Diagnostic, Severity};
use crate::drift::report::{build_drift_report, validate_drift_report_json, DriftReport};
use crate::exempt::{
    apply::apply_exemptions, list, verify as exempt_verify, Codeowners, ListFilter, Registry,
};
use crate::fingerprint;
use crate::scanner::{self, ScannerSummary, ScannerTool};
use crate::spec_index::{digest_sha256_hex, link_spec_index, validate_spec_index_value, SpecIndex};
use crate::trace::{build_trace_graph, types::TraceGraph, validate_trace_graph};

/// Stable rule ids surfaced when the gate computation itself fails. Bound to
/// the in-tree CONTRACT.yaml claim `chassis.jsonrpc-release-gate-honest`.
pub mod rule_id {
    /// The repo root is missing or unreadable — no trace graph could be built.
    pub const REPO_UNREADABLE: &str = "CH-GATE-REPO-UNREADABLE";
    /// No Git metadata at `--repo` (e.g. extracted `tar.gz` / zip source drop).
    /// Drift scoring and the predicate's `git_commit` field require a checkout.
    pub const GIT_METADATA_REQUIRED: &str = "CH-GATE-GIT-METADATA-REQUIRED";
    /// At least one repo `CONTRACT.yaml` failed its canonical schema check
    /// during preflight. Contract validation runs before trace/drift/exempt
    /// and is **not** exemption-applicable: a contract that cannot be parsed
    /// or doesn't satisfy `schemas/contract.schema.json` is a release-gate
    /// prerequisite, not a normalized diagnostic findings stream.
    pub const CONTRACT_INVALID: &str = "CH-GATE-CONTRACT-INVALID";
    /// Trace, drift, or git inspection failed below the gate.
    pub const SUBSYSTEM_FAILURE: &str = "CH-GATE-SUBSYSTEM-FAILURE";
    /// The exemption registry file existed but failed to parse.
    pub const REGISTRY_MALFORMED: &str = "CH-GATE-REGISTRY-MALFORMED";
    /// An emitted artifact did not satisfy its canonical JSON Schema.
    pub const SCHEMA_INVALID: &str = "CH-GATE-SCHEMA-INVALID";
    /// `--require-scanners` / CI path: normalized scanner evidence missing,
    /// corrupt, incomplete, or reporting blocking findings when required clean.
    pub const SCANNER_EVIDENCE_REQUIRED: &str = "CH-GATE-SCANNER-EVIDENCE-REQUIRED";
}

/// Error from [`compute`]. Each variant carries a stable rule id so the CLI
/// and JSON-RPC surfaces map gate failures onto the same diagnostic vocabulary.
///
/// `contract_report` is `Some` only for `CONTRACT_INVALID`, allowing a caller
/// (CLI or JSON-RPC) to render the per-contract structured summary without
/// re-walking the tree. For every other rule id it is `None`.
#[derive(Debug)]
pub struct GateError {
    pub rule_id: &'static str,
    pub message: String,
    pub contract_report: Option<ContractReport>,
}

impl GateError {
    fn new(rule_id: &'static str, message: impl Into<String>) -> Self {
        Self {
            rule_id,
            message: message.into(),
            contract_report: None,
        }
    }

    fn with_contract_report(mut self, report: ContractReport) -> Self {
        self.contract_report = Some(report);
        self
    }
}

impl std::fmt::Display for GateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule_id, self.message)
    }
}

impl std::error::Error for GateError {}

/// Per-contract validation outcome surfaced by [`validate_repo_contracts`].
/// Mirrors the structured `contract_validation` payload the CLI prints, so
/// CLI and JSON-RPC callers can render the same JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractError {
    /// Path of the failing CONTRACT.yaml, repo-relative when possible.
    pub path: String,
    /// Stable rule id for the failure (typically `CH-RUST-METADATA-CONTRACT`,
    /// `CH-GATE-CONTRACT-MISSING`, or `CH-GATE-CONTRACT-MALFORMED`).
    pub code: String,
    /// Human-readable failure detail.
    pub message: String,
}

/// Whole-repo contract preflight report. `invalid > 0` means the gate must
/// fail closed; trace/drift/exemption inputs are skipped in that case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractReport {
    pub checked: usize,
    pub valid: usize,
    pub invalid: usize,
    pub errors: Vec<ContractError>,
}

impl ContractReport {
    /// True iff at least one contract failed validation.
    pub fn has_invalid(&self) -> bool {
        self.invalid > 0
    }
}

/// Stable rule id for a contract that failed `schemas/contract.schema.json`.
const CH_RUST_METADATA_CONTRACT: &str = "CH-RUST-METADATA-CONTRACT";
/// Stable rule id when a contract file can't be read or YAML-parsed.
const CH_GATE_CONTRACT_MALFORMED: &str = "CH-GATE-CONTRACT-MALFORMED";

/// Walk the repo for `CONTRACT.yaml` files and validate each against
/// `schemas/contract.schema.json`. Skips `target/`, `node_modules/`, `.git/`,
/// `fixtures/`, and `reference/` (matching the CLI and trace walkers so the
/// preflight, trace graph, and CLI summary all see the same set of contracts).
///
/// Used by [`compute`] as a fail-closed preflight; CLI callers may invoke it
/// directly to format the structured `contract_validation` summary, then hand
/// the resulting `ContractReport` back to the user before [`compute`] runs.
pub fn validate_repo_contracts(repo: &Path) -> Result<ContractReport, GateError> {
    if !repo.is_dir() {
        return Err(GateError::new(
            rule_id::REPO_UNREADABLE,
            format!("repo root not a directory: {}", repo.display()),
        ));
    }
    let contracts = discover_contract_files(repo).map_err(|e| {
        GateError::new(
            rule_id::SUBSYSTEM_FAILURE,
            format!("walk for CONTRACT.yaml under {}: {e}", repo.display()),
        )
    })?;

    let mut report = ContractReport {
        checked: contracts.len(),
        valid: 0,
        invalid: 0,
        errors: Vec::new(),
    };

    for path in contracts {
        let rel = path
            .strip_prefix(repo)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.clone());
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                report.invalid += 1;
                report.errors.push(ContractError {
                    path: rel.display().to_string(),
                    code: CH_GATE_CONTRACT_MALFORMED.to_string(),
                    message: format!("read: {e}"),
                });
                continue;
            }
        };
        let yaml: serde_yaml::Value = match serde_yaml::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                report.invalid += 1;
                report.errors.push(ContractError {
                    path: rel.display().to_string(),
                    code: CH_GATE_CONTRACT_MALFORMED.to_string(),
                    message: format!("yaml parse: {e}"),
                });
                continue;
            }
        };
        let value = match serde_json::to_value(yaml) {
            Ok(v) => v,
            Err(e) => {
                report.invalid += 1;
                report.errors.push(ContractError {
                    path: rel.display().to_string(),
                    code: CH_GATE_CONTRACT_MALFORMED.to_string(),
                    message: format!("yaml→json: {e}"),
                });
                continue;
            }
        };
        match validate_metadata_contract(&value) {
            Ok(()) => report.valid += 1,
            Err(errs) => {
                report.invalid += 1;
                report.errors.push(ContractError {
                    path: rel.display().to_string(),
                    code: CH_RUST_METADATA_CONTRACT.to_string(),
                    message: errs.join("; "),
                });
            }
        }
    }
    Ok(report)
}

fn discover_contract_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for ent in std::fs::read_dir(dir)? {
            let ent = ent?;
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

/// Optional Spec Kit index (`artifacts/spec-index.json`) linkage state for the gate.
#[derive(Debug, Clone)]
pub struct SpecGateState {
    pub present: bool,
    pub digest: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl SpecGateState {
    pub fn failed(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }
}

/// Resolved gate state — everything needed to derive the predicate, the
/// blocking reasons, and the human-readable verdict envelope. Owned values
/// so callers can re-use parts (e.g. a CLI prints contract validation, a
/// JSON-RPC surface returns only the predicate).
#[derive(Debug)]
pub struct GateRun {
    /// Preflight contract-validation summary. Always populated and always
    /// passing on a successful [`compute`] (a failing contract would have
    /// short-circuited with `CONTRACT_INVALID`).
    pub contracts: ContractReport,
    pub trace: TraceGraph,
    pub drift: DriftReport,
    pub spec: SpecGateState,
    pub exempt_registry: Option<Registry>,
    pub exempt_diagnostics: Vec<Diagnostic>,
    pub unsuppressed_drift: Vec<Diagnostic>,
    pub suppressed: usize,
    pub overridden: usize,
    pub audit: usize,
    pub schema_fingerprint: String,
    pub git_commit: String,
    pub fail_on_drift: bool,
    pub now: DateTime<Utc>,
    pub scanner_summaries: Vec<ScannerSummary>,
    pub unsuppressed_scanner: Vec<Diagnostic>,
}

impl GateRun {
    /// True if the trace has orphan sites, or if any contract claim lacks
    /// implementation or test sites (matches `chassis release-gate` semantics).
    pub fn trace_failed(&self) -> bool {
        !self.trace.orphan_sites.is_empty()
            || self
                .trace
                .claims
                .values()
                .any(|n| n.impl_sites.is_empty() || n.test_sites.is_empty())
    }

    /// True iff `fail_on_drift` is set AND at least one unsuppressed drift
    /// diagnostic carries `error` or `warning` severity.
    pub fn drift_failed(&self) -> bool {
        self.fail_on_drift
            && self
                .unsuppressed_drift
                .iter()
                .any(|d| matches!(d.severity, Severity::Error | Severity::Warning))
    }

    /// True iff the exemption-registry verifier emitted at least one error.
    pub fn exemption_failed(&self) -> bool {
        self.exempt_diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Count of unsuppressed trace/spec/drift diagnostics at error/warning
    /// severity after exemptions were applied (used by the release-gate
    /// predicate and exported summaries).
    pub fn unsuppressed_blocking(&self) -> usize {
        let blocking = |d: &&Diagnostic| matches!(d.severity, Severity::Error | Severity::Warning);
        self.trace.diagnostics.iter().filter(blocking).count()
            + self.spec.diagnostics.iter().filter(blocking).count()
            + self.unsuppressed_drift.iter().filter(blocking).count()
            + self.unsuppressed_scanner.iter().filter(blocking).count()
    }

    /// Active (non-revoked, non-expired) entries in the registry, as of `now`.
    pub fn exempt_active(&self) -> usize {
        match &self.exempt_registry {
            None => 0,
            Some(reg) => list(
                reg,
                ListFilter {
                    rule_id: None,
                    path: None,
                    active_at: Some(self.now.date_naive()),
                },
            )
            .len(),
        }
    }

    pub fn spec_failed(&self) -> bool {
        self.spec.failed()
    }

    pub fn scanner_failed(&self) -> bool {
        self.unsuppressed_scanner
            .iter()
            .any(|d| matches!(d.severity, Severity::Error | Severity::Warning))
    }

    pub fn scanner_predicate_summary(&self) -> ScannerPredicateSummary {
        let mut tools = Vec::new();
        let mut errors = 0usize;
        let mut warnings = 0usize;
        let mut digests = ScannerSarifDigests {
            semgrep: None,
            codeql: None,
        };
        for s in &self.scanner_summaries {
            tools.push(match s.tool {
                ScannerTool::Semgrep => "semgrep".to_string(),
                ScannerTool::Codeql => "codeql".to_string(),
            });
            errors += s.errors;
            warnings += s.warnings;
            match s.tool {
                ScannerTool::Semgrep => digests.semgrep = Some(s.sarif_sha256.clone()),
                ScannerTool::Codeql => digests.codeql = Some(s.sarif_sha256.clone()),
            }
        }
        ScannerPredicateSummary {
            tools,
            errors,
            warnings,
            sarif_digests: digests,
        }
    }

    /// Build the per-axis [`GateOutcome`] embedded into the signed predicate.
    /// `attestation_failed` reflects whether signing succeeded (false when the
    /// caller did not attempt to sign).
    pub fn outcome(&self, attestation_failed: bool) -> GateOutcome {
        let trace_failed = self.trace_failed();
        let drift_failed = self.drift_failed();
        let exemption_failed = self.exemption_failed();
        let spec_failed = self.spec_failed();
        let scanner_failed = self.scanner_failed();
        let passed = !spec_failed
            && !trace_failed
            && !drift_failed
            && !exemption_failed
            && !attestation_failed
            && !scanner_failed;
        // Exit code precedence matches `chassis release-gate`.
        let final_exit_code = if passed {
            0
        } else if attestation_failed {
            6
        } else if spec_failed {
            2
        } else if trace_failed || drift_failed || scanner_failed {
            5
        } else if exemption_failed {
            3
        } else {
            5
        };
        GateOutcome {
            verdict: if passed { Verdict::Pass } else { Verdict::Fail },
            fail_on_drift: self.fail_on_drift,
            trace_failed,
            drift_failed,
            exemption_failed,
            attestation_failed,
            scanner_failed,
            spec_index_present: self.spec.present,
            spec_index_digest: self.spec.digest.clone(),
            spec_failed,
            spec_error_count: self.spec.error_count(),
            unsuppressed_blocking: self.unsuppressed_blocking(),
            suppressed: self.suppressed,
            severity_overridden: self.overridden,
            final_exit_code,
            scanner_summary: self.scanner_predicate_summary(),
        }
    }

    /// Assemble the canonical release-gate predicate. Both the JSON-RPC
    /// surface and the CLI render this through `serde_json::to_value` and
    /// validate against `schemas/release-gate.schema.json`.
    pub fn predicate(
        &self,
        commands_run: Vec<CommandRun>,
        attestation_failed: bool,
    ) -> Result<ReleaseGatePredicate, GateError> {
        let outcome = self.outcome(attestation_failed);
        let today = self.now.date_naive();
        let trace_summary = TraceSummary {
            claims: self.trace.claims.len(),
            orphan_sites: self.trace.orphan_sites.len(),
        };
        let drift_summary = DriftSummary {
            stale: self.drift.summary.stale,
            abandoned: self.drift.summary.abandoned,
            missing: self.drift.summary.missing,
        };
        let exempt_summary = match &self.exempt_registry {
            None => ExemptSummary {
                active: 0,
                expired_present: 0,
            },
            Some(reg) => {
                let mut active = 0usize;
                let mut expired_present = 0usize;
                for e in &reg.entries {
                    if e.status == crate::exempt::ExemptionStatus::Revoked {
                        continue;
                    }
                    let expired =
                        e.expires_at < today || e.status == crate::exempt::ExemptionStatus::Expired;
                    if expired {
                        expired_present += 1;
                    } else if e.status == crate::exempt::ExemptionStatus::Active {
                        active += 1;
                    }
                }
                ExemptSummary {
                    active,
                    expired_present,
                }
            }
        };

        let predicate = ReleaseGatePredicate {
            schema_fingerprint: self.schema_fingerprint.clone(),
            git_commit: self.git_commit.clone(),
            built_at: self.now.to_rfc3339(),
            verdict: outcome.verdict,
            fail_on_drift: outcome.fail_on_drift,
            trace_failed: outcome.trace_failed,
            drift_failed: outcome.drift_failed,
            exemption_failed: outcome.exemption_failed,
            attestation_failed: outcome.attestation_failed,
            scanner_failed: outcome.scanner_failed,
            spec_index_present: outcome.spec_index_present,
            spec_index_digest: outcome.spec_index_digest.clone(),
            spec_failed: outcome.spec_failed,
            spec_error_count: outcome.spec_error_count,
            unsuppressed_blocking: outcome.unsuppressed_blocking,
            suppressed: outcome.suppressed,
            severity_overridden: outcome.severity_overridden,
            final_exit_code: outcome.final_exit_code,
            trace_summary,
            drift_summary,
            exempt_summary,
            scanner_summary: outcome.scanner_summary.clone(),
            commands_run,
        };

        let value = serde_json::to_value(&predicate)
            .map_err(|e| GateError::new(rule_id::SCHEMA_INVALID, format!("serialize: {e}")))?;
        crate::attest::predicate::validate_release_gate_predicate(&value).map_err(|errs| {
            GateError::new(
                rule_id::SCHEMA_INVALID,
                format!("release-gate predicate failed schema: {errs:?}"),
            )
        })?;
        Ok(predicate)
    }
}

/// Walk the repo and build every input the release-gate verdict depends on.
///
/// Caller controls `fail_on_drift` so the CLI's `--fail-on-drift` flag and a
/// future JSON-RPC `params.fail_on_drift` option produce matching verdicts.
///
/// When `require_scanners` is true (CLI: `--require-scanners`), both
/// `dist/scanner-semgrep.json` and `dist/scanner-codeql.json` must exist and
/// validate; otherwise [`GateError`] carries [`rule_id::SCANNER_EVIDENCE_REQUIRED`].
pub fn compute(
    repo: &Path,
    now: DateTime<Utc>,
    fail_on_drift: bool,
    require_scanners: bool,
) -> Result<GateRun, GateError> {
    if !repo.is_dir() {
        return Err(GateError::new(
            rule_id::REPO_UNREADABLE,
            format!("repo root not a directory: {}", repo.display()),
        ));
    }

    require_git_worktree(repo)?;

    // Fail-closed contract preflight: a release-gate predicate cannot describe
    // a repo whose CONTRACT.yaml files don't satisfy the canonical schema. The
    // CLI also surfaces the structured report in its output envelope; both
    // surfaces share this exact code path so JSON-RPC and CLI agree.
    let contracts = validate_repo_contracts(repo)?;
    if contracts.has_invalid() {
        let summary = format!(
            "{} of {} contract(s) failed canonical schema validation; first: {}",
            contracts.invalid,
            contracts.checked,
            contracts
                .errors
                .first()
                .map(|e| format!("{} [{}]: {}", e.path, e.code, e.message))
                .unwrap_or_else(|| "unknown".to_string())
        );
        return Err(
            GateError::new(rule_id::CONTRACT_INVALID, summary).with_contract_report(contracts)
        );
    }

    let mut trace = build_trace_graph(repo).map_err(|e| {
        GateError::new(
            rule_id::SUBSYSTEM_FAILURE,
            format!("build trace graph: {e}"),
        )
    })?;
    validate_trace_graph(&trace).map_err(|errs| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("trace graph failed schema: {errs:?}"),
        )
    })?;

    let mut spec = load_spec_index_gate(repo, &trace)?;

    let mut drift = build_drift_report(repo, &trace, now).map_err(|e| {
        GateError::new(
            rule_id::SUBSYSTEM_FAILURE,
            format!("build drift report: {e}"),
        )
    })?;
    let drift_value = serde_json::to_value(&drift).map_err(|e| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("serialize drift report: {e}"),
        )
    })?;
    validate_drift_report_json(&drift_value).map_err(|errs| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("drift report failed schema: {errs:?}"),
        )
    })?;

    let scanner_summaries_loaded = match scanner::load_scanner_summaries_from_repo(repo) {
        Ok(s) => s,
        Err(e) => {
            return Err(GateError::new(
                if require_scanners {
                    rule_id::SCANNER_EVIDENCE_REQUIRED
                } else {
                    rule_id::SUBSYSTEM_FAILURE
                },
                e,
            ));
        }
    };
    if require_scanners {
        let has_semgrep = scanner_summaries_loaded
            .iter()
            .any(|s| s.tool == ScannerTool::Semgrep);
        let has_codeql = scanner_summaries_loaded
            .iter()
            .any(|s| s.tool == ScannerTool::Codeql);
        if !has_semgrep || !has_codeql {
            return Err(GateError::new(
                rule_id::SCANNER_EVIDENCE_REQUIRED,
                "require-scanners: expected dist/scanner-semgrep.json and dist/scanner-codeql.json",
            ));
        }
        // Plan / CI: require-scanners means normalized evidence must be clean
        // (summary.errors is pre-exemption SARIF rollup).
        if scanner_summaries_loaded.iter().any(|s| s.errors > 0) {
            return Err(GateError::new(
                rule_id::SCANNER_EVIDENCE_REQUIRED,
                "require-scanners: scanner summaries must report errors == 0",
            ));
        }
    }

    let scan_flat: Vec<Diagnostic> = scanner_summaries_loaded
        .iter()
        .flat_map(|s| s.diagnostics.clone())
        .collect();

    let exempt = load_and_apply_exemptions(
        repo,
        std::mem::take(&mut trace.diagnostics),
        std::mem::take(&mut spec.diagnostics),
        std::mem::take(&mut drift.diagnostics),
        scan_flat,
        now,
    )?;

    trace.diagnostics = exempt.unsuppressed_trace;
    spec.diagnostics = exempt.unsuppressed_spec;
    let drift_unsuppressed = exempt.unsuppressed_drift;
    drift.diagnostics = drift_unsuppressed.clone();

    let scanner_summaries = crate::exports::rebuild_scanner_summaries(
        &scanner_summaries_loaded,
        &exempt.unsuppressed_scanner,
    );
    let unsuppressed_scanner = exempt.unsuppressed_scanner;

    let schema_fingerprint = fingerprint::compute(repo)
        .map_err(|e| GateError::new(rule_id::SUBSYSTEM_FAILURE, format!("fingerprint: {e}")))?;
    let git_commit = git_head(repo)?;

    Ok(GateRun {
        contracts,
        trace,
        drift,
        spec,
        exempt_registry: exempt.registry,
        exempt_diagnostics: exempt.diagnostics,
        unsuppressed_drift: drift_unsuppressed,
        suppressed: exempt.suppressed,
        overridden: exempt.overridden,
        audit: exempt.audit,
        schema_fingerprint,
        git_commit,
        fail_on_drift,
        now,
        scanner_summaries,
        unsuppressed_scanner,
    })
}

fn require_git_worktree(repo: &Path) -> Result<(), GateError> {
    let r = Repository::open(repo).map_err(|e| {
        GateError::new(
            rule_id::GIT_METADATA_REQUIRED,
            format!(
                "release-gate is not runnable on a plain source tree at {}: no `.git` metadata ({}). \
                 Use a Git clone or full checkout; extracted release archives omit history and HEAD.",
                repo.display(),
                e.message()
            ),
        )
    })?;
    r.head().map_err(|e| {
        GateError::new(
            rule_id::GIT_METADATA_REQUIRED,
            format!(
                "release-gate requires a readable Git HEAD at {}: {}",
                repo.display(),
                e.message()
            ),
        )
    })?;
    Ok(())
}

fn load_spec_index_gate(repo: &Path, trace: &TraceGraph) -> Result<SpecGateState, GateError> {
    let p = repo.join("artifacts/spec-index.json");
    if !p.is_file() {
        return Ok(SpecGateState {
            present: false,
            digest: None,
            diagnostics: Vec::new(),
        });
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("read {}: {e}", p.display()),
        )
    })?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("{}: JSON: {e}", p.display()),
        )
    })?;
    validate_spec_index_value(&v).map_err(|errs| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("{}: {}", p.display(), errs.join("; ")),
        )
    })?;
    let spec: SpecIndex = serde_json::from_value(v).map_err(|e| {
        GateError::new(
            rule_id::SCHEMA_INVALID,
            format!("{}: spec index shape: {e}", p.display()),
        )
    })?;
    let digest =
        digest_sha256_hex(&spec).map_err(|e| GateError::new(rule_id::SCHEMA_INVALID, e))?;
    let diagnostics = link_spec_index(&spec, repo, trace);
    Ok(SpecGateState {
        present: true,
        digest: Some(digest),
        diagnostics,
    })
}

fn git_head(repo: &Path) -> Result<String, GateError> {
    let r = Repository::open(repo)
        .map_err(|e| GateError::new(rule_id::SUBSYSTEM_FAILURE, format!("open repo: {e}")))?;
    let head = r
        .head()
        .map_err(|e| GateError::new(rule_id::SUBSYSTEM_FAILURE, format!("HEAD: {e}")))?;
    let oid = head
        .peel_to_commit()
        .map_err(|e| GateError::new(rule_id::SUBSYSTEM_FAILURE, format!("peel commit: {e}")))?;
    Ok(oid.id().to_string())
}

/// Output of [`load_and_apply_exemptions`]: the parsed registry (if any), the
/// verifier diagnostics, the summed suppressed/overridden/audit counts from
/// applying the registry independently across trace/spec/drift diagnostic
/// buckets, plus each bucket's surviving diagnostics.
struct ExemptionState {
    registry: Option<Registry>,
    diagnostics: Vec<Diagnostic>,
    suppressed: usize,
    overridden: usize,
    audit: usize,
    unsuppressed_trace: Vec<Diagnostic>,
    unsuppressed_spec: Vec<Diagnostic>,
    unsuppressed_drift: Vec<Diagnostic>,
    unsuppressed_scanner: Vec<Diagnostic>,
}

fn load_and_apply_exemptions(
    repo: &Path,
    trace_diag: Vec<Diagnostic>,
    spec_diag: Vec<Diagnostic>,
    drift_diag: Vec<Diagnostic>,
    scanner_diag: Vec<Diagnostic>,
    now: DateTime<Utc>,
) -> Result<ExemptionState, GateError> {
    let p = repo.join(".chassis/exemptions.yaml");
    if !p.exists() {
        return Ok(ExemptionState {
            registry: None,
            diagnostics: Vec::new(),
            suppressed: 0,
            overridden: 0,
            audit: 0,
            unsuppressed_trace: trace_diag,
            unsuppressed_spec: spec_diag,
            unsuppressed_drift: drift_diag,
            unsuppressed_scanner: scanner_diag,
        });
    }
    let raw = std::fs::read_to_string(&p).map_err(|e| {
        GateError::new(
            rule_id::REGISTRY_MALFORMED,
            format!("read {}: {e}", p.display()),
        )
    })?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&raw)
        .map_err(|e| GateError::new(rule_id::REGISTRY_MALFORMED, format!("parse YAML: {e}")))?;
    let value = serde_json::to_value(yaml)
        .map_err(|e| GateError::new(rule_id::REGISTRY_MALFORMED, format!("to JSON: {e}")))?;
    let registry: Registry = serde_json::from_value(value)
        .map_err(|e| GateError::new(rule_id::REGISTRY_MALFORMED, format!("registry shape: {e}")))?;
    let co_path = repo.join("CODEOWNERS");
    let co_raw = std::fs::read_to_string(&co_path).unwrap_or_default();
    let codeowners = Codeowners::parse(&co_raw)
        .map_err(|e| GateError::new(rule_id::REGISTRY_MALFORMED, format!("CODEOWNERS: {e}")))?;
    let exempt_diagnostics = exempt_verify(&registry, now, &codeowners);

    let mut suppressed = 0usize;
    let mut overridden = 0usize;
    let mut audit = 0usize;

    let applied_trace = apply_exemptions(trace_diag, &registry, now);
    suppressed += applied_trace.suppressed.len();
    overridden += applied_trace.overridden.len();
    audit += applied_trace.audit.len();
    let trace_us = applied_trace.unsuppressed;

    let applied_spec = apply_exemptions(spec_diag, &registry, now);
    suppressed += applied_spec.suppressed.len();
    overridden += applied_spec.overridden.len();
    audit += applied_spec.audit.len();
    let spec_us = applied_spec.unsuppressed;

    let applied_drift = apply_exemptions(drift_diag, &registry, now);
    suppressed += applied_drift.suppressed.len();
    overridden += applied_drift.overridden.len();
    audit += applied_drift.audit.len();
    let drift_us = applied_drift.unsuppressed;

    let applied_scanner = apply_exemptions(scanner_diag, &registry, now);
    suppressed += applied_scanner.suppressed.len();
    overridden += applied_scanner.overridden.len();
    audit += applied_scanner.audit.len();
    let scanner_us = applied_scanner.unsuppressed;

    Ok(ExemptionState {
        registry: Some(registry),
        diagnostics: exempt_diagnostics,
        suppressed,
        overridden,
        audit,
        unsuppressed_trace: trace_us,
        unsuppressed_spec: spec_us,
        unsuppressed_drift: drift_us,
        unsuppressed_scanner: scanner_us,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn compute_produces_schema_valid_predicate_for_self_repo() {
        let repo = repo_root();
        let run = compute(&repo, Utc::now(), true, false).expect("gate compute");
        let predicate = run.predicate(vec![], false).expect("predicate");
        let v = serde_json::to_value(&predicate).unwrap();
        crate::attest::predicate::validate_release_gate_predicate(&v)
            .expect("self-repo predicate matches schema");
    }

    #[test]
    fn compute_rejects_missing_repo() {
        let bogus = std::path::PathBuf::from("/this/path/does/not/exist/anywhere");
        let err = compute(&bogus, Utc::now(), true, false).expect_err("must fail for missing repo");
        assert_eq!(err.rule_id, rule_id::REPO_UNREADABLE);
    }

    #[test]
    fn compute_rejects_source_tree_without_git() {
        let base = std::env::temp_dir().join(format!(
            "chassis-gate-no-git-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let err = compute(&base, Utc::now(), true, false).expect_err("must require git metadata");
        assert_eq!(err.rule_id, rule_id::GIT_METADATA_REQUIRED);
        assert!(
            err.message.contains("`.git`") && err.message.contains("archive"),
            "message should name missing git metadata / archive case: {}",
            err.message
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn validate_repo_contracts_passes_for_self_repo() {
        let report = validate_repo_contracts(&repo_root()).expect("validate self repo");
        assert!(
            !report.has_invalid(),
            "self-repo contracts must validate clean: {report:?}"
        );
        assert!(report.checked >= 1, "expected at least one CONTRACT.yaml");
        assert_eq!(report.invalid, 0);
        assert_eq!(report.valid, report.checked);
    }

    #[test]
    fn compute_fails_closed_when_a_contract_is_invalid() {
        // Build a tiny git repo with a malformed CONTRACT.yaml at the root
        // (missing `kind` and most required fields). The preflight must
        // short-circuit with `CONTRACT_INVALID` before trace/drift run; the
        // returned error must carry the structured ContractReport so the CLI
        // and JSON-RPC surfaces can render the same `contract_validation`
        // payload from one source of truth.
        let base = std::env::temp_dir().join(format!(
            "chassis-gate-bad-contract-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("CONTRACT.yaml"), "name: broken\nkind: library\n").unwrap();
        let _ = git2::Repository::init(&base).unwrap();
        // Init enough git state for `require_git_worktree` (HEAD readable).
        let repo = git2::Repository::open(&base).unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(std::path::Path::new("CONTRACT.yaml")).unwrap();
        idx.write().unwrap();
        let tree_id = idx.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let err =
            compute(&base, Utc::now(), true, false).expect_err("invalid contract must fail closed");
        assert_eq!(err.rule_id, rule_id::CONTRACT_INVALID);
        let report = err
            .contract_report
            .as_ref()
            .expect("CONTRACT_INVALID must carry the structured report");
        assert!(report.has_invalid());
        assert_eq!(report.invalid, 1);
        assert_eq!(report.errors[0].code, CH_RUST_METADATA_CONTRACT);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn compute_require_scanners_rejects_nonzero_summary_errors() {
        use serde_json::json;

        let base = std::env::temp_dir().join(format!(
            "chassis-gate-scan-err-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(base.join("dist")).unwrap();
        let scan = json!({
            "tool": "semgrep",
            "sarifSha256": "a234567890123456789012345678901234567890123456789012345678901234",
            "total": 1,
            "errors": 1,
            "warnings": 0,
            "infos": 0,
            "diagnostics": [{
                "ruleId": "CH-SCANNER-FINDING",
                "severity": "error",
                "message": "blocked",
                "source": "semgrep",
                "subject": "src/x.rs"
            }]
        });
        std::fs::write(
            base.join("dist/scanner-semgrep.json"),
            serde_json::to_string_pretty(&scan).unwrap(),
        )
        .unwrap();
        let mut codeql = scan.clone();
        codeql["tool"] = json!("codeql");
        std::fs::write(
            base.join("dist/scanner-codeql.json"),
            serde_json::to_string_pretty(&codeql).unwrap(),
        )
        .unwrap();

        let contract = std::fs::read_to_string(
            repo_root().join("fixtures/happy-path/rust-minimal/CONTRACT.yaml"),
        )
        .unwrap();
        std::fs::write(base.join("CONTRACT.yaml"), contract).unwrap();

        let _ = git2::Repository::init(&base).unwrap();
        let repo = git2::Repository::open(&base).unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(std::path::Path::new("CONTRACT.yaml")).unwrap();
        idx.add_path(std::path::Path::new("dist/scanner-semgrep.json")).unwrap();
        idx.add_path(std::path::Path::new("dist/scanner-codeql.json")).unwrap();
        idx.write().unwrap();
        let tree_id = idx.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let err = compute(&base, Utc::now(), true, true).expect_err("scanner summary errors");
        assert_eq!(err.rule_id, rule_id::SCANNER_EVIDENCE_REQUIRED);
        let _ = std::fs::remove_dir_all(&base);
    }
}
