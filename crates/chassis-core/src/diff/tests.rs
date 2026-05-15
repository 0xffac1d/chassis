//! Tests for `diff()` — unit checks plus a fixture-driven sweep over
//! `fixtures/diff/<case>/{old.yaml,new.yaml,expected.json}`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::diagnostic::Severity;
use super::classify::ladder_rank;
use super::{
    diff,
    finding_classification,
    Classification,
    DiffError,
    CH_DIFF_CLAIM_REMOVED,
    CH_DIFF_PARSE_ERROR,
    CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR,
};

fn fixtures_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is chassis-core/. The fixtures live two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("diff")
}

fn load_yaml(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));
    serde_yaml::from_str(&text)
        .unwrap_or_else(|e| panic!("invalid yaml at {}: {e}", path.display()))
}

fn load_json(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("invalid json at {}: {e}", path.display()))
}

#[derive(Debug)]
struct ExpectedFinding {
    rule_id: String,
    severity: Option<String>,
    classification: Option<String>,
    subject: Option<String>,
}

#[derive(Debug)]
struct Expected {
    parse_error: bool,
    has_breaking: Option<bool>,
    expected_findings: Vec<ExpectedFinding>,
    counts: BTreeMap<String, usize>,
}

fn parse_expected(v: &Value) -> Expected {
    let parse_error = v
        .get("parse_error")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_breaking = v.get("has_breaking").and_then(Value::as_bool);
    let mut expected_findings = Vec::new();
    if let Some(arr) = v.get("findings").and_then(Value::as_array) {
        for f in arr {
            expected_findings.push(ExpectedFinding {
                rule_id: f
                    .get("ruleId")
                    .and_then(Value::as_str)
                    .expect("expected.findings[*].ruleId")
                    .to_string(),
                severity: f
                    .get("severity")
                    .and_then(Value::as_str)
                    .map(String::from),
                classification: f
                    .get("classification")
                    .and_then(Value::as_str)
                    .map(String::from),
                subject: f.get("subject").and_then(Value::as_str).map(String::from),
            });
        }
    }
    let mut counts = BTreeMap::new();
    if let Some(map) = v.get("rule_id_counts").and_then(Value::as_object) {
        for (k, val) in map {
            counts.insert(k.clone(), val.as_u64().unwrap() as usize);
        }
    }
    Expected {
        parse_error,
        has_breaking,
        expected_findings,
        counts,
    }
}

