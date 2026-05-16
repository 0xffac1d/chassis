# Release policy evaluated by OPA over `chassis export --format opa` JSON.
# Chassis exports evidence only; this package decides allow/deny.
#
# Input shape: { "input": <PolicyInput per schemas/policy-input.schema.json> }
# Query: data.chassis.release.result and/or data.chassis.release.allow

package chassis.release

import rego.v1

# Fail closed unless every rule passes.
default allow := false

# Wrapped policy object from export (null / non-object => deny).
pi := object.get(input, "input", null)

deny_reasons contains "missing_opa_input_wrapper" if {
	pi == null
}

deny_reasons contains "policy_input_not_object" if {
	pi != null
	not is_object(pi)
}

version := object.get(pi, "version", null)

deny_reasons contains "invalid_policy_version" if {
	is_object(pi)
	not version == 1
}

ds := object.get(pi, "drift_summary", null)

deny_reasons contains "missing_drift_summary" if {
	is_object(pi)
	version == 1
	not is_object(ds)
}

stale := object.get(ds, "stale", -1)

abandoned := object.get(ds, "abandoned", -1)

missing := object.get(ds, "missing", -1)

deny_reasons contains "drift_stale_nonzero" if {
	version == 1
	is_object(ds)
	stale > 0
}

deny_reasons contains "drift_abandoned_nonzero" if {
	version == 1
	is_object(ds)
	abandoned > 0
}

deny_reasons contains "drift_missing_nonzero" if {
	version == 1
	is_object(ds)
	missing > 0
}

claims := object.get(pi, "claims", [])

deny_reasons contains sprintf("claim_missing_impl_sites:%s", [object.get(claim, "claim_id", "")]) if {
	version == 1
	is_object(pi)
	some claim
	claim = claims[_]
	count(object.get(claim, "impl_sites", [])) == 0
}

deny_reasons contains sprintf("claim_missing_test_sites:%s", [object.get(claim, "claim_id", "")]) if {
	version == 1
	is_object(pi)
	some claim
	claim = claims[_]
	count(object.get(claim, "test_sites", [])) == 0
}

# Blocking: error or warning in merged diagnostics (trace + drift + exemption verify).
diagnostics := object.get(pi, "diagnostics", [])

deny_reasons contains sprintf("diagnostic_error:%s", [object.get(d, "ruleId", "")]) if {
	version == 1
	is_object(pi)
	some d
	d = diagnostics[_]
	d.severity == "error"
}

deny_reasons contains sprintf("diagnostic_warning:%s", [object.get(d, "ruleId", "")]) if {
	version == 1
	is_object(pi)
	some d
	d = diagnostics[_]
	d.severity == "warning"
}

# Belt-and-suspenders: malformed severities must not slip past OPA type checks if schema validation is bypassed.
deny_reasons contains "diagnostic_schema_invalid" if {
	version == 1
	is_object(pi)
	some d
	d = diagnostics[_]
	not diagnostic_severity_valid(d)
}

deny_reasons contains "exemption_diagnostic_schema_invalid" if {
	version == 1
	is_object(pi)
	is_object(ex)
	some d
	d = object.get(ex, "diagnostics", [])[_]
	not diagnostic_severity_valid(d)
}

diagnostic_severity_valid(d) if d.severity == "error"

diagnostic_severity_valid(d) if d.severity == "warning"

diagnostic_severity_valid(d) if d.severity == "info"

ex := object.get(pi, "exemptions", null)

deny_reasons contains "missing_exemptions" if {
	is_object(pi)
	version == 1
	not is_object(ex)
}

deny_reasons contains sprintf("exemption_verify_error:%s", [object.get(d, "ruleId", "")]) if {
	version == 1
	is_object(ex)
	some d
	d = object.get(ex, "diagnostics", [])[_]
	d.severity == "error"
}

# Scanner evidence (normalized SARIF summaries); see ADR-0033.
scanner_summaries := object.get(pi, "scanner_summaries", [])

scanner_evidence_required if {
	object.get(pi, "scanner_required", false) == true
}

deny_reasons contains sprintf("scanner_required_missing:%s", [tool]) if {
	version == 1
	is_object(pi)
	scanner_evidence_required
	tool = scanner_required_tools[_]
	not has_scanner_tool(tool)
}

scanner_required_tools := ["semgrep", "codeql"]

has_scanner_tool(tool) if {
	some s
	s = scanner_summaries[_]
	object.get(s, "tool", "") == tool
}

deny_reasons contains sprintf("scanner_error:%s", [object.get(d, "ruleId", "")]) if {
	version == 1
	is_object(pi)
	some s
	s = scanner_summaries[_]
	some d
	d = object.get(s, "diagnostics", [])[_]
	d.severity == "error"
}

allow if {
	count(deny_reasons) == 0
}

# Machine-readable outcome for CI and attest consumers.
result := {
	"allow": allow,
	"policy_package": "chassis.release",
	"deny_reasons": sort([r | r := deny_reasons[_]]),
}
