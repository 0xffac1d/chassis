//! Claim-set diff: invariants + edge_cases keyed by stable claim `id`.
//!
//! Per ADR-0003, claims are `{id, text, ...}` rows with `id` as the stable
//! identity. The diff uses `id` to align rows across versions; text deltas on
//! the same id are non-breaking, removals/cross-bucket moves can be breaking.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use super::{
    envelope, Diagnostic, CH_DIFF_CLAIM_ADDED, CH_DIFF_CLAIM_REMOVED, CH_DIFF_CLAIM_TEXT_CHANGED,
    CH_DIFF_EDGE_CASE_PROMOTED_TO_INVARIANT, CH_DIFF_INVARIANT_DEMOTED_TO_EDGE_CASE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bucket {
    Invariants,
    EdgeCases,
}

impl Bucket {
    fn key(self) -> &'static str {
        match self {
            Bucket::Invariants => "invariants",
            Bucket::EdgeCases => "edge_cases",
        }
    }
}

#[derive(Debug, Clone)]
struct ClaimRow {
    text: String,
    bucket: Bucket,
}

fn collect_claims(obj: &Map<String, Value>) -> BTreeMap<String, ClaimRow> {
    let mut out: BTreeMap<String, ClaimRow> = BTreeMap::new();
    for bucket in [Bucket::Invariants, Bucket::EdgeCases] {
        let Some(arr) = obj.get(bucket.key()).and_then(|v| v.as_array()) else {
            continue;
        };
        for item in arr {
            let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let text = item
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            // First write wins; duplicate ids inside the same document are a
            // schema authoring error caught upstream by validation.
            out.entry(id.to_string()).or_insert(ClaimRow { text, bucket });
        }
    }
    out
}

pub(super) fn diff_claim_sets(
    old: &Map<String, Value>,
    new: &Map<String, Value>,
    subject_prefix: &str,
    findings: &mut Vec<Diagnostic>,
) {
    let old_claims = collect_claims(old);
    let new_claims = collect_claims(new);

    // Deterministic order: BTreeMap walks sorted by id.
    for (id, old_row) in &old_claims {
        match new_claims.get(id) {
            None => {
                findings.push(envelope::breaking(
                    CH_DIFF_CLAIM_REMOVED,
                    &format!("{subject_prefix}.{}.{}", old_row.bucket.key(), id),
                    format!(
                        "{} claim {id:?} removed",
                        old_row.bucket.key()
                    ),
                    json!({
                        "id": id,
                        "bucket": old_row.bucket.key(),
                        "text": old_row.text,
                    }),
                ));
            }
            Some(new_row) => {
                if old_row.bucket == new_row.bucket {
                    if old_row.text != new_row.text {
                        findings.push(envelope::non_breaking(
                            CH_DIFF_CLAIM_TEXT_CHANGED,
                            &format!("{subject_prefix}.{}.{}", old_row.bucket.key(), id),
                            format!("{} claim {id:?} text changed", old_row.bucket.key()),
                            json!({
                                "id": id,
                                "bucket": old_row.bucket.key(),
                                "before": old_row.text,
                                "after": new_row.text,
                            }),
                        ));
                    }
                } else {
                    // Cross-bucket move — emit one rule per direction.
                    match (old_row.bucket, new_row.bucket) {
                        (Bucket::Invariants, Bucket::EdgeCases) => {
                            findings.push(envelope::breaking(
                                CH_DIFF_INVARIANT_DEMOTED_TO_EDGE_CASE,
                                &format!("{subject_prefix}.invariants.{id}"),
                                format!(
                                    "invariant {id:?} demoted to edge_case (weaker guarantee)"
                                ),
                                json!({
                                    "id": id,
                                    "from": "invariants",
                                    "to": "edge_cases",
                                }),
                            ));
                        }
                        (Bucket::EdgeCases, Bucket::Invariants) => {
                            findings.push(envelope::non_breaking(
                                CH_DIFF_EDGE_CASE_PROMOTED_TO_INVARIANT,
                                &format!("{subject_prefix}.edge_cases.{id}"),
                                format!(
                                    "edge_case {id:?} promoted to invariant (stronger guarantee)"
                                ),
                                json!({
                                    "id": id,
                                    "from": "edge_cases",
                                    "to": "invariants",
                                }),
                            ));
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    for (id, new_row) in &new_claims {
        if !old_claims.contains_key(id) {
            findings.push(envelope::additive(
                CH_DIFF_CLAIM_ADDED,
                &format!("{subject_prefix}.{}.{}", new_row.bucket.key(), id),
                format!("{} claim {id:?} added", new_row.bucket.key()),
                json!({
                    "id": id,
                    "bucket": new_row.bucket.key(),
                    "text": new_row.text,
                }),
            ));
        }
    }
}
