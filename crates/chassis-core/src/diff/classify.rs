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
        "component" => &[
            "accessibility",
            "dependencies",
            "events",
            "props",
            "slots",
            "states",
            "ui_taxonomy",
        ],
        "endpoint" => &["auth", "method", "path", "request", "response"],
        "entity" => &["fields", "indexes", "relationships", "timestamps"],
        "service" => &[
            "consumes",
            "endpoints",
            "produces",
            "protocol",
            "resilience",
        ],
        "event-stream" => &["consumers", "delivery", "payload", "source"],
        "feature-flag" => &["defaultValue", "metrics", "targeting", "type"],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_ordering_matches_adr_0002() {
        let rungs = [
            "declared",
            "coherent",
            "verified",
            "enforced",
            "observed",
        ];
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
