//! Spec Kit index (`spec-index.schema.json`): export, validation, linker.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;

use crate::contract::validate_metadata_contract;
use crate::diagnostic::{Diagnostic, Severity, Violated};
use crate::trace::types::TraceGraph;

// @claim chassis.exports-not-policy-engines

static SPEC_INDEX_SCHEMA_STR: &str = include_str!("../../../schemas/spec-index.schema.json");
static SPEC_INDEX_SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let schema: Value =
        serde_json::from_str(SPEC_INDEX_SCHEMA_STR).expect("spec-index.schema.json");
    jsonschema::validator_for(&schema).expect("compile spec-index schema")
});

pub const CH_SPEC_SOURCE_PARSE: &str = "CH-SPEC-SOURCE-PARSE";
pub const CH_SPEC_DUPLICATE_ID: &str = "CH-SPEC-DUPLICATE-ID";
pub const CH_SPEC_ACCEPTANCE_MISSING: &str = "CH-SPEC-ACCEPTANCE-MISSING";
pub const CH_SPEC_INDEX_SCHEMA: &str = "CH-SPEC-INDEX-SCHEMA";
pub const CH_SPEC_UNBOUND_REQUIREMENT: &str = "CH-SPEC-UNBOUND-REQUIREMENT";
pub const CH_SPEC_UNKNOWN_CLAIM_REF: &str = "CH-SPEC-UNKNOWN-CLAIM-REF";
pub const CH_SPEC_INVALID_TASK_EDGE: &str = "CH-SPEC-INVALID-TASK-EDGE";
pub const CH_SPEC_ORPHAN_TASK: &str = "CH-SPEC-ORPHAN-TASK";
pub const CH_SPEC_RELATED_TASK_MISSING: &str = "CH-SPEC-RELATED-TASK-MISSING";
pub const CH_SPEC_DUPLICATE_CLAIM_REF: &str = "CH-SPEC-DUPLICATE-CLAIM-REF";
pub const CH_SPEC_CLAIM_IMPL_MISSING: &str = "CH-SPEC-CLAIM-IMPL-MISSING";
pub const CH_SPEC_CLAIM_TEST_MISSING: &str = "CH-SPEC-CLAIM-TEST-MISSING";
pub const CH_SPEC_TOUCHED_PATH_UNCOVERED: &str = "CH-SPEC-TOUCHED-PATH-UNCOVERED";

