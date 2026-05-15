//! Exemption registry — types, lifecycle operations, and verifier per ADR-0004.
//!
//! # Rule IDs
//!
//! All diagnostics emitted by this module use `ruleId`s of the form
//! `CH-EXEMPT-*`. They are documented in `docs/adr/ADR-0020-exemption-rules.md`.
//!
//! | Rule ID                            | Severity | Meaning                                                          |
//! |------------------------------------|----------|------------------------------------------------------------------|
//! | `CH-EXEMPT-QUOTA-EXCEEDED`         | error    | > 25 active entries.                                             |
//! | `CH-EXEMPT-LIFETIME-EXCEEDED`      | error    | `expires_at - created_at > 90 days`.                             |
//! | `CH-EXEMPT-EXPIRED`                | error    | Entry's `expires_at < now` but still present in registry.        |
//! | `CH-EXEMPT-MISSING-CODEOWNERS`     | error    | Required CODEOWNERS signoff missing from acknowledgments.        |
//! | `CH-EXEMPT-DUPLICATE-ID`           | error    | Two entries share the same `id`.                                 |
//! | `CH-EXEMPT-MALFORMED-ID`           | error    | `id` doesn't match `^EX-\d{4}-\d{4}$`.                           |
//! | `CH-EXEMPT-RULE-NOT-IN-ADR`        | warning  | The exempted `rule_id` doesn't resolve to an ADR's `enforces[]`. |
//! | `CH-EXEMPT-REMOVED-BY-SWEEPER`     | info     | Sweeper removed an expired entry.                                |
//! | `CH-EXEMPT-PATHS-EMPTY`            | error    | `paths` array is empty.                                          |
//! | `CH-EXEMPT-CODEOWNERS-PARSE-ERROR` | error    | CODEOWNERS file couldn't be parsed.                              |
//! | `CH-EXEMPT-NOT-FOUND`              | error    | `remove()` could not locate the requested id.                    |
//!
//! Cross-session note: `Diagnostic` is shared via [`crate::diagnostic::Diagnostic`].
//! Session D (contract-diff) also emits diagnostics through the same envelope.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub mod codeowners;
mod envelope;
mod lifecycle;
mod verify;

#[cfg(test)]
mod tests;

pub use crate::diagnostic::Diagnostic;
pub use codeowners::{Codeowners, CodeownersRule, ParseError as CodeownersParseError};

/// Maximum lifetime (created → expires) for any exemption entry. ADR-0004.
pub const MAX_LIFETIME_DAYS: i64 = 90;

/// Maximum number of simultaneously active entries. ADR-0004.
pub const MAX_ACTIVE_ENTRIES: usize = 25;

/// Rule IDs emitted by this module. Strings rather than an enum so they can
/// be embedded directly in [`Diagnostic::rule_id`].
pub mod rule_id {
    pub const QUOTA_EXCEEDED: &str = "CH-EXEMPT-QUOTA-EXCEEDED";
    pub const LIFETIME_EXCEEDED: &str = "CH-EXEMPT-LIFETIME-EXCEEDED";
    pub const EXPIRED: &str = "CH-EXEMPT-EXPIRED";
    pub const MISSING_CODEOWNERS: &str = "CH-EXEMPT-MISSING-CODEOWNERS";
    pub const DUPLICATE_ID: &str = "CH-EXEMPT-DUPLICATE-ID";
    pub const MALFORMED_ID: &str = "CH-EXEMPT-MALFORMED-ID";
    pub const RULE_NOT_IN_ADR: &str = "CH-EXEMPT-RULE-NOT-IN-ADR";
    pub const REMOVED_BY_SWEEPER: &str = "CH-EXEMPT-REMOVED-BY-SWEEPER";
    pub const PATHS_EMPTY: &str = "CH-EXEMPT-PATHS-EMPTY";
    pub const CODEOWNERS_PARSE_ERROR: &str = "CH-EXEMPT-CODEOWNERS-PARSE-ERROR";
    /// Emitted by `remove()` when the supplied id is not present. Routing aid;
    /// distinct from `DUPLICATE_ID` per ADR-0011 immutability.
    pub const NOT_FOUND: &str = "CH-EXEMPT-NOT-FOUND";
}

/// An exemption registry entry.
///
/// Round-trips via serde to/from JSON validating against
/// `schemas/exemption-registry.schema.json` (v1.2). `paths` deserializes from
/// either a string or array (schema's `path` oneOf form) and re-serializes as
/// an array for canonical output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exemption {
    pub id: String,

    #[serde(rename = "rule_id")]
    pub rule_id: String,

    pub reason: String,

    pub owner: String,

    pub created_at: NaiveDate,

    pub expires_at: NaiveDate,

    #[serde(
        rename = "path",
        deserialize_with = "deserialize_paths",
        serialize_with = "serialize_paths"
    )]
    pub paths: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub codeowner_acknowledgments: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_issue: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adr: Option<String>,
}

/// The whole exemption registry document (root-level wrapper).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    /// Matches the integer `version` enum from `exemption-registry.schema.json`.
    pub version: i64,

    pub entries: Vec<Exemption>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<serde_json::Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_global: Option<bool>,
}

