#![forbid(unsafe_code)]

use std::collections::hash_map::{Entry, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use std::sync::LazyLock;
use tree_sitter::{Node, Parser};

use crate::diagnostic::{Diagnostic, Severity, Violated};
use crate::trace::backend::TraceExtractBackend;
use crate::trace::types::{ClaimSite, SiteKind};

pub const RULE_MALFORMED: &str = "CH-TRACE-MALFORMED-CLAIM";
pub const RULE_DUPLICATE_SITE: &str = "CH-TRACE-DUPLICATE-CLAIM-AT-SITE";
pub const RULE_PARSE_ERROR: &str = "CH-TRACE-PARSE-ERROR";

static CLAIM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*//\s*@claim\s+([^\s]+)\s*$").expect("claim regex"));

static CLAIM_ID_OK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_.-]*$").expect("claim id grammar per STABLE-IDS"));

fn diag(rule: &str, sev: Severity, msg: String, subject: String) -> Diagnostic {
    Diagnostic {
        rule_id: rule.to_string(),
        severity: sev,
        message: msg,
        source: Some("trace::extract::rust".to_string()),
        subject: Some(subject),
        violated: Some(Violated {
            convention: if rule == RULE_PARSE_ERROR {
                "ADR-0028".to_string()
            } else {
                "ADR-0023".to_string()
            },
        }),
        docs: None,
        fix: None,
        location: None,
        detail: None,
    }
}

#[inline]
fn scan_for_test_macro(lines: &[String], upto: usize) -> bool {
    let start = upto.saturating_sub(8);
    lines
        .iter()
        .take(upto.saturating_sub(0))
        .skip(start)
        .any(|l| l.contains("#[test]") || l.contains("#[tokio::test]") || l.contains("#[rstest]"))
}

/// First code line index (0-based) after `@claim` block, skipping blanks / comments / attrs.
fn find_back_site_idx(lines: &[String], mut pos: usize) -> Option<usize> {
    while pos < lines.len() {
        let t = lines[pos].trim();
        if t.is_empty() || t.starts_with("//") {
            pos += 1;
            continue;
        }
        if t.starts_with("#[derive")
            || t.starts_with("#[allow")
            || t.starts_with("#[cfg")
            || t.starts_with("#[instrument")
        {
            pos += 1;
            continue;
        }
        if t.starts_with("#[") || t.starts_with("#![") || t.starts_with("#!") {
            pos += 1;
            continue;
        }
        return Some(pos);
    }
    None
}

pub fn scan_rust_source(rel: &Path, lines: &[String]) -> (Vec<ClaimSite>, Vec<Diagnostic>) {
    let mut sites = Vec::new();
    let mut diags = Vec::new();
    let mut seen_at_site: HashMap<(usize, String), usize> = HashMap::new();
    let mut raw_string_end: Option<String> = None;

    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some(end) = raw_string_end.as_deref() {
            if lines[idx].contains(end) {
                raw_string_end = None;
            }
            idx += 1;
            continue;
        }
        if let Some(end) = raw_string_end_delimiter(&lines[idx]) {
            raw_string_end = Some(end);
            idx += 1;
            continue;
        }

        if !CLAIM_RE.is_match(&lines[idx]) {
            idx += 1;
            continue;
        }

        let first_claim_line = idx + 1;
        let mut claim_ids = Vec::<String>::new();
        while idx < lines.len() {
            let l = &lines[idx];
            if let Some(cap) = CLAIM_RE.captures(l) {
                let raw = cap.get(1).unwrap().as_str().to_string();
                if CLAIM_ID_OK.is_match(&raw) {
                    claim_ids.push(raw);
                } else {
                    diags.push(diag(
                        RULE_MALFORMED,
                        Severity::Error,
                        format!("claim id `{raw}` violates STABLE-IDS grammar"),
                        format!("{}:{}", rel.display(), idx + 1),
                    ));
                }
                idx += 1;
            } else {
                break;
            }
        }

        let Some(site_ix) = find_back_site_idx(lines, idx) else {
            continue;
        };

        let is_test = lines[site_ix].contains("#[test]") || scan_for_test_macro(lines, site_ix);
        let kind = if is_test {
            SiteKind::Test
        } else {
            SiteKind::Impl
        };

        let site_ln = site_ix + 1;
        for cid in &claim_ids {
            let key = (site_ln, cid.clone());
            match seen_at_site.entry(key.clone()) {
                Entry::Occupied(_) => {
                    diags.push(diag(
                        RULE_DUPLICATE_SITE,
                        Severity::Info,
                        format!("duplicate @claim `{cid}` anchored at the same site"),
                        format!("{}:{}", rel.display(), site_ln),
                    ));
                }
                Entry::Vacant(v) => {
                    v.insert(1);
                    sites.push(ClaimSite {
                        file: rel.to_path_buf(),
                        line: first_claim_line.max(1),
                        claim_id: cid.clone(),
                        kind,
                    });
                }
            }
        }

        idx = site_ix + 1;
    }

    (sites, diags)
}

