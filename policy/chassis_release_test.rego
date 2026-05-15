package chassis.release_test

import rego.v1

test_allow_empty_repo if {
	data.chassis.release.allow with input as clean_input
}

test_result_clean if {
	r := data.chassis.release.result with input as clean_input
	r.allow == true
	r.deny_reasons == []
}

test_deny_empty_wrapper if {
	not data.chassis.release.allow with input as {}
}

test_deny_invalid_version if {
	not data.chassis.release.allow with input as {"input": {"version": 2}}
}

test_deny_drift_stale if {
	not data.chassis.release.allow with input as drift_stale_input
}

test_deny_missing_claim_impl if {
	not data.chassis.release.allow with input as missing_impl_input
}

test_deny_diagnostic_error if {
	not data.chassis.release.allow with input as diagnostic_error_input
}

# Minimal schema-valid policy input (OPA tests use the same keys as chassis export).
clean_input := {"input": {
	"version": 1,
	"repo": {
		"root": "/tmp/repo",
		"git_commit": "abc",
		"schema_fingerprint": "a234567890123456789012345678901234567890123456789012345678901234",
	},
	"contracts": [],
	"claims": [],
	"diagnostics": [],
	"exemptions": {"registry": null, "diagnostics": []},
	"drift_summary": {"stale": 0, "abandoned": 0, "missing": 0},
}}

drift_stale_input := {"input": {
	"version": 1,
	"repo": {"root": "."},
	"contracts": [],
	"claims": [],
	"diagnostics": [],
	"exemptions": {"registry": null, "diagnostics": []},
	"drift_summary": {"stale": 1, "abandoned": 0, "missing": 0},
}}

missing_impl_input := {"input": {
	"version": 1,
	"repo": {"root": "."},
	"contracts": [],
	"claims": [{
		"claim_id": "demo.alpha",
		"contract_path": "CONTRACT.yaml",
		"contract_kind": "invariant",
		"claim_record": {"id": "demo.alpha", "text": "t"},
		"impl_sites": [],
		"test_sites": [{"file": "lib.rs", "line": 1, "claim_id": "demo.alpha", "kind": "test"}],
		"adr_refs": [],
		"active_exemptions": [],
	}],
	"diagnostics": [],
	"exemptions": {"registry": null, "diagnostics": []},
	"drift_summary": {"stale": 0, "abandoned": 0, "missing": 0},
}}

diagnostic_error_input := {"input": {
	"version": 1,
	"repo": {"root": "."},
	"contracts": [],
	"claims": [],
	"diagnostics": [{"ruleId": "CH-TEST-RULE", "severity": "error", "message": "failure"}],
	"exemptions": {"registry": null, "diagnostics": []},
	"drift_summary": {"stale": 0, "abandoned": 0, "missing": 0},
}}
