#!/usr/bin/env python3
"""Chassis MCP server (stdio transport) — initial Python implementation.

Milestone B.1. Speaks the Model Context Protocol (MCP) over stdio. A native
Rust implementation using ``rmcp 0.16+`` at ``crates/chassis-mcp/`` is
scheduled as a later optimisation; this Python version is the reference
implementation and is what ``mcp-shim`` (D.5) wraps for Aider.

Verbs (15):
  listCategories            → artifact-kinds summary
  searchByCapability        → entries matching tags
  getDetail                 → full manifest for a kind/name
  getDependents             → reverse graph (manifest-level)
  getCompositionChildren    → forward graph (blueprint children)
  getDiagnostics            → structured diagnostics (release-gate JSON)
  validateManifest          → pass/fail + diagnostics for submitted YAML
  coherenceReport           → coherence JSON for the repo
  generateContext           → tier-aware agent-context payload
  listConventions           → ADRs scoped to a path (glob-matched)
  listExemptions            → current exemption registry slice
  applyFix                  → where fix.applicability=automatic
  listObjectives            → product objectives (post-P0.1)
  describeSchema            → JSON Schema for an artifact kind
  listSkills                → available SKILL.md entries

Transport: newline-delimited JSON-RPC 2.0 over stdio. Every request is met
with a response referencing the request id; parse errors return -32700.
"""
from __future__ import annotations

import argparse
import fnmatch
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

_SCRIPTS_CHASSIS = Path(__file__).resolve().parent
if str(_SCRIPTS_CHASSIS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_CHASSIS))

try:
    import yaml  # type: ignore
except ImportError:
    yaml = None  # type: ignore

import cache
import registry
from repo_layout import repo_root

FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---\n", re.DOTALL)

CHASSIS_CLI = lambda: _CHASSIS_CLI  # set in main()
_CHASSIS_CLI: Path | None = None


def _run_cli(args: list[str]) -> tuple[int, str, str]:
    """Invoke the chassis CLI; return (rc, stdout, stderr)."""
    cmd = [str(_CHASSIS_CLI)] + args
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    return proc.returncode, proc.stdout, proc.stderr


# ---------- Verb implementations -------------------------------------------


def v_list_categories(root: Path, params: dict) -> Any:
    """listCategories → { families: [ {family, kind_count, kinds: [...]} ] }."""
    # Ensure registry is built at least once; serve from on-disk tier 1.
    top = root / ".chassis" / "registry" / "index.json"
    if not top.is_file():
        registry.build(root)
    return json.loads(top.read_text(encoding="utf-8"))


def v_search_by_capability(root: Path, params: dict) -> Any:
    """searchByCapability(tags: [str]) → entries whose description or name matches any tag."""
    tags = [t.lower() for t in params.get("tags", []) or []]
    if not tags:
        return {"matches": []}
    kinds_path = root / "schemas" / "artifact-kinds.json"
    kinds = json.loads(kinds_path.read_text(encoding="utf-8")).get("kinds", [])
    matches = []
    for k in kinds:
        kid = (k.get("id") or "").lower()
        schema_rel = k.get("schema") or ""
        # Lazy-read schema for title/description
        sp = root / schema_rel
        try:
            schema = json.loads(sp.read_text(encoding="utf-8")) if sp.is_file() else {}
        except json.JSONDecodeError:
            schema = {}
        haystack = " ".join([kid, schema.get("title", ""), schema.get("description", "")]).lower()
        if any(t in haystack for t in tags):
            matches.append({
                "kind_id": kid,
                "schema": schema_rel,
                "title": schema.get("title"),
            })
    return {"matches": matches}


def v_get_detail(root: Path, params: dict) -> Any:
    """getDetail(kind, name) → per-schema contents from tier 3."""
    kind = params.get("kind")
    name = params.get("name")
    if not kind or not name:
        raise ValueError("kind and name required")
    path = root / ".chassis" / "registry" / kind / f"{name}.json"
    if not path.is_file():
        registry.build(root)
    if not path.is_file():
        return {"error": f"unknown kind/name: {kind}/{name}"}
    return json.loads(path.read_text(encoding="utf-8"))


def v_get_dependents(root: Path, params: dict) -> Any:
    """getDependents(kind, name) → CONTRACT.yaml files whose depends_on mentions the target."""
    kind = params.get("kind")
    name = params.get("name")
    target = f"{kind}/{name}" if kind and name else (params.get("target") or "")
    if not target or yaml is None:
        return {"dependents": []}
    out = []
    for p in root.rglob("CONTRACT.yaml"):
        try:
            data = yaml.safe_load(p.read_text(encoding="utf-8")) or {}
        except yaml.YAMLError:
            continue
        deps = data.get("depends_on") or []
        if isinstance(deps, list) and target in deps:
            out.append(str(p.relative_to(root)))
    return {"dependents": sorted(out)}