/// Markdown `yaml-meta` preset: exactly one non-empty fenced block (ADR-0029).
pub const CH_SPEC_MARKDOWN_NO_FENCE: &str = "CH-SPEC-MARKDOWN-NO-FENCE";
pub const CH_SPEC_MARKDOWN_MULTIPLE_FENCES: &str = "CH-SPEC-MARKDOWN-MULTIPLE-FENCES";
pub const CH_SPEC_MARKDOWN_EMPTY_FENCE: &str = "CH-SPEC-MARKDOWN-EMPTY-FENCE";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecIndex {
    pub version: i32,
    pub chassis_preset_version: i32,
    pub feature_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub constitution_principles: Vec<ConstitutionPrinciple>,
    pub non_goals: Vec<String>,
    pub requirements: Vec<Requirement>,
    pub tasks: Vec<SpecTask>,
    pub implementation_constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstitutionPrinciple {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub acceptance_criteria: Vec<String>,
    pub claim_ids: Vec<String>,
    pub related_task_ids: Vec<String>,
    pub touched_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecTask {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_group: Option<String>,
    pub touched_paths: Vec<String>,
}

/// Validate a JSON value against `schemas/spec-index.schema.json`.
pub fn validate_spec_index_json(value: &Value) -> Result<(), Vec<String>> {
    let errs: Vec<String> = SPEC_INDEX_SCHEMA
        .iter_errors(value)
        .map(|e| e.to_string())
        .collect();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

/// Alias consistent with other artifact validators (`*_value`).
pub fn validate_spec_index_value(value: &Value) -> Result<(), Vec<String>> {
    validate_spec_index_json(value)
}

fn sort_strings(v: &mut [String]) {
    v.sort();
}

/// Canonicalizes the index for stable JSON and digest (sorts arrays).
pub fn canonicalize(mut idx: SpecIndex) -> SpecIndex {
    idx.constitution_principles.sort_by(|a, b| a.id.cmp(&b.id));
    sort_strings(&mut idx.non_goals);
    for r in &mut idx.requirements {
        sort_strings(&mut r.acceptance_criteria);
        sort_strings(&mut r.claim_ids);
        sort_strings(&mut r.related_task_ids);
        sort_strings(&mut r.touched_paths);
    }
    idx.requirements.sort_by(|a, b| a.id.cmp(&b.id));
    for t in &mut idx.tasks {
        sort_strings(&mut t.depends_on);
        sort_strings(&mut t.touched_paths);
    }
    idx.tasks.sort_by(|a, b| a.id.cmp(&b.id));
    sort_strings(&mut idx.implementation_constraints);
    idx
}

/// Lowercase hex SHA-256 of canonical UTF-8 JSON for `spec_index`.
pub fn digest_sha256_hex(idx: &SpecIndex) -> Result<String, String> {
    let c = canonicalize(idx.clone());
    let v = serde_json::to_vec(&c).map_err(|e| format!("serialize spec index: {e}"))?;
    Ok(format!("{:x}", Sha256::digest(v)))
}

#[derive(Debug)]
pub struct ExportError {
    pub rule_id: &'static str,
    pub message: String,
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.rule_id, self.message)
    }
}

impl std::error::Error for ExportError {}

/// Parse YAML source, validate semantic rules, canonicalize, validate JSON Schema match.
pub fn export_from_source_yaml_bytes(raw: &[u8]) -> Result<SpecIndex, ExportError> {
    let idx: SpecIndex = serde_yaml::from_slice(raw).map_err(|e| ExportError {
        rule_id: CH_SPEC_SOURCE_PARSE,
        message: format!("yaml parse: {e}"),
    })?;
    validate_source_semantics(&idx)?;
    let idx = canonicalize(idx);
    let v = serde_json::to_value(&idx).map_err(|e| ExportError {
        rule_id: CH_SPEC_SOURCE_PARSE,
        message: format!("serialize: {e}"),
    })?;
    validate_spec_index_json(&v).map_err(|errs| ExportError {
        rule_id: CH_SPEC_INDEX_SCHEMA,
        message: errs.join("; "),
    })?;
    Ok(idx)
}

/// Parse YAML file.
pub fn export_from_source_yaml_path(path: &Path) -> Result<SpecIndex, ExportError> {
    let raw = fs::read(path).map_err(|e| ExportError {
        rule_id: CH_SPEC_SOURCE_PARSE,
        message: format!("read {}: {e}", path.display()),
    })?;
    export_from_source_yaml_bytes(&raw)
}

fn validate_source_semantics(idx: &SpecIndex) -> Result<(), ExportError> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for p in &idx.constitution_principles {
        if !seen.insert(p.id.clone()) {
            return Err(ExportError {
                rule_id: CH_SPEC_DUPLICATE_ID,
                message: format!("duplicate constitution principle id `{}`", p.id),
            });
        }
    }
    seen.clear();
    for r in &idx.requirements {
        if !seen.insert(r.id.clone()) {
            return Err(ExportError {
                rule_id: CH_SPEC_DUPLICATE_ID,
                message: format!("duplicate requirement id `{}`", r.id),
            });
        }
        if r.acceptance_criteria.is_empty() {
            return Err(ExportError {
                rule_id: CH_SPEC_ACCEPTANCE_MISSING,
                message: format!("requirement `{}` has empty acceptance_criteria", r.id),
            });
        }
    }
    seen.clear();
    for t in &idx.tasks {
        if !seen.insert(t.id.clone()) {
            return Err(ExportError {
                rule_id: CH_SPEC_DUPLICATE_ID,
                message: format!("duplicate task id `{}`", t.id),
            });
        }
    }

    let task_ids: BTreeSet<String> = idx.tasks.iter().map(|t| t.id.clone()).collect();
    for t in &idx.tasks {
        for d in &t.depends_on {
            if !task_ids.contains(d) {
                return Err(ExportError {
                    rule_id: CH_SPEC_INVALID_TASK_EDGE,
                    message: format!("task `{}` depends_on references unknown task `{}`", t.id, d),
                });
            }
            if d == &t.id {
                return Err(ExportError {
                    rule_id: CH_SPEC_INVALID_TASK_EDGE,
                    message: format!("task `{}` cannot depend_on itself", t.id),
                });
            }
        }
    }

    Ok(())
}

