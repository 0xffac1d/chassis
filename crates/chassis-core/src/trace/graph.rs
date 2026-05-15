#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;

use crate::contract::{validate_metadata_contract, Contract};
use crate::diagnostic::{Diagnostic, Severity, Violated};
use crate::exempt::{self, Registry as ExemptionRegistry};
use crate::trace::extract::{rust::extract_rust, typescript::extract_typescript};
use crate::trace::types::{ClaimContractKind, ClaimNode, SiteKind, TraceError, TraceGraph};

pub const RULE_NOT_IN_CONTRACT: &str = "CH-TRACE-CLAIM-NOT-IN-CONTRACT";

#[inline]
fn contract_base(contract: &Contract) -> (&[crate::contract::Claim], &[crate::contract::Claim]) {
    match contract {
        Contract::Library(x) => (&x.base.invariants, &x.base.edge_cases),
        Contract::Cli(x) => (&x.base.invariants, &x.base.edge_cases),
        Contract::Component(x) => (&x.base.invariants, &x.base.edge_cases),
        Contract::Endpoint(x) => (&x.base.invariants, &x.base.edge_cases),
        Contract::Entity(x) => (&x.base.invariants, &x.base.edge_cases),
        Contract::Service(x) => (&x.base.invariants, &x.base.edge_cases),
        Contract::EventStream(x) => (&x.base.invariants, &x.base.edge_cases),
        Contract::FeatureFlag(x) => (&x.base.invariants, &x.base.edge_cases),
    }
}

fn discover_contracts(root: &Path) -> Result<Vec<PathBuf>, TraceError> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), TraceError> {
        for ent in fs::read_dir(dir).map_err(TraceError::Io)? {
            let ent = ent.map_err(TraceError::Io)?;
            let p = ent.path();
            if p.is_dir() {
                if matches!(
                    p.file_name().and_then(|n| n.to_str()),
                    Some("target") | Some("node_modules") | Some(".git")
                ) {
                    continue;
                }
                if matches!(
                    p.file_name().and_then(|n| n.to_str()),
                    Some("fixtures") | Some("reference")
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

    let mut paths = Vec::new();
    walk(root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn load_contract(path: &Path) -> Result<Contract, TraceError> {
    let raw = fs::read_to_string(path).map_err(TraceError::Io)?;
    let v: Value = serde_yaml::from_str(&raw).map_err(|e| {
        TraceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("yaml {e}"),
        ))
    })?;
    validate_metadata_contract(&v).map_err(|errs| {
        TraceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            errs.join("; "),
        ))
    })?;
    serde_json::from_value(v).map_err(|e| {
        TraceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("contract json {e}"),
        ))
    })
}

fn load_exemptions(root: &Path) -> Option<ExemptionRegistry> {
    let path = root.join(".chassis/exemptions.yaml");
    let raw = fs::read_to_string(path).ok()?;
    let y: serde_yaml::Value = serde_yaml::from_str(&raw).ok()?;
    let j = serde_json::to_value(y).ok()?;
    serde_json::from_value(j).ok()
}

fn extract_frontmatter_block(raw: &str) -> Option<&str> {
    let body = raw.strip_prefix("---\n")?;
    let end = body.find("\n---\n")?;
    Some(&body[..end])
}

fn strip_frontmatter_body(raw: &str) -> String {
    if raw.starts_with("---\n") {
        if let Some(i) = raw.find("\n---\n") {
            let tail = &raw[i + "\n---\n".len()..];
            return tail.to_string();
        }
    }
    raw.to_string()
}

fn adr_id_from_frontmatter(raw: &str) -> Option<String> {
    let fm = extract_frontmatter_block(raw)?;
    for line in fm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("id:") {
            return Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

fn extract_adr_refs(root: &Path, ids: &BTreeSet<String>) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for id in ids {
        out.insert(id.clone(), Vec::new());
    }

    let adr_dir = root.join("docs/adr");
    let Ok(rd) = fs::read_dir(&adr_dir) else {
        return out;
    };

    for ent in rd.flatten() {
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&p) else {
            continue;
        };
        let Some(adr_id) = adr_id_from_frontmatter(&text) else {
            continue;
        };
        let body = strip_frontmatter_body(&text);
        for cid in ids {
            if body.contains(cid.as_str()) {
                let ent = out.get_mut(cid).expect("key exists");
                if !ent.contains(&adr_id) {
                    ent.push(adr_id.clone());
                }
            }
        }
    }
    out
}

