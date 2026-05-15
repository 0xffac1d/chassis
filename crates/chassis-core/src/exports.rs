#![forbid(unsafe_code)]

//! Export-only facts for downstream governance systems.
//!
//! This module deliberately serializes Chassis facts; it does not evaluate
//! policy, define a policy language, or decide enforcement outcomes.

// @claim chassis.exports-not-policy-engines

use std::path::PathBuf;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::contract::Claim;
use crate::diagnostic::Diagnostic;
use crate::drift::report::DriftSummaryCounts;
use crate::exempt::Registry;
use crate::trace::types::{ClaimContractKind, ClaimSite, TraceGraph};

static POLICY_INPUT_SCHEMA_STR: &str = include_str!("../../../schemas/policy-input.schema.json");
static OPA_INPUT_SCHEMA_STR: &str = include_str!("../../../schemas/opa-input.schema.json");
static CEDAR_FACTS_SCHEMA_STR: &str = include_str!("../../../schemas/cedar-facts.schema.json");
static EVENTCATALOG_METADATA_SCHEMA_STR: &str =
    include_str!("../../../schemas/eventcatalog-metadata.schema.json");

static POLICY_INPUT_SCHEMA: LazyLock<jsonschema::Validator> =
    LazyLock::new(|| compile_schema(POLICY_INPUT_SCHEMA_STR, "policy-input.schema.json"));
static OPA_INPUT_SCHEMA: LazyLock<jsonschema::Validator> =
    LazyLock::new(|| compile_schema(OPA_INPUT_SCHEMA_STR, "opa-input.schema.json"));
static CEDAR_FACTS_SCHEMA: LazyLock<jsonschema::Validator> =
    LazyLock::new(|| compile_schema(CEDAR_FACTS_SCHEMA_STR, "cedar-facts.schema.json"));