fn discover_contract_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
        for ent in entries {
            let ent = ent.map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
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

/// Collect every invariant and edge_case claim id from CONTRACT.yaml files under `root`.
pub fn collect_contract_claim_ids(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for path in discover_contract_paths(root)? {
        let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let yaml = serde_yaml::from_str::<serde_yaml::Value>(&raw)
            .map_err(|e| format!("yaml {}: {e}", path.display()))?;
        let value =
            serde_json::to_value(yaml).map_err(|e| format!("yaml→json {}: {e}", path.display()))?;
        validate_metadata_contract(&value).map_err(|e| {
            format!(
                "contract {} invalid before claim extraction: {}",
                path.display(),
                e.join("; ")
            )
        })?;
        if let Some(inv) = value.get("invariants").and_then(|x| x.as_array()) {
            for c in inv {
                if let Some(id) = c.get("id").and_then(|x| x.as_str()) {
                    ids.insert(id.to_string());
                }
            }
        }
        if let Some(ec) = value.get("edge_cases").and_then(|x| x.as_array()) {
            for c in ec {
                if let Some(id) = c.get("id").and_then(|x| x.as_str()) {
                    ids.insert(id.to_string());
                }
            }
        }
    }
    Ok(ids)
}

fn adr_violated() -> Option<Violated> {
    Some(Violated {
        convention: "ADR-0026".to_string(),
    })
}

fn normalize_rel_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn normalize_spec_user_path(s: &str) -> String {
    let t = s.trim();
    let t = t.trim_start_matches("./");
    t.replace('\\', "/")
}

fn collect_trace_site_paths_for_claims<'a, I>(trace: &TraceGraph, claim_ids: I) -> BTreeSet<String>
where
    I: IntoIterator<Item = &'a String>,
{
    let mut s = BTreeSet::new();
    for cid in claim_ids {
        if let Some(n) = trace.claims.get(cid) {
            for site in &n.impl_sites {
                s.insert(normalize_rel_path(&site.file));
            }
            for site in &n.test_sites {
                s.insert(normalize_rel_path(&site.file));
            }
        }
    }
    s
}

/// Repo-relative paths listed in CONTRACT.yaml `exports[].path` entries.
fn collect_contract_export_paths(repo_root: &Path) -> Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();
    for path in discover_contract_paths(repo_root)? {
        let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let yaml = serde_yaml::from_str::<serde_yaml::Value>(&raw)
            .map_err(|e| format!("yaml {}: {e}", path.display()))?;
        let value =
            serde_json::to_value(yaml).map_err(|e| format!("yaml→json {}: {e}", path.display()))?;
        validate_metadata_contract(&value).map_err(|e| {
            format!(
                "contract {} invalid before export path extraction: {}",
                path.display(),
                e.join("; ")
            )
        })?;

        let rel = path
            .strip_prefix(repo_root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.clone());
        let base = rel.parent().unwrap_or_else(|| Path::new(""));

        if let Some(exports) = value.get("exports").and_then(|x| x.as_array()) {
            for export in exports {
                if let Some(export_path) = export.get("path").and_then(|x| x.as_str()) {
                    let export_rel = Path::new(export_path);
                    let joined = if export_rel.is_absolute() {
                        export_rel.to_path_buf()
                    } else {
                        base.join(export_rel)
                    };
                    paths.insert(normalize_rel_path(&joined));
                }
            }
        }
    }
    Ok(paths)
}

fn path_is_covered_by_contract_export_or_trace(
    touched_norm: &str,
    site_paths: &BTreeSet<String>,
    contract_export_paths: &BTreeSet<String>,
) -> bool {
    if touched_norm.is_empty() {
        return true;
    }
    if site_paths.contains(touched_norm) {
        return true;
    }
    contract_export_paths.contains(touched_norm)
}

