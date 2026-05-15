#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::diagnostic::{Diagnostic, Severity, Violated};
use crate::trace::extract::rust::{normalize_rel, RULE_DUPLICATE_SITE, RULE_MALFORMED};
use crate::trace::types::{ClaimSite, SiteKind};

static CLAIM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*//\s*@claim\s+([^\s]+)\s*$").expect("ts claim regex"));

static CLAIM_ID_OK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_.-]*$").expect("claim id grammar per STABLE-IDS"));

// Rejected pre-ADR-0023 JSDoc form. Matches `/** @claim id */`, `/* @claim id */`,
// and a leading-` * @claim id` line inside a multi-line JSDoc block. Used only to
// surface the rejected syntax as `CH-TRACE-MALFORMED-CLAIM` so it cannot fail
// silently — see ADR-0023 for the accepted grammar.
static REJECTED_JSDOC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:/\*+|\*)\s*@claim\b").expect("rejected jsdoc claim regex")
});

fn diag(rule: &str, sev: Severity, msg: String, subject: String) -> Diagnostic {
    Diagnostic {
        rule_id: rule.to_string(),
        severity: sev,
        message: msg,
        source: Some("trace::extract::typescript".to_string()),
        subject: Some(subject),
        violated: Some(Violated {
            convention: "ADR-0023".to_string(),
        }),
        docs: None,
        fix: None,
        location: None,
        detail: None,
    }
}

#[inline]
fn scan_for_jest(lines: &[String], upto: usize) -> bool {
    let start = upto.saturating_sub(8);
    lines
        .iter()
        .take(upto)
        .skip(start)
        .any(|l| l.contains("jest") || l.contains("vitest"))
}

fn find_back_site_idx(lines: &[String], mut pos: usize) -> Option<usize> {
    while pos < lines.len() {
        let t = lines[pos].trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with("/*") || t.starts_with("*") {
            pos += 1;
            continue;
        }
        // `export { foo, bar }` re-exports and bare `import` statements are not
        // binding sites; skip them. `export function`/`export const`/`export class`
        // ARE binding sites and must not be skipped (see ADR-0023).
        if t.starts_with("import ") || t.starts_with("export {") || t.starts_with("export *") {
            pos += 1;
            continue;
        }
        return Some(pos);
    }
    None
}

pub fn scan_typescript(rel: &Path, lines: &[String]) -> (Vec<ClaimSite>, Vec<Diagnostic>) {
    use std::collections::hash_map::{Entry, HashMap};

    let mut sites = Vec::new();
    let mut diags = Vec::new();
    let mut seen: HashMap<(usize, String), usize> = HashMap::new();

    let mut idx = 0usize;
    while idx < lines.len() {
        if !CLAIM_RE.is_match(&lines[idx]) {
            if REJECTED_JSDOC_RE.is_match(&lines[idx]) {
                diags.push(diag(
                    RULE_MALFORMED,
                    Severity::Error,
                    "JSDoc `@claim` form is rejected by ADR-0023 (supersedes ADR-0005); \
                     use `// @claim <id>` on its own line immediately before the backed item"
                        .to_string(),
                    format!("{}:{}", rel.display(), idx + 1),
                ));
            }
            idx += 1;
            continue;
        }
        let first = idx + 1;
        let mut ids = Vec::<String>::new();
        while idx < lines.len() {
            let l = &lines[idx];
            if let Some(cap) = CLAIM_RE.captures(l) {
                let raw = cap.get(1).unwrap().as_str().to_string();
                if CLAIM_ID_OK.is_match(&raw) {
                    ids.push(raw);
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

        let is_test = lines[site_ix].contains("test(")
            || lines[site_ix].contains(".test.")
            || lines[site_ix].contains("describe(")
            || scan_for_jest(lines, site_ix);

        let kind = if is_test {
            SiteKind::Test
        } else {
            SiteKind::Impl
        };

        let site_ln = site_ix + 1;
        for cid in ids {
            let key = (site_ln, cid.clone());
            match seen.entry(key) {
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
                        line: first,
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

pub fn extract_typescript(root: &Path) -> (Vec<ClaimSite>, Vec<Diagnostic>) {
    let pk = root.join("packages");
    let mut sites = Vec::new();
    let mut diags = Vec::new();
    if !pk.is_dir() {
        return (sites, diags);
    }
    walk_ts(&pk, root, &mut sites, &mut diags);
    (sites, diags)
}

fn walk_ts(dir: &Path, root: &Path, sites: &mut Vec<ClaimSite>, diags: &mut Vec<Diagnostic>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_dir() {
            if p.file_name().and_then(|n| n.to_str()) == Some("node_modules") {
                continue;
            }
            walk_ts(&p, root, sites, diags);
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str());
        if !matches!(ext, Some("ts") | Some("tsx")) {
            continue;
        }
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
        let (mut s, mut d) = scan_typescript(&rel, &ls);
        sites.append(&mut s);
        diags.append(&mut d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ls(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    #[test]
    fn line_comment_is_accepted() {
        let src = r##"
// @claim demo.alpha
export function demo() {}
"##;
        let (sites, diags) = scan_typescript(Path::new("pkg/demo.ts"), &ls(src));
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].claim_id, "demo.alpha");
        assert!(matches!(sites[0].kind, SiteKind::Impl));
        assert!(diags.iter().all(|d| d.rule_id != RULE_MALFORMED));
    }

    #[test]
    fn malformed_dup_like_rust() {
        let src = r##"
// @claim BAD
export const x = 1;
"##;
        let (sites, diags) = scan_typescript(Path::new("pkg/x.ts"), &ls(src));
        assert!(sites.is_empty());
        assert!(diags.iter().any(|d| d.rule_id == RULE_MALFORMED));
    }

    #[test]
    fn jsdoc_form_is_rejected_loudly() {
        let src = r##"
/** @claim demo.alpha */
export function demo() {}
"##;
        let (sites, diags) = scan_typescript(Path::new("pkg/demo.ts"), &ls(src));
        assert!(
            sites.is_empty(),
            "JSDoc claim must not be admitted as a site"
        );
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == RULE_MALFORMED && d.severity == Severity::Error),
            "JSDoc form must surface CH-TRACE-MALFORMED-CLAIM, got {diags:?}"
        );
    }

    #[test]
    fn jsdoc_block_inner_star_form_is_rejected() {
        let src = r##"
/**
 * @claim demo.alpha
 */
export function demo() {}
"##;
        let (sites, diags) = scan_typescript(Path::new("pkg/demo.ts"), &ls(src));
        assert!(sites.is_empty());
        assert!(diags
            .iter()
            .any(|d| d.rule_id == RULE_MALFORMED && d.message.contains("ADR-0023")));
    }

    #[test]
    fn duplicate_same_site_two_lines() {
        let src = r##"
// @claim demo.alpha
// @claim demo.alpha
export function demo() {}
"##;
        let (_sites, diags) = scan_typescript(Path::new("pkg/dup.ts"), &ls(src));
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == RULE_DUPLICATE_SITE && d.severity == Severity::Info),
            "{diags:?}"
        );
    }

    #[test]
    fn test_call_is_classified_as_test_site() {
        let src = r##"
// @claim demo.alpha
test("returns 1", () => { /* ... */ });
"##;
        let (sites, _) = scan_typescript(Path::new("pkg/x.test.ts"), &ls(src));
        assert_eq!(sites.len(), 1);
        assert!(
            matches!(sites[0].kind, SiteKind::Test),
            "test(...) site must classify as SiteKind::Test"
        );
    }
}
