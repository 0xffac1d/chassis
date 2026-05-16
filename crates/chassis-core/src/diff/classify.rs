//! Helpers for severity classification: assurance-ladder ordering and the
//! kind-specific required-field table from `schemas/contract.schema.json`.

/// Assurance ladder rank per ADR-0002:
/// `declared < coherent < verified < enforced < observed`.
pub fn ladder_rank(level: &str) -> Option<u8> {
    match level {
        "declared" => Some(0),
        "coherent" => Some(1),
        "verified" => Some(2),
        "enforced" => Some(3),
        "observed" => Some(4),
        _ => None,
    }
}

/// Kind-specific required fields per `schemas/contract.schema.json` `oneOf`
/// branches. `kind` itself is omitted — that's handled by the top-level kind
/// check, not the required-field-removal rule.
pub fn kind_required_fields(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "library" => &["exports"],
        "cli" => &["argsSummary", "entrypoint"],
        "component" => &["props", "events", "slots", "states"],
        "endpoint" => &["auth", "method", "path", "request", "response"],
        "entity" => &["fields", "relationships"],
        "service" => &["protocol", "endpoints", "consumes", "produces"],
        "event-stream" => &["consumers", "delivery", "payload", "source"],
        "feature-flag" => &["defaultValue", "metrics", "targeting", "type"],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    use serde_json::Value;

    use super::*;

    const UNIVERSAL_REQUIRED: &[&str] = &[
        "name",
        "kind",
        "purpose",
        "status",
        "since",
        "version",
        "assurance_level",
        "owner",
        "invariants",
        "edge_cases",
    ];

    #[test]
    fn ladder_ordering_matches_adr_0002() {
        let rungs = ["declared", "coherent", "verified", "enforced", "observed"];
        // All 10 ordered pairs (i, j) with i < j must satisfy rank(i) < rank(j).
        for (i, a) in rungs.iter().enumerate() {
            for b in &rungs[i + 1..] {
                let ra = ladder_rank(a).unwrap();
                let rb = ladder_rank(b).unwrap();
                assert!(
                    ra < rb,
                    "ladder pair violates ADR-0002: rank({a})={ra} not < rank({b})={rb}"
                );
            }
        }
    }

    #[test]
    fn unknown_levels_have_no_rank() {
        assert!(ladder_rank("nonsense").is_none());
        assert!(ladder_rank("").is_none());
    }

    #[test]
    fn kind_required_table_matches_contract_kind_schemas() {
        let schemas_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/contract-kinds");
        assert!(schemas_dir.is_dir(), "expected {}", schemas_dir.display());

        let pairs = [
            ("library", "library.schema.json"),
            ("cli", "cli.schema.json"),
            ("component", "component.schema.json"),
            ("endpoint", "endpoint.schema.json"),
            ("entity", "entity.schema.json"),
            ("service", "service.schema.json"),
            ("event-stream", "event-stream.schema.json"),
            ("feature-flag", "feature-flag.schema.json"),
        ];

        for &(kind, fname) in &pairs {
            let p = schemas_dir.join(fname);
            let raw: Value = serde_json::from_reader(
                fs::File::open(&p).unwrap_or_else(|e| panic!("open {}: {e}", p.display())), // nosemgrep: chassis-no-panic-runtime-core
            )
            .unwrap_or_else(|e| panic!("parse {}: {e}", p.display())); // nosemgrep: chassis-no-panic-runtime-core
            let req = raw
                .get("required")
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("{}: missing required", p.display())); // nosemgrep: chassis-no-panic-runtime-core

            let mut from_schema: BTreeSet<String> = BTreeSet::new();
            for v in req {
                let s = v.as_str().expect("required[] strings");
                if !UNIVERSAL_REQUIRED.contains(&s) {
                    from_schema.insert(s.to_string());
                }
            }
            let tbl: BTreeSet<String> = kind_required_fields(kind)
                .unwrap_or_else(|| panic!("no table row for kind `{kind}`")) // nosemgrep: chassis-no-panic-runtime-core
                .iter()
                .map(|s| (*s).to_string())
                .collect();

            assert_eq!(
                from_schema,
                tbl,
                "kind `{kind}` required-field slice drift vs {}",
                p.display()
            );
        }
    }

    #[test]
    fn unknown_kinds_have_no_required_fields() {
        assert!(kind_required_fields("nonsense").is_none());
    }

    #[test]
    fn all_supported_kinds_present() {
        for k in [
            "library",
            "cli",
            "component",
            "endpoint",
            "entity",
            "service",
            "event-stream",
            "feature-flag",
        ] {
            assert!(
                kind_required_fields(k).is_some(),
                "missing required-fields entry for kind {k}"
            );
        }
    }
}
