#!/usr/bin/env python3
"""
Compare two Chassis contract YAML documents and classify changes.
Requires Mike Farah yq on PATH (or YQ env).
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any

SURFACE_KEYS = ("inputs", "outputs", "exports", "name", "kind")
METADATA_NON_BREAKING = (
    "purpose",
    "owner",
    "tags",
    "caveats",
    "since",
    "status",
    "depended_by",
    "todos",
    "signature",
    "performance",
    "perf",
)


def yq_bin() -> str:
    return os.environ.get("YQ", "yq")


def load(path: Path) -> dict[str, Any]:
    try:
        proc = subprocess.run(
            [yq_bin(), "-o=json", str(path)],
            capture_output=True,
            text=True,
            check=True,
        )
    except FileNotFoundError:
        print("chassis contract-diff: yq not found; set YQ= to Mike Farah yq.", file=sys.stderr)
        sys.exit(1)
    except subprocess.CalledProcessError as e:
        print(e.stderr or str(e), file=sys.stderr)
        sys.exit(1)
    data = json.loads(proc.stdout)
    if not isinstance(data, dict):
        print("chassis contract-diff: expected YAML root object", file=sys.stderr)
        sys.exit(1)
    return data


def canon(v: Any) -> str:
    return json.dumps(v, sort_keys=True, separators=(",", ":"), default=str)


def diff_surface(old: dict[str, Any], new: dict[str, Any]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for key in SURFACE_KEYS:
        o, n = old.get(key), new.get(key)
        if canon(o) == canon(n):
            continue
        detail: dict[str, Any] = {"key": key, "before": o, "after": n}
        if key == "name":
            detail["rename"] = {"from": o, "to": n}
            out.append(
                {
                    "category": "contract_renamed",
                    "severity": "breaking",
                    "detail": detail,
                }
            )
            continue
        out.append(
            {
                "category": "breaking_surface",
                "severity": "breaking",
                "detail": detail,
            }
        )
    return out


def diff_depends_on(old: dict[str, Any], new: dict[str, Any]) -> list[dict[str, Any]]:
    o = list(old.get("depends_on") or [])
    n = list(new.get("depends_on") or [])
    co, cn = Counter(o), Counter(n)
    findings: list[dict[str, Any]] = []
    removed = sorted((co - cn).elements())
    added = sorted((cn - co).elements())
    for dep in removed:
        findings.append(
            {
                "category": "depends_on_removed",
                "severity": "breaking",
                "detail": {"dependency": dep},
            }
        )
    for dep in added:
        findings.append(
            {
                "category": "depends_on_added",
                "severity": "non_breaking",
                "detail": {"dependency": dep},
            }
        )
    return findings


def diff_edge_cases_multiset(old: list[str], new: list[str]) -> list[dict[str, Any]]:
    co, cn = Counter(old), Counter(new)
    out: list[dict[str, Any]] = []
    for text, count in (cn - co).items():
        for _ in range(count):
            out.append(
                {
                    "category": "added_edge_case",
                    "severity": "non_breaking",
                    "detail": {"text": text},
                }
            )
    for text, count in (co - cn).items():
        for _ in range(count):
            out.append(
                {
                    "category": "removed_edge_case",
                    "severity": "breaking",
                    "detail": {"text": text},
                }
            )
    return out


def _edge_case_text(item: Any) -> str:
    """Behavioral text of an edge_cases entry (string or {text: ...})."""
    if isinstance(item, dict):
        return str(item.get("text", ""))
    return str(item)


def _invariant_text(item: Any) -> str:
    """Extract the behavioral text of an invariant.

    Invariants in CONTRACT.yaml are either plain strings or objects of the
    form ``{text: "...", test_linkage: [...]}``. The behavioral claim is the
    ``text`` field; ``test_linkage`` is verification metadata that is
    additive and never breaking on its own.
    """
    if isinstance(item, dict):
        return str(item.get("text", ""))
    return str(item)


def _invariant_test_linkage(item: Any) -> list[Any]:
    if isinstance(item, dict):
        tl = item.get("test_linkage")
        if isinstance(tl, list):
            return tl
    return []


def diff_invariants_indexed(old: list[Any], new: list[Any]) -> list[dict[str, Any]]:
    """Index-aligned changes plus multiset tail for length mismatch.

    Compares the behavioral ``text`` of each invariant, not the full object.
    A change to ``test_linkage`` (added, removed, or modified) on an
    otherwise-unchanged invariant is classified as ``non_breaking_test_linkage``
    rather than ``changed_invariant`` — adding verification to a stated
    invariant is an improvement, not a breaking semantic change.
    """
    out: list[dict[str, Any]] = []
    m = min(len(old), len(new))
    for i in range(m):
        old_text = _invariant_text(old[i])
        new_text = _invariant_text(new[i])
        if old_text != new_text:
            out.append(
                {
                    "category": "changed_invariant",
                    "severity": "breaking",
                    "detail": {"index": i, "before": old_text, "after": new_text},
                }
            )
            continue
        # Text unchanged. Check for test_linkage delta.
        old_tl = _invariant_test_linkage(old[i])
        new_tl = _invariant_test_linkage(new[i])
        if canon(old_tl) != canon(new_tl):
            out.append(
                {
                    "category": "non_breaking_test_linkage",
                    "severity": "non_breaking",
                    "detail": {
                        "index": i,
                        "text": new_text,
                        "before": old_tl,
                        "after": new_tl,
                    },
                }
            )
    if len(old) > len(new):
        for i in range(len(new), len(old)):
            out.append(
                {
                    "category": "removed_invariant",
                    "severity": "breaking",
                    "detail": {"index": i, "text": _invariant_text(old[i])},
                }
            )
    elif len(new) > len(old):
        for i in range(len(old), len(new)):
            out.append(
                {
                    "category": "added_invariant",
                    "severity": "non_breaking",
                    "detail": {"index": i, "text": _invariant_text(new[i])},
                }
            )
    return out


def diff_metadata(old: dict[str, Any], new: dict[str, Any]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for key in METADATA_NON_BREAKING:
        if key not in old and key not in new:
            continue
        o, n = old.get(key), new.get(key)
        if canon(o) != canon(n):
            out.append(
                {
                    "category": "non_breaking_metadata",
                    "severity": "non_breaking",
                    "detail": {"key": key, "before": o, "after": n},
                }
            )
    return out


def analyze(old: dict[str, Any], new: dict[str, Any]) -> dict[str, Any]:
    findings: list[dict[str, Any]] = []
    findings.extend(diff_surface(old, new))
    findings.extend(diff_depends_on(old, new))

    o_ec = [_edge_case_text(x) for x in (old.get("edge_cases") or [])]
    n_ec = [_edge_case_text(x) for x in (new.get("edge_cases") or [])]
    findings.extend(diff_edge_cases_multiset(o_ec, n_ec))

    o_inv = list(old.get("invariants") or [])
    n_inv = list(new.get("invariants") or [])
    findings.extend(diff_invariants_indexed(o_inv, n_inv))

    findings.extend(diff_metadata(old, new))

    # Acknowledgement mechanism: a major or minor `since:` bump explicitly
    # acknowledges any breaking changes in this diff. Patch bumps do not
    # count — those are reserved for non-breaking fixes. Reclassify all
    # `breaking` findings to `acknowledged_breaking` so the gate can pass.
    acknowledged = _since_bump_acknowledges(old.get("since"), new.get("since"))
    if acknowledged:
        for f in findings:
            if f["severity"] == "breaking":
                f["severity"] = "acknowledged_breaking"
                f["acknowledged_via"] = {
                    "since_before": old.get("since"),
                    "since_after": new.get("since"),
                }

    breaking = sum(1 for f in findings if f["severity"] == "breaking")
    non_breaking = sum(1 for f in findings if f["severity"] == "non_breaking")
    acknowledged_count = sum(
        1 for f in findings if f["severity"] == "acknowledged_breaking"
    )
    return {
        "schema_version": "1",
        "summary": {
            "breaking_count": breaking,
            "non_breaking_count": non_breaking,
            "acknowledged_breaking_count": acknowledged_count,
            "total": len(findings),
        },
        "findings": findings,
    }


def _since_bump_acknowledges(old_since: Any, new_since: Any) -> bool:
    """Return True if `new_since` is a major or minor bump over `old_since`.

    Patch bumps (e.g. 0.1.0 -> 0.1.1) DO NOT acknowledge breaking changes —
    patch is reserved for fixes. Major (1.0.0 -> 2.0.0) or minor (0.1.0 ->
    0.2.0) bumps do. Pre-1.0 minor bumps are treated as breaking-allowed
    per cargo SemVer convention.
    """
    if not old_since or not new_since or old_since == new_since:
        return False
    try:
        old_parts = [int(x) for x in str(old_since).split(".")[:3]]
        new_parts = [int(x) for x in str(new_since).split(".")[:3]]
    except ValueError:
        return False
    while len(old_parts) < 3:
        old_parts.append(0)
    while len(new_parts) < 3:
        new_parts.append(0)
    # Major bump or minor bump = acknowledged
    if new_parts[0] > old_parts[0]:
        return True
    if new_parts[0] == old_parts[0] and new_parts[1] > old_parts[1]:
        return True
    return False


def format_text(report: dict[str, Any]) -> str:
    summary = report["summary"]
    header = (
        f"breaking: {summary['breaking_count']}  "
        f"non_breaking: {summary['non_breaking_count']}"
    )
    ack = summary.get("acknowledged_breaking_count", 0)
    if ack:
        header += f"  acknowledged_breaking: {ack}"
    lines = [header, ""]
    for f in report["findings"]:
        lines.append(f"[{f['severity']}] {f['category']}: {json.dumps(f['detail'], ensure_ascii=False)}")
    return "\n".join(lines) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser(description="Diff two Chassis contract manifests.")
    ap.add_argument("--old", required=True, help="Path to baseline CONTRACT.yaml")
    ap.add_argument("--new", required=True, help="Path to new CONTRACT.yaml")
    ap.add_argument("--format", choices=("json", "text"), default="json")
    ap.add_argument(
        "--output",
        "-o",
        default="",
        help="Write report to file (default: stdout)",
    )
    ap.add_argument(
        "--fail-on-breaking",
        action="store_true",
        help="Exit 1 when any finding has severity breaking",
    )
    args = ap.parse_args()

    old_p = Path(args.old).resolve()
    new_p = Path(args.new).resolve()
    if not old_p.is_file() or not new_p.is_file():
        print("chassis contract-diff: --old and --new must be files", file=sys.stderr)
        sys.exit(1)

    old_d = load(old_p)
    new_d = load(new_p)
    report = analyze(old_d, new_d)
    report["old_path"] = str(old_p)
    report["new_path"] = str(new_p)

    body = format_text(report) if args.format == "text" else json.dumps(report, indent=2, ensure_ascii=False) + "\n"

    if args.output:
        Path(args.output).write_text(body, encoding="utf-8")
    else:
        sys.stdout.write(body)

    if args.fail_on_breaking and report["summary"]["breaking_count"] > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