fn raw_string_end_delimiter(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] != b'r' {
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < bytes.len() && bytes[j] == b'#' {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'"' {
            i += 1;
            continue;
        }

        let hashes = j - (i + 1);
        let end = format!("\"{}", "#".repeat(hashes));
        if line[j + 1..].contains(&end) {
            return None;
        }
        return Some(end);
    }
    None
}

/// Walk `crates/**/*.rs` using `backend` (regex vs tree-sitter).
pub fn extract_rust(
    root: &Path,
    backend: TraceExtractBackend,
) -> (Vec<ClaimSite>, Vec<Diagnostic>) {
    match backend {
        TraceExtractBackend::Regex => extract_rust_regex(root),
        TraceExtractBackend::TreeSitter => extract_rust_treesitter(root),
    }
}

/// Walk `crates/**/*.rs` (regex backend; default for `build_trace_graph`).
pub fn extract_rust_regex(root: &Path) -> (Vec<ClaimSite>, Vec<Diagnostic>) {
    let crates = root.join("crates");
    let mut sites = Vec::new();
    let mut diags = Vec::new();
    if !crates.is_dir() {
        return (sites, diags);
    }
    walk_rs(&crates, root, &mut sites, &mut diags);
    (sites, diags)
}

fn extract_rust_treesitter(root: &Path) -> (Vec<ClaimSite>, Vec<Diagnostic>) {
    let crates = root.join("crates");
    let mut sites = Vec::new();
    let mut diags = Vec::new();
    if !crates.is_dir() {
        return (sites, diags);
    }
    walk_rs_ts(&crates, root, &mut sites, &mut diags);
    (sites, diags)
}

fn collect_rust_line_comment_rows(node: Node<'_>, rows: &mut Vec<usize>) {
    let mut c = node.walk();
    if !c.goto_first_child() {
        return;
    }
    loop {
        let n = c.node();
        if n.kind() == "line_comment" {
            rows.push(n.start_position().row);
        } else {
            collect_rust_line_comment_rows(n, rows);
        }
        if !c.goto_next_sibling() {
            break;
        }
    }
}