impl Registry {
    /// Construct an empty v2 registry.
    pub fn empty() -> Self {
        Self {
            version: 2,
            entries: Vec::new(),
            quota: None,
            allow_global: None,
        }
    }
}

/// Filter passed to [`list`]. Each field, when `Some`, narrows the result.
#[derive(Debug, Default, Clone)]
pub struct ListFilter {
    pub rule_id: Option<String>,
    /// If set, only entries whose `paths` contain this string are returned.
    pub path: Option<String>,
    /// Filter to entries active at this date. None = no temporal filter.
    pub active_at: Option<NaiveDate>,
}

/// Add a new exemption to the registry.
///
/// Enforces every CH-EXEMPT-* rule with severity `error` plus the `RULE-NOT-IN-ADR`
/// warning when an `adr_rule_ids` set is supplied via [`add_with_adr_index`].
/// Use this entry point when no ADR index is available.
pub fn add(
    registry: Registry,
    entry: Exemption,
    now: DateTime<Utc>,
    codeowners: &Codeowners,
) -> Result<Registry, Vec<Diagnostic>> {
    lifecycle::add(registry, entry, now, codeowners, None)
}

/// Like [`add`], but also checks the entry's `rule_id` against a caller-supplied
/// set of rule IDs known to ADRs. A miss surfaces `CH-EXEMPT-RULE-NOT-IN-ADR`
/// (warning) but does not reject the entry.
pub fn add_with_adr_index(
    registry: Registry,
    entry: Exemption,
    now: DateTime<Utc>,
    codeowners: &Codeowners,
    adr_rule_ids: &[String],
) -> Result<Registry, Vec<Diagnostic>> {
    lifecycle::add(registry, entry, now, codeowners, Some(adr_rule_ids))
}

/// Remove an entry by id. Returns the new registry on success or an error
/// diagnostic when no entry matches.
pub fn remove(registry: Registry, id: &str) -> Result<Registry, Diagnostic> {
    lifecycle::remove(registry, id)
}

/// Filter the registry. Returns references in the order they appear in the
/// underlying `entries` vector.
pub fn list<'a>(registry: &'a Registry, filter: ListFilter) -> Vec<&'a Exemption> {
    lifecycle::list(registry, filter)
}

/// Remove every entry whose `expires_at` is strictly before `now`. Emits one
/// `CH-EXEMPT-REMOVED-BY-SWEEPER` info diagnostic per removed entry.
pub fn sweep(registry: Registry, now: DateTime<Utc>) -> (Registry, Vec<Diagnostic>) {
    lifecycle::sweep(registry, now)
}

/// Static checks across the whole registry.
///
/// Validates: ID grammar and uniqueness, lifetime cap, active-count quota,
/// expiration state, CODEOWNERS signoff coverage, non-empty paths.
/// Returns one diagnostic per violation. An empty vec = clean.
pub fn verify(
    registry: &Registry,
    now: DateTime<Utc>,
    codeowners: &Codeowners,
) -> Vec<Diagnostic> {
    verify::verify(registry, now, codeowners, None)
}

/// Like [`verify`], but also surfaces `CH-EXEMPT-RULE-NOT-IN-ADR` warnings for
/// `rule_id`s that don't appear in the supplied set.
pub fn verify_with_adr_index(
    registry: &Registry,
    now: DateTime<Utc>,
    codeowners: &Codeowners,
    adr_rule_ids: &[String],
) -> Vec<Diagnostic> {
    verify::verify(registry, now, codeowners, Some(adr_rule_ids))
}

/// True iff `id` matches the canonical EX-YYYY-NNNN grammar.
pub(crate) fn id_matches_grammar(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() != 12 {
        return false;
    }
    if &bytes[0..3] != b"EX-" || bytes[7] != b'-' {
        return false;
    }
    (3..7).chain(8..12).all(|i| bytes[i].is_ascii_digit())
}

/// Compute `expires_at - created_at` in whole days. Negative values are
/// returned as `i64::MIN` to signal "expires before created" (also a violation).
pub(crate) fn lifetime_days(created: NaiveDate, expires: NaiveDate) -> i64 {
    (expires - created).num_days()
}

/// True iff `now`'s UTC date is strictly after `expires_at`.
pub(crate) fn is_expired(expires: NaiveDate, now: DateTime<Utc>) -> bool {
    now.date_naive() > expires
}

/// Return the calendar-day delta count expected by ADR-0004's 90-day cap.
#[allow(dead_code)]
pub(crate) fn year_of(date: NaiveDate) -> i32 {
    date.year()
}

fn deserialize_paths<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize as _;
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        S(String),
        V(Vec<String>),
    }
    Ok(match StringOrVec::deserialize(d)? {
        StringOrVec::S(s) => vec![s],
        StringOrVec::V(v) => v,
    })
}

fn serialize_paths<S>(paths: &Vec<String>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = s.serialize_seq(Some(paths.len()))?;
    for p in paths {
        seq.serialize_element(p)?;
    }
    seq.end()
}