/// Map spec requirements/tasks to contracts and trace evidence; emit diagnostics (errors for blocking gaps).
pub fn link_spec_index(spec: &SpecIndex, repo_root: &Path, trace: &TraceGraph) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let known = match collect_contract_claim_ids(repo_root) {
        Ok(k) => k,
        Err(msg) => {
            out.push(Diagnostic {
                rule_id: CH_SPEC_SOURCE_PARSE.to_string(),
                severity: Severity::Error,
                message: format!("cannot collect contract claim ids: {msg}"),
                source: Some("spec-index-link".to_string()),
                subject: None,
                violated: adr_violated(),
                docs: None,
                fix: None,
                location: None,
                detail: None,
            });
            return out;
        }
    };

    let contract_export_paths = match collect_contract_export_paths(repo_root) {
        Ok(x) => x,
        Err(msg) => {
            out.push(Diagnostic {
                rule_id: CH_SPEC_SOURCE_PARSE.to_string(),
                severity: Severity::Error,
                message: format!("cannot collect contract export paths: {msg}"),
                source: Some("spec-index-link".to_string()),
                subject: None,
                violated: adr_violated(),
                docs: None,
                fix: None,
                location: None,
                detail: None,
            });
            return out;
        }
    };

    let task_id_set: BTreeSet<String> = spec.tasks.iter().map(|t| t.id.clone()).collect();
    let mut task_claim_ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for req in &spec.requirements {
        for tid in &req.related_task_ids {
            let ids = task_claim_ids.entry(tid.clone()).or_default();
            ids.extend(req.claim_ids.iter().cloned());
        }
    }

    for req in &spec.requirements {
        if req.claim_ids.is_empty() {
            out.push(Diagnostic {
                rule_id: CH_SPEC_UNBOUND_REQUIREMENT.to_string(),
                severity: Severity::Error,
                message: format!(
                    "requirement `{}` has empty claim_ids (bind at least one CONTRACT.yaml claim)",
                    req.id
                ),
                source: Some("spec-index-link".to_string()),
                subject: Some(req.id.clone()),
                violated: adr_violated(),
                docs: None,
                fix: None,
                location: None,
                detail: None,
            });
            continue;
        }

        for tid in &req.related_task_ids {
            if !task_id_set.contains(tid) {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_RELATED_TASK_MISSING.to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "requirement `{}` related_task_ids references unknown task `{}`",
                        req.id, tid
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(format!("{}:{}", req.id, tid)),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
            }
        }

        let mut seen_claim: BTreeSet<String> = BTreeSet::new();
        for cid in &req.claim_ids {
            if !seen_claim.insert(cid.clone()) {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_DUPLICATE_CLAIM_REF.to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "requirement `{}` lists claim_id `{}` more than once",
                        req.id, cid
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(format!("{}:{}", req.id, cid)),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
                continue;
            }
            if !known.contains(cid) {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_UNKNOWN_CLAIM_REF.to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "requirement `{}` references unknown claim_id `{}` (not in any CONTRACT.yaml)",
                        req.id, cid
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(format!("{}:{}", req.id, cid)),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
                continue;
            }
            let Some(node) = trace.claims.get(cid) else {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_UNKNOWN_CLAIM_REF.to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "requirement `{}` claim_id `{}` is declared in CONTRACT.yaml but missing from the trace graph",
                        req.id, cid
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(format!("{}:{}", req.id, cid)),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
                continue;
            };
            if node.impl_sites.is_empty() {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_CLAIM_IMPL_MISSING.to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "requirement `{}` claim_id `{}` has no implementation sites in the trace graph",
                        req.id, cid
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(format!("{}:{}", req.id, cid)),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
            }
            if node.test_sites.is_empty() {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_CLAIM_TEST_MISSING.to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "requirement `{}` claim_id `{}` has no test sites in the trace graph",
                        req.id, cid
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(format!("{}:{}", req.id, cid)),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
            }
        }

        let site_paths = collect_trace_site_paths_for_claims(trace, &req.claim_ids);
        for tp in &req.touched_paths {
            let n = normalize_spec_user_path(tp);
            if !path_is_covered_by_contract_export_or_trace(&n, &site_paths, &contract_export_paths)
            {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_TOUCHED_PATH_UNCOVERED.to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "requirement `{}` touched_path `{}` is not listed by a contract export path and does not appear in trace impl/test sites",
                        req.id, tp
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(format!("{}:{}", req.id, tp)),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
            }
        }
    }

    for t in &spec.tasks {
        for d in &t.depends_on {
            if !task_id_set.contains(d) {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_INVALID_TASK_EDGE.to_string(),
                    severity: Severity::Error,
                    message: format!("task `{}` depends_on unknown task `{}`", t.id, d),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(t.id.clone()),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
            }
        }
        let site_paths = task_claim_ids
            .get(&t.id)
            .map(|ids| collect_trace_site_paths_for_claims(trace, ids))
            .unwrap_or_default();
        for tp in &t.touched_paths {
            let n = normalize_spec_user_path(tp);
            if !path_is_covered_by_contract_export_or_trace(&n, &site_paths, &contract_export_paths)
            {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_TOUCHED_PATH_UNCOVERED.to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "task `{}` touched_path `{}` is not listed by a contract export path and does not appear in trace impl/test sites",
                        t.id, tp
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(format!("{}:{}", t.id, tp)),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
            }
        }
    }

    let mut covered: BTreeSet<String> = BTreeSet::new();
    for r in &spec.requirements {
        for t in &r.related_task_ids {
            covered.insert(t.clone());
        }
    }

    if !spec.tasks.is_empty() {
        for t in &spec.tasks {
            if !covered.contains(&t.id) {
                out.push(Diagnostic {
                    rule_id: CH_SPEC_ORPHAN_TASK.to_string(),
                    severity: Severity::Info,
                    message: format!(
                        "task `{}` is not listed in any requirement.related_task_ids",
                        t.id
                    ),
                    source: Some("spec-index-link".to_string()),
                    subject: Some(t.id.clone()),
                    violated: adr_violated(),
                    docs: None,
                    fix: None,
                    location: None,
                    detail: None,
                });
            }
        }
    }

    out.sort_by(|a, b| {
        a.rule_id
            .cmp(&b.rule_id)
            .then_with(|| a.message.cmp(&b.message))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_yaml() -> &'static [u8] {
        br#"
version: 1
chassis_preset_version: 1
feature_id: demo-bridge
title: Demo
constitution_principles:
  - id: P1
    text: Keep it deterministic
non_goals: []
requirements:
  - id: REQ-001
    title: r1
    description: desc
    acceptance_criteria:
      - done
    claim_ids:
      - demo.claim
    related_task_ids:
      - TASK-001
    touched_paths:
      - src/lib.rs
tasks:
  - id: TASK-001
    title: t1
    depends_on: []
    touched_paths:
      - src/lib.rs
implementation_constraints: []
"#
    }

    #[test]
    fn export_round_trip_validates_schema() {
        let idx = export_from_source_yaml_bytes(sample_yaml()).expect("export");
        let v = serde_json::to_value(&idx).unwrap();
        validate_spec_index_json(&v).expect("schema ok");
        digest_sha256_hex(&idx).expect("digest");
    }

    #[test]
    fn duplicate_requirement_ids_rejected() {
        let bad = br#"
version: 1
chassis_preset_version: 1
feature_id: x
constitution_principles: []
non_goals: []
requirements:
  - id: REQ-001
    title: a
    description: d
    acceptance_criteria: ["x"]
    claim_ids: ["a"]
    related_task_ids: []
    touched_paths: []
  - id: REQ-001
    title: b
    description: d
    acceptance_criteria: ["y"]
    claim_ids: ["b"]
    related_task_ids: []
    touched_paths: []
tasks: []
implementation_constraints: []
"#;
        let e = export_from_source_yaml_bytes(bad).expect_err("dup");
        assert_eq!(e.rule_id, CH_SPEC_DUPLICATE_ID);
    }

    #[test]
    fn related_task_missing_emits_diagnostic() {
        use crate::trace::types::TraceGraph;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let spec = SpecIndex {
            version: 1,
            chassis_preset_version: 1,
            feature_id: "x".into(),
            title: None,
            summary: None,
            constitution_principles: vec![ConstitutionPrinciple {
                id: "P1".into(),
                text: "p".into(),
            }],
            non_goals: vec![],
            requirements: vec![Requirement {
                id: "REQ-001".into(),
                title: "r".into(),
                description: "d".into(),
                acceptance_criteria: vec!["a".into()],
                claim_ids: vec!["not.in.any.contract".into()],
                related_task_ids: vec!["TASK-MISSING".into()],
                touched_paths: vec![],
            }],
            tasks: vec![],
            implementation_constraints: vec![],
        };
        let trace = TraceGraph {
            claims: Default::default(),
            orphan_sites: vec![],
            diagnostics: vec![],
        };
        let diags = link_spec_index(&spec, &root, &trace);
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == CH_SPEC_RELATED_TASK_MISSING),
            "expected related-task diagnostic, got {diags:?}"
        );
    }
}