fn walk_rs_ts(dir: &Path, root: &Path, sites: &mut Vec<ClaimSite>, diags: &mut Vec<Diagnostic>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            diags.push(diag(
                RULE_MALFORMED,
                Severity::Error,
                format!("cannot read `{}`: {}", dir.display(), e),
                dir.display().to_string(),
            ));
            return;
        }
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            if p.file_name().and_then(|n| n.to_str()) == Some("target") {
                continue;
            }
            walk_rs_ts(&p, root, sites, diags);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            let raw = match fs::read_to_string(&p) {
                Ok(s) => s,
                Err(e) => {
                    diags.push(diag(
                        RULE_MALFORMED,
                        Severity::Error,
                        format!("read failed: {e}"),
                        p.display().to_string(),
                    ));
                    continue;
                }
            };
            let rel = normalize_rel(root, &p);
            let ls: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
            if skip_generated_rust(&ls) {
                continue;
            }
            let mut parser = Parser::new();
            if let Err(e) = parser.set_language(&tree_sitter_rust::language()) {
                diags.push(diag(
                    RULE_MALFORMED,
                    Severity::Error,
                    format!("tree-sitter rust grammar: {e}"),
                    rel.display().to_string(),
                ));
                continue;
            }
            let Some(tree) = parser.parse(&raw, None) else {
                diags.push(diag(
                    RULE_PARSE_ERROR,
                    Severity::Warning,
                    "tree-sitter parse returned no tree".into(),
                    rel.display().to_string(),
                ));
                continue;
            };
            let root_node = tree.root_node();
            if root_node.has_error() {
                diags.push(diag(
                    RULE_PARSE_ERROR,
                    Severity::Warning,
                    "tree-sitter parse contains error nodes — extraction may be incomplete".into(),
                    rel.display().to_string(),
                ));
            }
            let mut rows = Vec::new();
            collect_rust_line_comment_rows(root_node, &mut rows);
            rows.sort_unstable();
            rows.dedup();
            let entry_set: std::collections::BTreeSet<usize> = rows.iter().copied().collect();
            let (mut s, mut d) = scan_rust_source_ts(&rel, &ls, &entry_set);
            sites.append(&mut s);
            diags.append(&mut d);
        }
    }
}

/// Only start `@claim` blocks on lines that tree-sitter classified as `line_comment`.
fn scan_rust_source_ts(
    rel: &Path,
    lines: &[String],
    comment_rows: &std::collections::BTreeSet<usize>,
) -> (Vec<ClaimSite>, Vec<Diagnostic>) {
    use std::collections::hash_map::{Entry, HashMap};

    let mut sites = Vec::new();
    let mut diags = Vec::new();
    let mut seen_at_site: HashMap<(usize, String), usize> = HashMap::new();

    let mut k = 0usize;
    while k < lines.len() {
        if !comment_rows.contains(&k) || !CLAIM_RE.is_match(lines[k].trim()) {
            k += 1;
            continue;
        }

        let first_claim_line = k + 1;
        let mut claim_ids = Vec::<String>::new();
        let mut idx = k;
        while idx < lines.len() {
            let l = &lines[idx];
            if let Some(cap) = CLAIM_RE.captures(l.trim()) {
                let raw = cap.get(1).unwrap().as_str().to_string();
                if CLAIM_ID_OK.is_match(&raw) {
                    claim_ids.push(raw);
                } else {
                    diags.push(diag(
                        RULE_MALFORMED,
                        Severity::Error,
                        format!("claim id `{raw}` violates STABLE-IDS grammar"),
                        format!("{}:{}", rel.display(), idx + 1),
                    ));
                }
                idx += 1;
            } else {
                break;
            }
        }

        let Some(site_ix) = find_back_site_idx(lines, idx) else {
            k = idx.max(k + 1);
            continue;
        };

        let is_test = lines[site_ix].contains("#[test]") || scan_for_test_macro(lines, site_ix);
        let kind = if is_test {
            SiteKind::Test
        } else {
            SiteKind::Impl
        };

        let site_ln = site_ix + 1;
        for cid in &claim_ids {
            let key = (site_ln, cid.clone());
            match seen_at_site.entry(key.clone()) {
                Entry::Occupied(_) => {
                    diags.push(diag(
                        RULE_DUPLICATE_SITE,
                        Severity::Info,
                        format!("duplicate @claim `{cid}` anchored at the same site"),
                        format!("{}:{}", rel.display(), site_ln),
                    ));
                }
                Entry::Vacant(v) => {
                    v.insert(1);
                    sites.push(ClaimSite {
                        file: rel.to_path_buf(),
                        line: first_claim_line.max(1),
                        claim_id: cid.clone(),
                        kind,
                    });
                }
            }
        }

        k = site_ix + 1;
    }

    (sites, diags)
}