fn attach_exemptions(
    reg: Option<&ExemptionRegistry>,
    cid: &str,
    active_at: NaiveDate,
    node: &mut ClaimNode,
) {
    let Some(reg) = reg else {
        return;
    };

    let active = exempt::list(
        reg,
        exempt::ListFilter {
            rule_id: None,
            path: None,
            active_at: Some(active_at),
        },
    );
    for entry in active {
        let hit = entry
            .finding_id
            .as_deref()
            .map(|s| s.contains(cid))
            .unwrap_or(false)
            || entry.reason.contains(cid);
        if hit && !node.active_exemptions.contains(&entry.id) {
            node.active_exemptions.push(entry.id.clone());
        }
    }
}

/// Build repo-wide trace metadata for every CONTRACT claim plus extracted sites.
pub fn build_trace_graph(root: &Path) -> Result<TraceGraph, TraceError> {
    build_trace_graph_at(root, Utc::now())
}

pub fn build_trace_graph_at(root: &Path, now: DateTime<Utc>) -> Result<TraceGraph, TraceError> {
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    let (mut rust_sites, rust_diags) = extract_rust(&root);
    let (mut ts_sites, ts_diags) = extract_typescript(&root);
    rust_sites.append(&mut ts_sites);
    let mut diag_all = rust_diags;
    diag_all.extend(ts_diags);

    let contracts = discover_contracts(&root)?;
    let exemptions = load_exemptions(&root);
    let today = now.date_naive();

    let mut contract_ids = BTreeSet::<String>::new();
    let mut nodes: BTreeMap<String, ClaimNode> = BTreeMap::new();

    for cp in contracts {
        let c = load_contract(&cp)?;
        let rel = cp
            .strip_prefix(&root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| cp.clone());
        let (invs, ecs) = contract_base(&c);

        for cl in invs {
            contract_ids.insert(cl.id.clone());
            nodes.entry(cl.id.clone()).or_insert_with(|| ClaimNode {
                claim_id: cl.id.clone(),
                contract_path: rel.clone(),
                contract_kind: ClaimContractKind::Invariant,
                claim_record: cl.clone(),
                impl_sites: Vec::new(),
                test_sites: Vec::new(),
                adr_refs: Vec::new(),
                active_exemptions: Vec::new(),
            });
        }

        for cl in ecs {
            contract_ids.insert(cl.id.clone());
            use std::collections::btree_map::Entry;
            match nodes.entry(cl.id.clone()) {
                Entry::Vacant(v) => {
                    v.insert(ClaimNode {
                        claim_id: cl.id.clone(),
                        contract_path: rel.clone(),
                        contract_kind: ClaimContractKind::EdgeCase,
                        claim_record: cl.clone(),
                        impl_sites: Vec::new(),
                        test_sites: Vec::new(),
                        adr_refs: Vec::new(),
                        active_exemptions: Vec::new(),
                    });
                }
                Entry::Occupied(mut o) => {
                    let n = o.get_mut();
                    if matches!(n.contract_kind, ClaimContractKind::Invariant) {
                        continue;
                    }
                    n.contract_path.clone_from(&rel);
                    n.claim_record = cl.clone();
                }
            }
        }
    }

    let adr_map = extract_adr_refs(&root, &contract_ids);

    let mut orphans = Vec::new();

    for site in &rust_sites {
        if contract_ids.contains(&site.claim_id) {
            if let Some(node) = nodes.get_mut(&site.claim_id) {
                match site.kind {
                    SiteKind::Impl => node.impl_sites.push(site.clone()),
                    SiteKind::Test => node.test_sites.push(site.clone()),
                }
            }
        } else {
            orphans.push(site.clone());
            diag_all.push(Diagnostic {
                rule_id: RULE_NOT_IN_CONTRACT.into(),
                severity: Severity::Warning,
                message: format!(
                    "claim `{}` not declared on any CONTRACT.yaml",
                    site.claim_id
                ),
                source: Some("trace::graph".into()),
                subject: Some(format!("{}:{}", site.file.display(), site.line)),
                violated: Some(Violated {
                    convention: "ADR-0023".to_string(),
                }),
                docs: None,
                fix: None,
                location: None,
                detail: None,
            });
        }
    }

    for (cid, node) in nodes.iter_mut() {
        if let Some(v) = adr_map.get(cid) {
            node.adr_refs.clone_from(v);
        }
        attach_exemptions(exemptions.as_ref(), cid, today, node);
    }

    Ok(TraceGraph {
        claims: nodes,
        orphan_sites: orphans,
        diagnostics: diag_all,
    })
}
