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

test_deny_malformed_diagnostic_severity if {
	not data.chassis.release.allow with input as malformed_diag_severity_input
}

test_deny_malformed_exemption_diagnostic_severity if {
	not data.chassis.release.allow with input as malformed_exemption_diag_input
}

test_deny_scanner_required_missing if {
	not data.chassis.release.allow with input as scanner_required_missing_input
}

test_deny_scanner_summary_diagnostic_error if {
	not data.chassis.release.allow with input as scanner_diag_error_input
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
	"scanner_summaries": [],
	"scanner_required": false,
}}

drift_stale_input := {"input": {
	"version": 1,
	"repo": {"root": "."},
	"contracts": [],
	"claims": [],
	"diagnostics": [],
	"exemptions": {"registry": null, "diagnostics": []},
	"drift_summary": {"stale": 1, "abandoned": 0, "missing": 0},
	"scanner_summaries": [],
	"scanner_required": false,
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
	"scanner_summaries": [],
	"scanner_required": false,
}}

diagnostic_error_input := {"input": {
	"version": 1,
	"repo": {"root": "."},
	"contracts": [],
	"claims": [],
	"diagnostics": [{"ruleId": "CH-TEST-RULE", "severity": "error", "message": "failure"}],
	"exemptions": {"registry": null, "diagnostics": []},
	"drift_summary": {"stale": 0, "abandoned": 0, "missing": 0},
	"scanner_summaries": [],
	"scanner_required": false,
}}

malformed_diag_severity_input := {"input": {
	"version": 1,
	"repo": {"root": "."},
	"contracts": [],
	"claims": [],
	"diagnostics": [{"ruleId": "CH-TEST-RULE", "severity": "garbage", "message": "x"}],
	"exemptions": {"registry": null, "diagnostics": []},
	"drift_summary": {"stale": 0, "abandoned": 0, "missing": 0},
	"scanner_summaries": [],
	"scanner_required": false,
}}

malformed_exemption_diag_input := {"input": {
	"version": 1,
	"repo": {"root": "."},
	"contracts": [],
	"claims": [],
	"diagnostics": [],
	"exemptions": {
		"registry": null,
		"diagnostics": [{"ruleId": "CH-RULE-X", "severity": "nope", "message": "bad"}],
	},
	"drift_summary": {"stale": 0, "abandoned": 0, "missing": 0},
	"scanner_summaries": [],
	"scanner_required": false,
}}

scanner_required_missing_input := {"input": {
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
	"scanner_summaries": [],
	"scanner_required": true,
}}

scanner_diag_error_input := {"input": {
	"version": 1,
	"repo": {"root": "."},
	"contracts": [],
	"claims": [],
	"diagnostics": [],
	"exemptions": {"registry": null, "diagnostics": []},
	"drift_summary": {"stale": 0, "abandoned": 0, "missing": 0},
	"scanner_summaries": [{
		"tool": "semgrep",
		"sarifSha256": "a234567890123456789012345678901234567890123456789012345678901234",
		"total": 1,
		"errors": 1,
		"warnings": 0,
		"infos": 0,
		"diagnostics": [{
			"ruleId": "CH-SCANNER-FINDING",
			"severity": "error",
			"message": "semgrep hit",
		}],
	}],
	"scanner_required": false,
}}