static EVENTCATALOG_METADATA_SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    compile_schema(
        EVENTCATALOG_METADATA_SCHEMA_STR,
        "eventcatalog-metadata.schema.json",
    )
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoFacts {
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractFact {
    pub path: PathBuf,
    pub name: String,
    pub kind: String,
    pub version: String,
    pub owner: String,
    pub assurance_level: String,
    pub status: String,
    pub document: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimFact {
    pub claim_id: String,
    pub contract_path: PathBuf,
    pub contract_kind: ClaimContractKind,
    pub claim_record: Claim,
    pub impl_sites: Vec<ClaimSite>,
    pub test_sites: Vec<ClaimSite>,
    pub adr_refs: Vec<String>,
    pub active_exemptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExemptionFacts {
    pub registry: Option<Registry>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Optional facts when `artifacts/spec-index.json` is present (digest for policy consumers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecKitExtension {
    pub spec_index_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInput {
    pub version: i32,
    pub repo: RepoFacts,
    pub contracts: Vec<ContractFact>,
    pub claims: Vec<ClaimFact>,
    pub diagnostics: Vec<Diagnostic>,
    pub exemptions: ExemptionFacts,
    pub drift_summary: DriftSummaryCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_kit: Option<SpecKitExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaInput {
    pub input: PolicyInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedarUid {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedarEntity {
    pub uid: CedarUid,
    pub attrs: Value,
    pub parents: Vec<CedarUid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedarActionFact {
    pub name: String,
    pub applies_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedarResourceFact {
    pub uid: CedarUid,
    pub attrs: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedarFacts {
    pub schema_version: i32,
    pub entities: Vec<CedarEntity>,
    pub actions: Vec<CedarActionFact>,
    pub resources: Vec<CedarResourceFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCatalogMetadata {
    pub schema_version: i32,
    pub services: Vec<Value>,
    pub messages: Vec<Value>,
    pub metadata: Value,
}

/// Build the canonical Chassis policy-input fact bundle.
///
/// **Exemption parity with the release gate:** when `exemptions.registry` is
/// present, the registry is applied to the trace, drift, and spec-link
/// diagnostic buckets the same way `chassis_core::gate::compute` applies it
/// — so OPA, Cedar, and any other consumer reading
/// `chassis export --format opa` cannot see (or deny on) a finding that
/// release-gate has already suppressed. Exemption *verifier* diagnostics
/// (registry shape errors) are passed through verbatim because they remain
/// fail-closed governance evidence and are never themselves suppressible.
#[allow(clippy::too_many_arguments)]
pub fn build_policy_input(
    repo: RepoFacts,
    contracts: Vec<ContractFact>,
    trace: &TraceGraph,
    drift_summary: DriftSummaryCounts,
    drift_diagnostics: Vec<Diagnostic>,
    exemptions: ExemptionFacts,
    spec_kit: Option<SpecKitExtension>,
    spec_link_diagnostics: Vec<Diagnostic>,
) -> PolicyInput {
    build_policy_input_at(
        repo,
        contracts,
        trace,
        drift_summary,
        drift_diagnostics,
        exemptions,
        spec_kit,
        spec_link_diagnostics,
        chrono::Utc::now(),
    )
}

/// Same as [`build_policy_input`] but takes an explicit `now` so deterministic
/// callers (tests, CI) can pin exemption window semantics.
#[allow(clippy::too_many_arguments)]
pub fn build_policy_input_at(
    repo: RepoFacts,
    contracts: Vec<ContractFact>,
    trace: &TraceGraph,
    drift_summary: DriftSummaryCounts,
    drift_diagnostics: Vec<Diagnostic>,
    exemptions: ExemptionFacts,
    spec_kit: Option<SpecKitExtension>,
    spec_link_diagnostics: Vec<Diagnostic>,
    now: chrono::DateTime<chrono::Utc>,
) -> PolicyInput {
    let trace_diag = trace.diagnostics.clone();
    let mut audit_diagnostics: Vec<Diagnostic> = Vec::new();

    let (trace_us, drift_us, spec_us) = if let Some(reg) = exemptions.registry.as_ref() {
        // Each bucket is filtered independently — same shape as gate.rs so
        // that suppressed/overridden counts add up identically.
        let t = crate::exempt::apply::apply_exemptions(trace_diag, reg, now);
        let d = crate::exempt::apply::apply_exemptions(drift_diagnostics, reg, now);
        let s = crate::exempt::apply::apply_exemptions(spec_link_diagnostics, reg, now);
        audit_diagnostics.extend(t.audit);
        audit_diagnostics.extend(d.audit);
        audit_diagnostics.extend(s.audit);
        (t.unsuppressed, d.unsuppressed, s.unsuppressed)
    } else {
        (trace_diag, drift_diagnostics, spec_link_diagnostics)
    };

    let mut diagnostics = Vec::new();
    diagnostics.extend(trace_us);
    diagnostics.extend(drift_us);
    diagnostics.extend(exemptions.diagnostics.clone());
    diagnostics.extend(spec_us);
    // Audit info diagnostics are non-blocking and let policy / agents see
    // the suppression actions chassis took on this run.
    diagnostics.extend(audit_diagnostics);

    PolicyInput {
        version: 1,
        repo,
        contracts,
        claims: trace
            .claims
            .values()
            .map(|node| ClaimFact {
                claim_id: node.claim_id.clone(),
                contract_path: node.contract_path.clone(),
                contract_kind: node.contract_kind,
                claim_record: node.claim_record.clone(),
                impl_sites: node.impl_sites.clone(),
                test_sites: node.test_sites.clone(),
                adr_refs: node.adr_refs.clone(),
                active_exemptions: node.active_exemptions.clone(),
            })
            .collect(),
        diagnostics,
        exemptions,
        drift_summary,
        spec_kit,
    }
}

pub fn contract_fact(path: PathBuf, document: Value) -> Result<ContractFact, String> {
    let obj = document
        .as_object()
        .ok_or_else(|| format!("{} is not a contract object", path.display()))?;
    Ok(ContractFact {
        path,
        name: string_field(obj, "name")?,
        kind: string_field(obj, "kind")?,
        version: string_field(obj, "version")?,
        owner: string_field(obj, "owner")?,
        assurance_level: string_field(obj, "assurance_level")?,
        status: string_field(obj, "status")?,
        document,
    })
}

pub fn opa_input(input: PolicyInput) -> OpaInput {
    OpaInput { input }
}

pub fn cedar_facts(input: &PolicyInput) -> CedarFacts {
    let repo_uid = CedarUid {
        entity_type: "Chassis::Repo".to_string(),
        id: input.repo.root.clone(),
    };
    let mut entities = vec![CedarEntity {
        uid: repo_uid.clone(),
        attrs: json!({
            "git_commit": input.repo.git_commit,
            "schema_fingerprint": input.repo.schema_fingerprint,
        }),
        parents: vec![],
    }];
    let mut resources = Vec::new();

    for contract in &input.contracts {
        let uid = CedarUid {
            entity_type: "Chassis::Contract".to_string(),
            id: contract.path.display().to_string(),
        };
        let attrs = json!({
            "name": contract.name,
            "kind": contract.kind,
            "version": contract.version,
            "owner": contract.owner,
            "assurance_level": contract.assurance_level,
            "status": contract.status,
        });
        entities.push(CedarEntity {
            uid: uid.clone(),
            attrs: attrs.clone(),
            parents: vec![repo_uid.clone()],
        });
        resources.push(CedarResourceFact { uid, attrs });
    }

    for claim in &input.claims {
        let contract_uid = CedarUid {
            entity_type: "Chassis::Contract".to_string(),
            id: claim.contract_path.display().to_string(),
        };
        let uid = CedarUid {
            entity_type: "Chassis::Claim".to_string(),
            id: claim.claim_id.clone(),
        };
        let attrs = json!({
            "text": claim.claim_record.text,
            "contract_path": claim.contract_path,
            "contract_kind": claim.contract_kind,
            "impl_site_count": claim.impl_sites.len(),
            "test_site_count": claim.test_sites.len(),
            "active_exemptions": claim.active_exemptions,
        });
        entities.push(CedarEntity {
            uid: uid.clone(),
            attrs: attrs.clone(),
            parents: vec![contract_uid],
        });
        resources.push(CedarResourceFact { uid, attrs });
    }

    CedarFacts {
        schema_version: 1,
        entities,
        actions: vec![
            CedarActionFact {
                name: "validate".to_string(),
                applies_to: vec!["Chassis::Contract".to_string()],
            },
            CedarActionFact {
                name: "trace".to_string(),
                applies_to: vec!["Chassis::Claim".to_string()],
            },
            CedarActionFact {
                name: "drift".to_string(),
                applies_to: vec!["Chassis::Claim".to_string()],
            },
            CedarActionFact {
                name: "exempt".to_string(),
                applies_to: vec![
                    "Chassis::Claim".to_string(),
                    "Chassis::Contract".to_string(),
                ],
            },
        ],
        resources,
    }
}

pub fn eventcatalog_metadata(input: &PolicyInput) -> EventCatalogMetadata {
    let mut services = Vec::new();
    let mut messages = Vec::new();

    for contract in &input.contracts {
        match contract.kind.as_str() {
            "service" => services.push(json!({
                "name": contract.name,
                "version": contract.version,
                "owner": contract.owner,
                "contract_path": contract.path,
                "protocol": contract.document.get("protocol").cloned().unwrap_or(Value::Null),
                "endpoints": contract.document.get("endpoints").cloned().unwrap_or_else(|| json!([])),
                "consumes": contract.document.get("consumes").cloned().unwrap_or_else(|| json!([])),
                "produces": contract.document.get("produces").cloned().unwrap_or_else(|| json!([])),
            })),
            "event-stream" => messages.push(json!({
                "name": contract.name,
                "version": contract.version,
                "owner": contract.owner,
                "contract_path": contract.path,
                "source": contract.document.get("source").cloned().unwrap_or(Value::Null),
                "payload": contract.document.get("payload").cloned().unwrap_or(Value::Null),
                "delivery": contract.document.get("delivery").cloned().unwrap_or(Value::Null),
                "consumers": contract.document.get("consumers").cloned().unwrap_or_else(|| json!([])),
            })),
            _ => {}
        }
    }

    EventCatalogMetadata {
        schema_version: 1,
        services,
        messages,
        metadata: json!({
            "source": "chassis",
            "note": "Export-only metadata derived from current Chassis service and event-stream contract fields.",
        }),
    }
}

pub fn validate_policy_input_value(value: &Value) -> Result<(), Vec<String>> {
    validate_value(&POLICY_INPUT_SCHEMA, value)
}

pub fn validate_opa_input_value(value: &Value) -> Result<(), Vec<String>> {
    validate_value(&OPA_INPUT_SCHEMA, value)
}

pub fn validate_cedar_facts_value(value: &Value) -> Result<(), Vec<String>> {
    validate_value(&CEDAR_FACTS_SCHEMA, value)
}

pub fn validate_eventcatalog_metadata_value(value: &Value) -> Result<(), Vec<String>> {
    validate_value(&EVENTCATALOG_METADATA_SCHEMA, value)
}

fn string_field(obj: &serde_json::Map<String, Value>, field: &str) -> Result<String, String> {
    obj.get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("contract missing string field `{field}`"))
}

fn compile_schema(raw: &'static str, name: &str) -> jsonschema::Validator {
    let schema: Value = serde_json::from_str(raw).unwrap_or_else(|e| panic!("{name}: {e}"));
    jsonschema::validator_for(&schema).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn validate_value(schema: &jsonschema::Validator, value: &Value) -> Result<(), Vec<String>> {
    let errors: Vec<String> = schema.iter_errors(value).map(|e| e.to_string()).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Claim;
    use crate::drift::report::DriftSummaryCounts;
    use crate::trace::types::{ClaimNode, SiteKind};
    use std::collections::BTreeMap;

    fn sample_input() -> PolicyInput {
        let contract_doc = json!({
            "name": "events",
            "kind": "event-stream",
            "purpose": "Publish account events.",
            "status": "stable",
            "since": "0.1.0",
            "version": "0.1.0",
            "assurance_level": "declared",
            "owner": "platform",
            "source": "accounts",
            "payload": { "format": "json", "schema_ref": "schemas/account-event.json" },
            "delivery": "at-least-once",
            "consumers": ["billing"],
            "invariants": [{ "id": "events.published", "text": "Events are published." }],
            "edge_cases": []
        });
        let contract = contract_fact(PathBuf::from("CONTRACT.yaml"), contract_doc).unwrap();
        let mut claims = BTreeMap::new();
        claims.insert(
            "events.published".to_string(),
            ClaimNode {
                claim_id: "events.published".to_string(),
                contract_path: PathBuf::from("CONTRACT.yaml"),
                contract_kind: ClaimContractKind::Invariant,
                claim_record: Claim {
                    id: "events.published".to_string(),
                    text: "Events are published.".to_string(),
                    test_linkage: None,
                },
                impl_sites: vec![ClaimSite {
                    file: PathBuf::from("src/lib.rs"),
                    line: 7,
                    claim_id: "events.published".to_string(),
                    kind: SiteKind::Impl,
                }],
                test_sites: vec![],
                adr_refs: vec![],
                active_exemptions: vec![],
            },
        );
        let trace = TraceGraph {
            claims,
            orphan_sites: vec![],
            diagnostics: vec![],
        };
        build_policy_input(
            RepoFacts {
                root: ".".to_string(),
                git_commit: Some("abc123".to_string()),
                schema_fingerprint: Some("f".repeat(64)),
            },
            vec![contract],
            &trace,
            DriftSummaryCounts {
                stale: 0,
                abandoned: 0,
                missing: 0,
            },
            vec![],
            ExemptionFacts {
                registry: None,
                diagnostics: vec![],
            },
            None,
            vec![],
        )
    }

    #[test]
    fn policy_input_validates() {
        let v = serde_json::to_value(sample_input()).unwrap();
        validate_policy_input_value(&v).expect("schema-valid policy input");
    }

    #[test]
    fn opa_input_wraps_policy_input() {
        let v = serde_json::to_value(opa_input(sample_input())).unwrap();
        validate_opa_input_value(&v).expect("schema-valid OPA input");
        assert!(v.get("input").is_some());
    }

    #[test]
    // @claim chassis.exports-not-policy-engines
    fn build_policy_input_applies_registry_exemptions_to_trace_drift_spec() {
        // Exemption registry that suppresses CH-DRIFT-IMPL-MISSING and
        // downgrades a fictional CH-TRACE-* rule. Build a policy input that
        // would otherwise carry blocking error diagnostics; confirm the
        // exemption pass mirrors gate semantics:
        //   - error-severity drift finding -> suppressed
        //   - error-severity trace finding -> downgraded to info
        //   - exemption verifier diagnostic -> passed through verbatim
        use crate::diagnostic::{Severity, Violated};
        use crate::exempt::{Exemption, ExemptionStatus, Registry};
        use chrono::TimeZone;

        let now = chrono::Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
        let drift_diag = Diagnostic {
            rule_id: "CH-DRIFT-IMPL-MISSING".into(),
            severity: Severity::Error,
            message: "missing impl for events.published".into(),
            source: Some("drift".into()),
            subject: Some("events.published".into()),
            violated: Some(Violated {
                convention: "ADR-0024".into(),
            }),
            docs: None,
            fix: None,
            location: None,
            detail: None,
        };
        let trace_diag = Diagnostic {
            rule_id: "CH-TRACE-CLAIM-NOT-IN-CONTRACT".into(),
            severity: Severity::Error,
            message: "orphan claim".into(),
            source: Some("trace".into()),
            subject: Some("orphan.id".into()),
            violated: Some(Violated {
                convention: "ADR-0023".into(),
            }),
            docs: None,
            fix: None,
            location: None,
            detail: None,
        };

        let suppress = Exemption {
            id: "EX-2026-0001".into(),
            rule_id: Some("CH-DRIFT-IMPL-MISSING".into()),
            finding_id: None,
            paths: vec!["**".into()],
            status: ExemptionStatus::Active,
            allow_global: Some(true),
            severity_override: None,
            created_at: chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            expires_at: chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
            owner: "platform".into(),
            reason: "fixture".into(),
            codeowner_acknowledgments: vec!["@platform".into()],
            linked_issue: None,
            adr: None,
        };
        let downgrade = Exemption {
            id: "EX-2026-0002".into(),
            rule_id: Some("CH-TRACE-CLAIM-NOT-IN-CONTRACT".into()),
            finding_id: None,
            paths: vec!["**".into()],
            status: ExemptionStatus::Active,
            allow_global: Some(true),
            severity_override: Some(Severity::Info),
            created_at: chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            expires_at: chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
            owner: "platform".into(),
            reason: "fixture".into(),
            codeowner_acknowledgments: vec!["@platform".into()],
            linked_issue: None,
            adr: None,
        };
        let registry = Registry {
            version: 2,
            entries: vec![suppress, downgrade],
            quota: None,
            allow_global: Some(true),
        };

        let mut sample = sample_input();
        let trace = TraceGraph {
            claims: Default::default(),
            orphan_sites: vec![],
            diagnostics: vec![trace_diag.clone()],
        };
        sample = build_policy_input_at(
            sample.repo.clone(),
            sample.contracts.clone(),
            &trace,
            DriftSummaryCounts {
                stale: 0,
                abandoned: 0,
                missing: 0,
            },
            vec![drift_diag.clone()],
            ExemptionFacts {
                registry: Some(registry),
                diagnostics: vec![],
            },
            None,
            vec![],
            now,
        );

        let v = serde_json::to_value(&sample).unwrap();
        validate_policy_input_value(&v).expect("schema-valid post-exemption policy input");

        let drift_present = sample
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "CH-DRIFT-IMPL-MISSING");
        let trace_present = sample
            .diagnostics
            .iter()
            .find(|d| d.rule_id == "CH-TRACE-CLAIM-NOT-IN-CONTRACT");
        assert!(
            !drift_present,
            "drift error must be suppressed in the merged diagnostics list"
        );
        let trace_after = trace_present.expect("trace diagnostic still surfaces");
        assert_eq!(
            trace_after.severity,
            Severity::Info,
            "trace severity must be downgraded by registry override"
        );
        let audit_count = sample
            .diagnostics
            .iter()
            .filter(|d| d.rule_id == "CH-EXEMPT-APPLIED")
            .count();
        assert!(
            audit_count >= 2,
            "audit info diagnostics must accompany suppress + override actions"
        );
    }

    #[test]
    // @claim chassis.exports-not-policy-engines
    fn cedar_facts_are_export_facts_not_policy_decisions() {
        let facts = cedar_facts(&sample_input());
        let v = serde_json::to_value(&facts).unwrap();
        validate_cedar_facts_value(&v).expect("schema-valid Cedar-style facts");
        assert!(facts
            .entities
            .iter()
            .any(|e| e.uid.entity_type == "Chassis::Claim"));
    }

    #[test]
    fn eventcatalog_metadata_uses_supported_event_contract_fields() {
        let metadata = eventcatalog_metadata(&sample_input());
        let v = serde_json::to_value(&metadata).unwrap();
        validate_eventcatalog_metadata_value(&v).expect("schema-valid EventCatalog metadata");
        assert_eq!(metadata.messages.len(), 1);
        assert_eq!(metadata.services.len(), 0);
    }
}