def v_get_composition_children(root: Path, params: dict) -> Any:
    """getCompositionChildren(kind, name) → blueprint children for the named composition."""
    name = params.get("name")
    if not name:
        return {"children": []}
    blueprints_dir = root / "blueprints"
    out = []
    if blueprints_dir.is_dir() and yaml is not None:
        for p in blueprints_dir.rglob("*.yaml"):
            try:
                data = yaml.safe_load(p.read_text(encoding="utf-8")) or {}
            except yaml.YAMLError:
                continue
            if data.get("name") == name or data.get("id") == name:
                out.append({
                    "blueprint": str(p.relative_to(root)),
                    "children": data.get("children", []) or data.get("artifacts", []),
                })
    return {"children": out}


def v_get_diagnostics(root: Path, params: dict) -> Any:
    """getDiagnostics() → release-gate JSON (schema/diagnostics/diagnostic.schema.json shape)."""
    rc, out, err = _run_cli(["release-gate", "--advisory", "--json", "--no-artifact"])
    if "{" not in out:
        return {"error": err.strip() or "no JSON output"}
    return json.loads(out[out.index("{"):])


def v_validate_manifest(root: Path, params: dict) -> Any:
    """validateManifest(kind, yaml) → {ok, diagnostics[]}."""
    yaml_text = params.get("yaml", "")
    if yaml is None:
        return {"ok": False, "diagnostics": [{"message": "PyYAML required"}]}
    if not yaml_text:
        return {"ok": False, "diagnostics": [{"message": "yaml parameter required"}]}
    # Write to a tmp file and call validate on that single file.
    import tempfile
    with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
        f.write(yaml_text)
        tmp_path = f.name
    try:
        # The generic CLI validates by walking the tree; for single-file validation
        # fall back to parsing + Draft7 round-trip via jsonschema_support.
        from jsonschema_support import draft7_validator_for_schema_file
        kind = params.get("kind", "metadata/contract")
        if kind == "metadata/contract":
            schema_path = root / "schemas" / "metadata" / "contract.schema.json"
        elif kind == "metadata/chassis-unit":
            schema_path = root / "schemas" / "metadata" / "chassis-unit.schema.json"
        else:
            return {"ok": False, "diagnostics": [{"message": f"unknown kind {kind}"}]}
        validator = draft7_validator_for_schema_file(schema_path)
        data = yaml.safe_load(yaml_text)
        errs = sorted(validator.iter_errors(data), key=lambda e: list(e.path))
        return {
            "ok": not errs,
            "diagnostics": [
                {"ruleId": "VALIDATE-MANIFEST-GENERIC", "severity": "error", "message": e.message}
                for e in errs
            ],
        }
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def v_coherence_report(root: Path, params: dict) -> Any:
    rc, out, err = _run_cli(["coherence", "--format", "json"])
    if "{" not in out:
        return {"error": err.strip() or "no JSON output"}
    return json.loads(out[out.index("{"):])


def v_generate_context(root: Path, params: dict) -> Any:
    """generateContext(scope) → agent-context JSON (tier-aware)."""
    scope = params.get("scope", "")
    rc, out, err = _run_cli(["agent-context"] + (["--scope", scope] if scope else []))
    if "{" not in out:
        return {"error": err.strip() or "no JSON output"}
    return json.loads(out[out.index("{"):])


def v_list_conventions(root: Path, params: dict) -> Any:
    """listConventions(path?) → ADRs scoped to path (glob-matched against applies_to)."""
    index_path = root / "docs" / "index.json"
    if not index_path.is_file():
        _run_cli(["adr", "index"])
    if not index_path.is_file():
        return {"adrs": []}
    data = json.loads(index_path.read_text(encoding="utf-8"))
    path_filter = params.get("path")
    out = []
    for adr in data.get("adrs", []) or []:
        if path_filter:
            globs = adr.get("applies_to") or []
            if not any(fnmatch.fnmatch(path_filter, g) for g in globs):
                continue
        out.append(adr)
    return {"adrs": out}


def v_list_exemptions(root: Path, params: dict) -> Any:
    """listExemptions(rule?) → current exemption registry, optionally filtered by rule."""
    path = root / ".exemptions" / "registry.yaml"
    if not path.is_file() or yaml is None:
        return {"entries": []}
    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    entries = data.get("entries") or []
    rule = params.get("rule")
    if rule:
        entries = [e for e in entries if e.get("rule") == rule]
    return {"entries": entries}


def v_apply_fix(root: Path, params: dict) -> Any:
    """applyFix(diagnostic_id) → { applied, fix } when fix.applicability=automatic."""
    # The diagnostic_id is opaque; callers typically pass the full diagnostic.
    finding = params.get("finding") or {}
    fix = finding.get("fix") or {}
    if fix.get("applicability") != "automatic":
        return {"applied": False, "reason": "fix not automatically applicable"}
    patch = fix.get("patch")
    if not patch:
        return {"applied": False, "reason": "no patch provided"}
    # Security: never execute the patch; just return it to the caller.
    return {"applied": False, "reason": "auto-apply is not executed server-side; caller must apply the patch", "fix": fix}