fn skip_generated_rust(lines: &[String]) -> bool {
    let mut i = 0usize;
    if let Some(f) = lines.first() {
        if f.starts_with("#!") {
            i = 1;
        }
    }
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }
    if i >= lines.len() {
        return false;
    }
    let t = lines[i].trim();
    t.contains("@generated") || t.contains("// @generated by")
}

fn walk_rs(dir: &Path, root: &Path, sites: &mut Vec<ClaimSite>, diags: &mut Vec<Diagnostic>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            diags.push(diag(
                RULE_MALFORMED,
                Severity::Error,
                format!("cannot read `{}`: {}", dir.display(), e),
                dir.display().to_string(),
            ));
            return;
        }
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            if p.file_name().and_then(|n| n.to_str()) == Some("target") {
                continue;
            }
            walk_rs(&p, root, sites, diags);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            let raw = match fs::read_to_string(&p) {
                Ok(s) => s,
                Err(e) => {
                    diags.push(diag(
                        RULE_MALFORMED,
                        Severity::Error,
                        format!("read failed: {e}"),
                        p.display().to_string(),
                    ));
                    continue;
                }
            };
            let rel = normalize_rel(root, &p);
            let ls: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
            let (mut s, mut d) = scan_rust_source(&rel, &ls);
            sites.append(&mut s);
            diags.append(&mut d);
        }
    }
}

pub fn normalize_rel(root: &Path, abs: &Path) -> PathBuf {
    abs.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| abs.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ls(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    #[test]
    fn happy_path_claim() {
        let src = r##"
pub mod x {
// @claim a.b
pub fn demo() {}
}
"##;
        let (sites, _) = scan_rust_source(Path::new("demo.rs"), &ls(src));
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].claim_id, "a.b");
    }

    #[test]
    fn malformed_id() {
        let src = r##"
// @claim BAD
pub fn demo() {}
"##;
        let (sites, d) = scan_rust_source(Path::new("bad.rs"), &ls(src));
        assert!(sites.is_empty());
        assert!(d
            .iter()
            .any(|x| x.rule_id == RULE_MALFORMED && x.severity == Severity::Error));
    }

    #[test]
    fn duplicate_same_site_two_lines() {
        let src = r##"
#[test]
// @claim a.b
// @claim a.b
fn t() {}
"##;
        let (_, d) = scan_rust_source(Path::new("dup.rs"), &ls(src));
        assert!(
            d.iter()
                .any(|x| x.rule_id == RULE_DUPLICATE_SITE && x.severity == Severity::Info),
            "{d:?}"
        );
    }

    #[test]
    fn test_attr_classifies_as_test_site() {
        let src = r##"
#[test]
// @claim a.b
fn covers() {}
"##;
        let (sites, _) = scan_rust_source(Path::new("test_attr.rs"), &ls(src));
        assert_eq!(sites.len(), 1);
        assert!(matches!(sites[0].kind, SiteKind::Test));
    }

    #[test]
    fn block_comment_form_is_not_accepted() {
        let src = r##"
/* @claim a.b */
pub fn demo() {}
"##;
        let (sites, _diags) = scan_rust_source(Path::new("blk.rs"), &ls(src));
        assert!(
            sites.is_empty(),
            "block-comment @claim must not produce a Rust site (ADR-0023)"
        );
    }

    #[test]
    fn raw_string_fixture_claims_are_not_extracted_from_rust_source() {
        let src = r##"
const FIXTURE: &str = r#"
// @claim demo.fixture
export function demo() {}
"#;

// @claim demo.real
pub fn demo() {}
"##;
        let (sites, diags) = scan_rust_source(Path::new("fixture_host.rs"), &ls(src));
        assert!(
            diags.is_empty(),
            "raw-string fixture contents must not produce trace diagnostics: {diags:?}"
        );
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].claim_id, "demo.real");
    }
}