#[test]
fn fixture_driven_diff_cases() {
    let root = fixtures_root();
    assert!(
        root.is_dir(),
        "fixtures/diff/ not found at {}",
        root.display()
    );

    let mut cases: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no fixture directories under {}", root.display());

    let mut failures: Vec<String> = Vec::new();

    for case in cases {
        let case_name = case.file_name().unwrap().to_string_lossy().to_string();
        let old_path = case.join("old.yaml");
        let new_path = case.join("new.yaml");
        let expected_path = case.join("expected.json");

        if !expected_path.exists() {
            failures.push(format!("[{case_name}] missing expected.json"));
            continue;
        }
        let expected = parse_expected(&load_json(&expected_path));

        if expected.parse_error {
            // For parse-error cases, old.yaml/new.yaml may be intentionally
            // malformed-as-Contract; we expect DiffError::Parse.
            let old_v = if old_path.exists() { load_yaml(&old_path) } else { json!({}) };
            let new_v = if new_path.exists() { load_yaml(&new_path) } else { json!({}) };
            match diff(&old_v, &new_v) {
                Err(DiffError::Parse(_)) => {}
                other => failures.push(format!(
                    "[{case_name}] expected DiffError::Parse, got {other:?}"
                )),
            }
            continue;
        }

        if !old_path.exists() || !new_path.exists() {
            failures.push(format!(
                "[{case_name}] missing old.yaml or new.yaml"
            ));
            continue;
        }

        let old_v = load_yaml(&old_path);
        let new_v = load_yaml(&new_path);
        let report = match diff(&old_v, &new_v) {
            Ok(r) => r,
            Err(e) => {
                failures.push(format!("[{case_name}] unexpected DiffError: {e:?}"));
                continue;
            }
        };

        if let Some(want_break) = expected.has_breaking {
            if report.has_breaking() != want_break {
                failures.push(format!(
                    "[{case_name}] has_breaking={} but expected {want_break}",
                    report.has_breaking()
                ));
            }
        }

        // Rule-id counts.
        let mut got_counts: BTreeMap<String, usize> = BTreeMap::new();
        for d in &report.findings {
            *got_counts.entry(d.rule_id.clone()).or_default() += 1;
        }
        for (rule, want) in &expected.counts {
            let got = got_counts.get(rule).copied().unwrap_or(0);
            if got != *want {
                failures.push(format!(
                    "[{case_name}] rule_id_counts['{rule}']: got {got}, expected {want}"
                ));
            }
        }

        // Per-finding contract: each expected ruleId+subject+severity+classification
        // must be present at least once.
        for ef in &expected.expected_findings {
            let mut matched = false;
            for d in &report.findings {
                if d.rule_id != ef.rule_id {
                    continue;
                }
                if let Some(s) = &ef.subject {
                    if d.subject.as_deref() != Some(s.as_str()) {
                        continue;
                    }
                }
                if let Some(sev) = &ef.severity {
                    let got = match d.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                        Severity::Info => "info",
                    };
                    if got != sev.as_str() {
                        continue;
                    }
                }
                if let Some(cls) = &ef.classification {
                    let got = match finding_classification(d) {
                        Some(Classification::Breaking) => "breaking",
                        Some(Classification::NonBreaking) => "non-breaking",
                        Some(Classification::Additive) => "additive",
                        None => "",
                    };
                    if got != cls.as_str() {
                        continue;
                    }
                }
                matched = true;
                break;
            }
            if !matched {
                failures.push(format!(
                    "[{case_name}] no finding matched expectation {ef:?}\n  got: {:#?}",
                    report
                        .findings
                        .iter()
                        .map(|d| (
                            d.rule_id.clone(),
                            d.subject.clone(),
                            d.severity,
                            finding_classification(d)
                        ))
                        .collect::<Vec<_>>()
                ));
            }
        }

        // Unexpected rule ids: if `findings` is given, treat its rule_id SET as the
        // exact set emitted. (rule_id_counts can subsume; this is a strictness
        // toggle controlled by presence of `strict_rule_id_set: true`.)
        if let Some(true) = load_json(&expected_path)
            .get("strict_rule_id_set")
            .and_then(Value::as_bool)
        {
            let expected_set: BTreeSet<String> =
                expected.expected_findings.iter().map(|e| e.rule_id.clone()).collect();
            let actual_set: BTreeSet<String> =
                report.findings.iter().map(|d| d.rule_id.clone()).collect();
            if expected_set != actual_set {
                failures.push(format!(
                    "[{case_name}] strict rule id set mismatch:\n  expected={:?}\n  actual=  {:?}",
                    expected_set, actual_set
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "fixture-driven diff failures ({}):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

#[test]
fn diff_is_deterministic() {
    let root = fixtures_root();
    let case = root.join("breaking-claim-removed");
    let old_v = load_yaml(&case.join("old.yaml"));
    let new_v = load_yaml(&case.join("new.yaml"));

    let r1 = diff(&old_v, &new_v).unwrap();
    let r2 = diff(&old_v, &new_v).unwrap();
    let j1 = serde_json::to_string(&r1).unwrap();
    let j2 = serde_json::to_string(&r2).unwrap();
    assert_eq!(j1, j2, "diff outputs must be byte-identical across runs");
}

#[test]
fn empty_to_empty_is_parse_error() {
    let err = diff(&json!({}), &json!({})).unwrap_err();
    assert!(
        matches!(err, DiffError::Parse(_)),
        "expected DiffError::Parse, got {err:?}"
    );
    assert!(err.to_string().contains(CH_DIFF_PARSE_ERROR));
}

#[test]
fn non_object_is_parse_error() {
    assert!(matches!(
        diff(&json!("nope"), &json!({"name": "x"})).unwrap_err(),
        DiffError::Parse(_)
    ));
    assert!(matches!(
        diff(&json!({"name": "x"}), &json!([])).unwrap_err(),
        DiffError::Parse(_)
    ));
}

#[test]
fn assurance_ladder_ordering_is_consistent_with_adr_0002() {
    // Reassert from this module to catch silent slip even if classify::tests
    // regresses (defense in depth).
    let rungs = ["declared", "coherent", "verified", "enforced", "observed"];
    for (i, a) in rungs.iter().enumerate() {
        for b in &rungs[i + 1..] {
            assert!(
                ladder_rank(a).unwrap() < ladder_rank(b).unwrap(),
                "{a} should rank below {b}"
            );
        }
    }
}

#[test]
fn semver_breaking_without_major_is_breaking() {
    // 0.1.0 -> 0.1.1 with a claim removed must emit BOTH:
    //   CH-DIFF-CLAIM-REMOVED (breaking)
    //   CH-DIFF-VERSION-BREAKING-WITHOUT-MAJOR (breaking)
    let old_v: Value = serde_yaml::from_str(
        r#"
name: tiny
kind: library
purpose: "Test contract for diff semver behavior."
status: stable
since: "0.1.0"
version: "0.1.0"
assurance_level: declared
owner: tests
exports: []
invariants:
  - id: a.one
    text: "first invariant"
  - id: a.two
    text: "second invariant"
edge_cases:
  - id: a.edge
    text: "first edge case"
"#,
    )
    .unwrap();
    let new_v: Value = serde_yaml::from_str(
        r#"
name: tiny
kind: library
purpose: "Test contract for diff semver behavior."
status: stable
since: "0.1.0"
version: "0.1.1"
assurance_level: declared
owner: tests
exports: []
invariants:
  - id: a.one
    text: "first invariant"
edge_cases:
  - id: a.edge
    text: "first edge case"
"#,
    )
    .unwrap();

    let report = diff(&old_v, &new_v).expect("diff should succeed");

    let rules: Vec<&str> = report.findings.iter().map(|d| d.rule_id.as_str()).collect();
    assert!(
        rules.contains(&CH_DIFF_CLAIM_REMOVED),
        "expected CH-DIFF-CLAIM-REMOVED, got: {rules:?}"
    );
    assert!(
        rules.contains(&CH_DIFF_VERSION_BREAKING_WITHOUT_MAJOR),
        "expected CH-DIFF-VERSION-BREAKING-WITHOUT-MAJOR, got: {rules:?}"
    );
    assert!(report.has_breaking(), "report must signal breaking");
}

#[test]
fn diff_envelope_conforms_to_diagnostic_schema() {
    // Each emitted Diagnostic, when serialized, must validate against
    // schemas/diagnostic.schema.json (ADR-0018 conformance).
    let schema_str = include_str!("../../../../schemas/diagnostic.schema.json");
    let schema: Value = serde_json::from_str(schema_str).expect("diagnostic schema parses");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");

    let root = fixtures_root();
    let mut checked = 0usize;
    for entry in fs::read_dir(&root).unwrap() {
        let case = entry.unwrap().path();
        if !case.is_dir() {
            continue;
        }
        let old = case.join("old.yaml");
        let new = case.join("new.yaml");
        if !old.exists() || !new.exists() {
            continue;
        }
        let Ok(report) = diff(&load_yaml(&old), &load_yaml(&new)) else {
            continue;
        };
        for d in &report.findings {
            let v = serde_json::to_value(d).unwrap();
            let errs: Vec<String> = validator.iter_errors(&v).map(|e| e.to_string()).collect();
            assert!(
                errs.is_empty(),
                "diagnostic from {} failed schema:\n  {}\n  errors: {:?}",
                case.display(),
                v,
                errs
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no diagnostics validated — fixtures may be empty");
}