def v_list_objectives(root: Path, params: dict) -> Any:
    """listObjectives() → config/chassis.objectives.yaml content."""
    path = root / "config" / "chassis.objectives.yaml"
    if not path.is_file() or yaml is None:
        return {"version": 0, "objectives": []}
    return yaml.safe_load(path.read_text(encoding="utf-8")) or {}


def v_describe_schema(root: Path, params: dict) -> Any:
    """describeSchema(kind) → the JSON Schema document."""
    kind = params.get("kind")
    if not kind:
        return {"error": "kind required"}
    kinds_path = root / "schemas" / "artifact-kinds.json"
    kinds = json.loads(kinds_path.read_text(encoding="utf-8")).get("kinds", [])
    for k in kinds:
        if k.get("id") == kind:
            schema_rel = k.get("schema", "")
            p = root / schema_rel
            if p.is_file():
                return json.loads(p.read_text(encoding="utf-8"))
    return {"error": f"unknown kind {kind}"}


def v_list_skills(root: Path, params: dict) -> Any:
    """listSkills() → skills/<name>/SKILL.md frontmatter entries."""
    skills_dir = root / "skills"
    out = []
    if not skills_dir.is_dir() or yaml is None:
        return {"skills": []}
    for p in sorted(skills_dir.rglob("SKILL.md")):
        m = FRONTMATTER_RE.match(p.read_text(encoding="utf-8"))
        if not m:
            continue
        fm = yaml.safe_load(m.group(1)) or {}
        out.append({
            "name": fm.get("name"),
            "description": fm.get("description"),
            "triggers": fm.get("triggers", []),
            "file": str(p.relative_to(root)),
        })
    return {"skills": out}


VERBS: dict[str, Any] = {
    "listCategories": v_list_categories,
    "searchByCapability": v_search_by_capability,
    "getDetail": v_get_detail,
    "getDependents": v_get_dependents,
    "getCompositionChildren": v_get_composition_children,
    "getDiagnostics": v_get_diagnostics,
    "validateManifest": v_validate_manifest,
    "coherenceReport": v_coherence_report,
    "generateContext": v_generate_context,
    "listConventions": v_list_conventions,
    "listExemptions": v_list_exemptions,
    "applyFix": v_apply_fix,
    "listObjectives": v_list_objectives,
    "describeSchema": v_describe_schema,
    "listSkills": v_list_skills,
}


def handle(root: Path, msg: dict) -> dict:
    """Dispatch a single JSON-RPC request."""
    req_id = msg.get("id")
    method = msg.get("method")
    params = msg.get("params") or {}
    if method == "initialize":
        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "serverInfo": {"name": "chassis-mcp", "version": "0.1.0"},
                "capabilities": {"verbs": sorted(VERBS.keys())},
            },
        }
    if method == "tools/list":
        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": [
                    {"name": name, "description": (fn.__doc__ or "").split("\n")[0]}
                    for name, fn in VERBS.items()
                ]
            },
        }
    if method == "tools/call":
        verb_name = params.get("name")
        verb = VERBS.get(verb_name)
        if verb is None:
            return {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32601, "message": f"unknown verb {verb_name}"}}
        try:
            result = verb(root, params.get("arguments") or {})
            return {"jsonrpc": "2.0", "id": req_id, "result": result}
        except Exception as exc:  # pragma: no cover — defensive
            return {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32000, "message": str(exc)}}
    # Direct verb call (non-MCP): useful for the mcp-shim CLI.
    if method in VERBS:
        try:
            return {"jsonrpc": "2.0", "id": req_id, "result": VERBS[method](root, params)}
        except Exception as exc:
            return {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32000, "message": str(exc)}}
    return {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32601, "message": f"method not found: {method}"}}


def main() -> int:
    parser = argparse.ArgumentParser(prog="chassis-mcp-server")
    parser.add_argument("--repo-root", default=str(repo_root()))
    parser.add_argument("--oneshot", help="Instead of stdio loop, run a single verb and exit", default=None)
    parser.add_argument("--params", help="JSON params for --oneshot (default: {})", default="{}")
    args = parser.parse_args()
    root = Path(args.repo_root).resolve()
    global _CHASSIS_CLI
    _CHASSIS_CLI = root / "scripts" / "chassis" / "chassis"

    if args.oneshot is not None:
        req = {"jsonrpc": "2.0", "id": 1, "method": args.oneshot, "params": json.loads(args.params)}
        resp = handle(root, req)
        print(json.dumps(resp, indent=2))
        return 0 if "result" in resp else 1

    # stdio loop
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "Parse error"}}) + "\n")
            sys.stdout.flush()
            continue
        resp = handle(root, msg)
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
