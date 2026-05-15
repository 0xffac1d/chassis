//! Fixture-driven coverage for `chassis_core::trace::extract` per ADR-0023.
//!
//! Each subdirectory under `fixtures/trace-extract/` carries one source file
//! plus `expected.json` describing the sites and diagnostic rule IDs the
//! scanner must produce. The `language` discriminator picks the Rust or
//! TypeScript extractor entry point. New fixtures land here as sibling
//! directories — no new test code needed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chassis_core::diagnostic::{Diagnostic, Severity};
use chassis_core::trace::extract::{rust::scan_rust_source, typescript::scan_typescript};
use chassis_core::trace::types::{ClaimSite, SiteKind};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExpectedSite {
    claim_id: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedDiag {
    rule_id: String,
    severity: String,
}

#[derive(Debug, Deserialize)]
struct Expected {
    language: String,
    source: String,
    sites: Vec<ExpectedSite>,
    diagnostics: Vec<ExpectedDiag>,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/trace-extract")
        .canonicalize()
        .expect("fixtures/trace-extract present")
}

fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .map(|l| l.to_string())
        .collect()
}

fn run_one(case_dir: &Path) {
    let expected_path = case_dir.join("expected.json");
    let raw = fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", expected_path.display()));
    let exp: Expected = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {}: {e}", expected_path.display()));

    let src_path = case_dir.join(&exp.source);
    let lines = read_lines(&src_path);
    let rel = Path::new(case_dir.file_name().unwrap()).join(&exp.source);

    let (sites, diags): (Vec<ClaimSite>, Vec<Diagnostic>) = match exp.language.as_str() {
        "rust" => scan_rust_source(&rel, &lines),
        "typescript" => scan_typescript(&rel, &lines),
        other => panic!("unknown language `{other}` in {}", expected_path.display()),
    };

    assert_eq!(
        sites.len(),
        exp.sites.len(),
        "{}: site count — got {sites:?}, expected {:?}",
        case_dir.display(),
        exp.sites
    );
    for (got, want) in sites.iter().zip(exp.sites.iter()) {
        assert_eq!(
            got.claim_id,
            want.claim_id,
            "{}: site claim_id mismatch",
            case_dir.display()
        );
        let want_kind = match want.kind.as_str() {
            "impl" => SiteKind::Impl,
            "test" => SiteKind::Test,
            other => panic!(
                "{}: unknown site kind `{other}` in expected.json",
                case_dir.display()
            ),
        };
        assert_eq!(
            got.kind,
            want_kind,
            "{}: site kind mismatch for {}",
            case_dir.display(),
            got.claim_id
        );
    }

    // Diagnostics are compared by (rule_id, severity) multiset so ordering can
    // change without breaking fixtures.
    let mut got_set: BTreeMap<(String, String), usize> = BTreeMap::new();
    for d in &diags {
        let sev = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        *got_set
            .entry((d.rule_id.clone(), sev.to_string()))
            .or_insert(0) += 1;
    }
    let mut want_set: BTreeMap<(String, String), usize> = BTreeMap::new();
    for d in &exp.diagnostics {
        *want_set
            .entry((d.rule_id.clone(), d.severity.clone()))
            .or_insert(0) += 1;
    }
    assert_eq!(
        got_set,
        want_set,
        "{}: diagnostic multiset mismatch — got {diags:?}",
        case_dir.display()
    );
}

#[test]
fn trace_extract_fixtures_all_cases() {
    let root = fixtures_root();
    let mut cases: Vec<PathBuf> = fs::read_dir(&root)
        .expect("read fixtures/trace-extract")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert!(
        !cases.is_empty(),
        "no fixture cases under {}",
        root.display()
    );

    let mut ran = 0usize;
    for c in cases {
        run_one(&c);
        ran += 1;
    }
    assert!(ran >= 9, "expected ≥9 fixture cases, ran {ran}");
}
