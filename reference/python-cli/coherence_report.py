#!/usr/bin/env python3
"""
Repository coherence report: authority alignment, stale CONTRACT metadata, structured validate findings,
trust ladder, and bounded-autonomy hints for agent integrations.

Runtime evidence batch paths: canonical list is top-level ``runtime_evidence_sources``.
``summary.runtime_evidence_sources`` and ``summary.observed_truth.runtime_evidence_sources`` are
synchronized mirrors of that value for legacy consumers (see schema + guides).
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from collections import defaultdict
from typing import Any, Dict, List, Optional, Set, Tuple, cast

_TOOLS_DIR = Path(__file__).resolve().parent
if str(_TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(_TOOLS_DIR))

try:
    import yaml  # type: ignore
except ImportError:
    yaml = None  # type: ignore

from jsonschema import Draft7Validator

from manifest_freshness import (
    any_source_newer_than_manifest,
    ir_graphs_newer_than_manifest,
    stale_reasons_for_manifest,
    workflows_newer_than_manifest,
)
from validate_metadata import SKIP_DIR_PARTS, collect_metadata_findings, _load_yaml, _walk_manifests

from authority_index import evaluate_entries, load_raw_authority_index

try:
    from edit_policy import load_policy as load_architecture_policy_doc
except ImportError:  # pragma: no cover
    load_architecture_policy_doc = None  # type: ignore[assignment]

_SOURCE_SUFFIXES = (".rs", ".ts", ".tsx", ".py", ".cs")
_MANIFEST_NAMES = frozenset({"CONTRACT.yaml", "chassis.unit.yaml"})


def _count_contract_manifests(root: Path) -> int:
    """Count CONTRACT.yaml files (excluding tooling noise) for observed_truth coverage ratio."""
    n = 0
    for p in root.rglob("CONTRACT.yaml"):
        if any(
            x in p.parts
            for x in ("node_modules", "target", ".git", "dist", "fixtures", "templates")
        ):
            continue
        n += 1
    return max(n, 1)


_LADDER_ORDER = ["declared", "coherent", "verified", "enforced", "observed"]


def _compute_assurance_ladder(root: Path) -> Dict[str, Any]:
    """Roll up CONTRACT.yaml assurance_level fields across the tree.

    Returns the shape declared in schemas/coherence/repository-coherence-report.schema.json
    (summary.assurance_ladder): counts per tier + `unset` for contracts that declare no
    level, plus `lowest` = the floor of the ladder across the tree.
    """
    counts: Dict[str, int] = {t: 0 for t in _LADDER_ORDER}
    counts["unset"] = 0
    # Lazy-import yaml to avoid hard dependency in environments that don't need it.
    try:
        import yaml  # type: ignore
    except ImportError:
        # Without yaml we can't parse assurance_level; return a conservative result.
        return {**counts, "lowest": "unset"}
    for p in root.rglob("CONTRACT.yaml"):
        if any(
            x in p.parts
            for x in ("node_modules", "target", ".git", "dist", "fixtures", "templates")
        ):
            continue
        try:
            data = yaml.safe_load(p.read_text(encoding="utf-8")) or {}
        except yaml.YAMLError:
            continue
        level = data.get("assurance_level")
        if level in counts:
            counts[level] += 1
        else:
            counts["unset"] += 1
    # Lowest observed tier: first in ladder order that has a non-zero count;
    # unset wins if any contract is unset, as per the schema description.
    if counts["unset"] > 0:
        lowest = "unset"
    else:
        lowest = next((t for t in _LADDER_ORDER if counts[t] > 0), "unset")
    return {**counts, "lowest": lowest}


def _repo_root() -> Path:
    # Delegate to the shared resolver so CHASSIS_TARGET_REPO_ROOT / --repo-root
    # are honored uniformly. CHASSIS_REPO_ROOT remains a legacy fallback.
    from repo_layout import target_repo_root

    return target_repo_root()


def _assets_root() -> Path:
    from repo_layout import chassis_assets_root

    return chassis_assets_root()


def _rel(root: Path, path: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def _locate_asset(relpath: str, target_root: Path) -> Optional[Path]:
    """Try target_root first, fall back to the Chassis assets root. ``None`` if neither has it."""
    candidate = target_root / relpath
    if candidate.is_file():
        return candidate
    assets_candidate = _assets_root() / relpath
    if assets_candidate.is_file():
        return assets_candidate
    return None


def load_trust_ladder(root: Path) -> List[Dict[str, Any]]:
    path = _locate_asset("schemas/coherence/trust-ladder.data.json", root)
    if path is None:
        raise FileNotFoundError(
            "Missing schemas/coherence/trust-ladder.data.json under repository root or CHASSIS_ROOT"
        )
    return cast(List[Dict[str, Any]], json.loads(path.read_text(encoding="utf-8")))


def extended_autonomy_hints(root: Path) -> List[Dict[str, str]]:
    hints = default_autonomy_hints()
    if load_architecture_policy_doc:
        doc = load_architecture_policy_doc(root, None)
        if isinstance(doc, dict) and doc.get("policyId"):
            rs = doc.get("rules") if isinstance(doc.get("rules"), dict) else {}
            kinds = rs.get("migrationSensitiveUnitKinds") or []
            ks = ", ".join(str(x) for x in kinds) or "—"
            hints.append(
                {
                    "when": "architecture_policy_active",
                    "hint": (
                        f"Edit policy {doc.get('policyId')!r} loaded; "
                        f"migration-sensitive unit kinds: {ks}."
                    ),
                }
            )
    return hints


def default_autonomy_hints() -> List[Dict[str, str]]:
    return [
        {
            "when": "any_error",
            "hint": "Do not merge or deploy until schema and authority errors are cleared or explicitly waived.",
        },
        {
            "when": "metadata_stale_warning",
            "hint": (
                "CONTRACT may be behind implementation or IR (mtime heuristic — advisory). "
                "Refresh CONTRACT.yaml if behavior changed; support-only paths (benches/fuzz/tests) "
                "do not trigger this by default."
            ),
        },
        {
            "when": "authority_warning",
            "hint": "Reconcile unit status with parent CONTRACT.yaml; treat mismatch as coherence debt.",
        },
    ]


def classify_status_pair(parent: str, unit: str) -> str:
    parent = (parent or "").strip()
    unit = (unit or "").strip()
    if not unit:
        return "warn:mismatch"
    if unit == parent:
        return "ok"
    if parent == "stable" and unit == "experimental":
        return "warn:not_promoted"
    if parent == "stable" and unit == "draft":
        return "warn:behind"
    if parent == "stable" and unit == "deprecated":
        return "err:stable_parent_deprecated_unit"
    if parent == "deprecated" and unit == "stable":
        return "err:deprecated_parent_stable_unit"
    if parent == "draft" and unit == "stable":
        return "warn:unit_ahead"
    if parent == "experimental" and unit == "stable":
        return "warn:unit_ahead"
    return "warn:mismatch"


def _purpose_nonempty(doc: Any) -> bool:
    if not isinstance(doc, dict):
        return False
    p = doc.get("purpose")
    if p is None:
        return False
    return bool(str(p).strip())


def find_parent_contract(start_dir: Path, root: Path) -> Optional[Path]:
    walk = start_dir.resolve()
    root_r = root.resolve()
    for _ in range(256):
        cand = walk / "CONTRACT.yaml"
        if cand.is_file():
            return cand
        if walk == root_r.parent or walk.parent == walk:
            break
        walk = walk.parent
    return None


def _walk_unit_files(root: Path) -> List[Path]:
    out: List[Path] = []
    for dirpath, dirnames, filenames in os.walk(root):
        parts = Path(dirpath).parts
        if any(p in SKIP_DIR_PARTS for p in parts):
            dirnames[:] = []
            continue
        rel = Path(dirpath).relative_to(root)
        if len(rel.parts) >= 1 and rel.parts[0] == "templates":
            dirnames[:] = []
            continue
        if "tools" in rel.parts and "templates" in rel.parts:
            dirnames[:] = []
            continue
        if "chassis.unit.yaml" not in filenames:
            continue
        out.append(Path(dirpath) / "chassis.unit.yaml")
    return sorted(out, key=lambda p: str(p))


def collect_authority_findings(root: Path) -> List[Dict[str, Any]]:
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return [
            {
                "id": "authority:pyyaml-missing",
                "category": "authority_alignment",
                "severity": "error",
                "message": "PyYAML required for coherence authority checks (pip install pyyaml).",
                "trust_rank_hint": 1,
            }
        ]

    for unit_path in _walk_unit_files(root):
        rel_unit = _rel(root, unit_path)
        unit_dir = unit_path.parent
        try:
            unit_doc = _load_yaml(unit_path)
        except Exception as e:  # noqa: BLE001
            findings.append(
                {
                    "id": f"authority:yaml:{rel_unit}",
                    "category": "authority_alignment",
                    "severity": "error",
                    "message": f"Cannot load unit manifest: {e}",
                    "path": rel_unit,
                    "trust_rank_hint": 1,
                }
            )
            continue

        parent = find_parent_contract(unit_dir, root)
        if parent is None:
            findings.append(
                {
                    "id": f"authority:no-parent:{rel_unit}",
                    "category": "authority_missing_parent",
                    "severity": "warning",
                    "message": f"No parent CONTRACT.yaml above { _rel(root, unit_dir) }",
                    "path": rel_unit,
                    "trust_rank_hint": 1,
                }
            )
            continue

        rel_parent_c = _rel(root, parent)
        try:
            parent_doc = _load_yaml(parent)
        except Exception as e:  # noqa: BLE001
            findings.append(
                {
                    "id": f"authority:parent-yaml:{rel_unit}",
                    "category": "authority_alignment",
                    "severity": "error",
                    "message": f"Cannot load parent contract {rel_parent_c}: {e}",
                    "path": rel_unit,
                    "related_paths": [rel_parent_c],
                    "trust_rank_hint": 1,
                }
            )
            continue

        if not _purpose_nonempty(parent_doc):
            findings.append(
                {
                    "id": f"authority:parent-purpose:{rel_unit}",
                    "category": "authority_alignment",
                    "severity": "warning",
                    "message": f"Parent CONTRACT purpose empty or missing ({rel_parent_c})",
                    "path": rel_unit,
                    "related_paths": [rel_parent_c],
                    "code": "empty_parent_purpose",
                    "trust_rank_hint": 1,
                }
            )

        if not _purpose_nonempty(unit_doc):
            name = str(unit_doc.get("name", "")).strip() if isinstance(unit_doc, dict) else ""
            if not name:
                name = unit_dir.name
            findings.append(
                {
                    "id": f"authority:unit-purpose:{rel_unit}",
                    "category": "authority_alignment",
                    "severity": "warning",
                    "message": f"{name}: purpose empty or missing",
                    "path": rel_unit,
                    "related_paths": [rel_parent_c],
                    "code": "empty_unit_purpose",
                    "trust_rank_hint": 1,
                }
            )

        parent_status = ""
        unit_status = ""
        if isinstance(parent_doc, dict):
            parent_status = str(parent_doc.get("status", "") or "").strip()
        if isinstance(unit_doc, dict):
            unit_status = str(unit_doc.get("status", "") or "").strip()

        if not parent_status:
            findings.append(
                {
                    "id": f"authority:parent-status:{rel_unit}",
                    "category": "authority_alignment",
                    "severity": "warning",
                    "message": f"Parent CONTRACT status missing or empty ({rel_parent_c})",
                    "path": rel_unit,
                    "related_paths": [rel_parent_c],
                    "trust_rank_hint": 1,
                }
            )
            continue

        if not unit_status:
            findings.append(
                {
                    "id": f"authority:unit-status:{rel_unit}",
                    "category": "authority_alignment",
                    "severity": "warning",
                    "message": "Unit status missing or empty",
                    "path": rel_unit,
                    "related_paths": [rel_parent_c],
                    "trust_rank_hint": 1,
                }
            )
            continue

        cl = classify_status_pair(parent_status, unit_status)
        unit_name = str(unit_doc.get("name", "")).strip() if isinstance(unit_doc, dict) else ""
        if not unit_name:
            unit_name = unit_dir.name

        if cl == "ok":
            continue
        if cl.startswith("err:"):
            findings.append(
                {
                    "id": f"authority:{cl}:{rel_unit}",
                    "category": "authority_alignment",
                    "severity": "error",
                    "message": f"{unit_name}: {unit_status} vs parent {parent_status} ({cl})",
                    "path": rel_unit,
                    "related_paths": [rel_parent_c],
                    "code": cl,
                    "trust_rank_hint": 1,
                }
            )
        else:
            findings.append(
                {
                    "id": f"authority:{cl}:{rel_unit}",
                    "category": "authority_alignment",
                    "severity": "warning",
                    "message": f"{unit_name}: {unit_status} vs parent {parent_status} ({cl})",
                    "path": rel_unit,
                    "related_paths": [rel_parent_c],
                    "code": cl,
                    "trust_rank_hint": 1,
                }
            )

    return findings


def _iter_architecture_graph_paths(root: Path, *, include_fixtures: bool) -> List[Path]:
    try:
        from repository_topology import iter_architecture_graph_files

        return iter_architecture_graph_files(root, include_fixtures=include_fixtures)
    except ImportError:
        return sorted(root.rglob("*.architecture-graph.json"))


def collect_architecture_ir_findings(root: Path) -> List[Dict[str, Any]]:
    """Duplicate manifestPath, missing manifestPath targets, unknown unit refs in relationships."""
    findings: List[Dict[str, Any]] = []
    global_claims: Dict[str, List[Tuple[str, str]]] = {}

    for gp in _iter_architecture_graph_paths(root, include_fixtures=True):
        rel_g = _rel(root, gp)
        try:
            doc = json.loads(gp.read_text(encoding="utf-8"))
        except Exception as e:  # noqa: BLE001
            findings.append(
                {
                    "id": f"ir:json:{rel_g}",
                    "category": "schema_invalid",
                    "severity": "warning",
                    "message": f"Cannot parse architecture graph JSON: {e}",
                    "path": rel_g,
                    "trust_rank_hint": 1,
                }
            )
            continue
        if not isinstance(doc, dict):
            continue
        units = doc.get("units") or []
        unit_ids: Set[str] = set()
        if isinstance(units, list):
            for u in units:
                if isinstance(u, dict):
                    uid = str(u.get("id", "")).strip()
                    if uid:
                        unit_ids.add(uid)

        mp_to_units: Dict[str, List[str]] = {}
        if isinstance(units, list):
            for u in units:
                if not isinstance(u, dict):
                    continue
                uid = str(u.get("id", "")).strip()
                mp = u.get("manifestPath")
                if not isinstance(mp, str) or not mp.strip():
                    continue
                mpn = mp.strip().replace("\\", "/")
                mp_to_units.setdefault(mpn, []).append(uid or "?")
                if not (root / mpn).is_file():
                    findings.append(
                        {
                            "id": f"ir:missing-manifest-path:{uid}:{rel_g}",
                            "category": "missing_architecture_link",
                            "severity": "warning",
                            "message": f"Architecture unit {uid!r} manifestPath not found: {mpn}",
                            "path": rel_g,
                            "related_paths": [mpn],
                            "trust_rank_hint": 1,
                        }
                    )
        for mp, uids in sorted(mp_to_units.items()):
            uniq = sorted({u for u in uids if u})
            if len(uniq) > 1:
                findings.append(
                    {
                        "id": f"ir:dup-manifest:{mp}",
                        "category": "semantic_contradiction",
                        "severity": "error",
                        "message": (
                            f"Multiple architecture units claim the same manifestPath {mp!r}: "
                            f"{', '.join(uniq)}"
                        ),
                        "path": rel_g,
                        "related_paths": [mp],
                        "code": "duplicate_manifest_path_claim",
                        "trust_rank_hint": 0,
                    }
                )

        for mp, uids in mp_to_units.items():
            for uid in uids:
                global_claims.setdefault(mp, []).append((rel_g, uid))

        rels = doc.get("relationships") or []
        if isinstance(rels, list):
            for r in rels:
                if not isinstance(r, dict):
                    continue
                fr = str(r.get("from", "")).strip()
                to = str(r.get("to", "")).strip()
                if fr and fr not in unit_ids:
                    findings.append(
                        {
                            "id": f"ir:unknown-from:{fr}:{rel_g}",
                            "category": "ir_graph_integrity",
                            "severity": "error",
                            "message": f"Relationship references unknown unit id (from): {fr!r}",
                            "path": rel_g,
                            "trust_rank_hint": 1,
                        }
                    )
                if to and to not in unit_ids:
                    findings.append(
                        {
                            "id": f"ir:unknown-to:{to}:{rel_g}",
                            "category": "ir_graph_integrity",
                            "severity": "error",
                            "message": f"Relationship references unknown unit id (to): {to!r}",
                            "path": rel_g,
                            "trust_rank_hint": 1,
                        }
                    )

    for mp, pairs in sorted(global_claims.items()):
        uids = sorted({p[1] for p in pairs if p[1] and p[1] != "?"})
        graphs = sorted({p[0] for p in pairs})
        if len(uids) > 1 and len(graphs) > 1:
            findings.append(
                {
                    "id": f"ir:cross-graph-dup-manifest:{mp}",
                    "category": "semantic_contradiction",
                    "severity": "error",
                    "message": (
                        f"Manifest path {mp!r} claimed by different units across IR files: "
                        f"{', '.join(uids)} in {', '.join(graphs)}"
                    ),
                    "path": graphs[0],
                    "related_paths": graphs + [mp],
                    "code": "cross_graph_manifest_path_conflict",
                    "trust_rank_hint": 0,
                }
            )

    return findings


def collect_drift_findings(root: Path) -> List[Dict[str, Any]]:
    """Surface drift engine findings (export surface, coverage hints, doc freshness) as coherence rows."""
    findings: List[Dict[str, Any]] = []
    engine = root / "tools" / "drift-detection" / "engine.py"
    if not engine.is_file():
        return findings
    out_path = ""
    raw: Optional[Dict[str, Any]] = None
    try:
        fd, out_path = tempfile.mkstemp(suffix=".json")
        os.close(fd)
        subprocess.run(
            [
                sys.executable,
                str(engine),
                "drift",
                "--repo-root",
                str(root),
                "--json-out",
                out_path,
                "--quiet",
            ],
            cwd=str(root),
            check=False,
            capture_output=True,
            text=True,
            timeout=180,
        )
        raw = json.loads(Path(out_path).read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001
        return [
            {
                "id": "drift:engine-error",
                "category": "drift_finding",
                "severity": "warning",
                "message": f"Drift engine did not produce JSON: {e}",
                "trust_rank_hint": 2,
            }
        ]
    finally:
        if out_path:
            Path(out_path).unlink(missing_ok=True)

    if not isinstance(raw, dict):
        return findings

    cap = 200
    for mod in raw.get("modules") or []:
        if not isinstance(mod, dict):
            continue
        mpath = str(mod.get("manifest") or "")
        for i, f in enumerate(mod.get("findings") or []):
            if len(findings) >= cap:
                break
            if not isinstance(f, dict):
                continue
            sev = str(f.get("severity") or "info")
            if sev not in ("error", "warning", "info"):
                sev = "info"
            cat = str(f.get("category") or "drift")
            msg = str(f.get("message") or "")
            code = str(f.get("code") or "")
            row: Dict[str, Any] = {
                "id": f"drift:{mpath}:{i}:{code}"[:120],
                "category": "drift_finding",
                "severity": sev,
                "message": f"[{cat}] {msg}"[:2000],
                "trust_rank_hint": 2,
            }
            if mpath:
                row["path"] = mpath
            if code:
                row["code"] = code
            ad = mod.get("adapter") if isinstance(mod.get("adapter"), dict) else {}
            if ad:
                row["drift_adapter"] = {
                    "language": ad.get("language"),
                    "status": ad.get("status"),
                    "coverage_confidence": ad.get("coverage_confidence"),
                }
            findings.append(row)
        if len(findings) >= cap:
            break
    return findings


def collect_inferred_truth_gap_findings(root: Path) -> List[Dict[str, Any]]:
    """Surfaces inferred brownfield placeholders explicitly (low authority)."""
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings
    for unit_path in _walk_unit_files(root):
        try:
            unit_doc = _load_yaml(unit_path)
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(unit_doc, dict):
            continue
        if str(unit_doc.get("status", "")).strip() != "inferred":
            continue
        rel_u = _rel(root, unit_path)
        findings.append(
            {
                "id": f"inferred:unit:{rel_u}",
                "category": "inferred_truth_gap",
                "severity": "info",
                "message": "Unit manifest is status=inferred — treat as unverified until promoted.",
                "path": rel_u,
                "trust_rank_hint": 2,
            }
        )
    return findings


def collect_structured_debt_findings(root: Path) -> List[Dict[str, Any]]:
    """Surface CONTRACT debt items that block verification or are critical + open."""
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings
    for path in sorted(_walk_manifests(root), key=lambda p: str(p)):
        if path.name != "CONTRACT.yaml":
            continue
        try:
            doc = _load_yaml(path)
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(doc, dict):
            continue
        debt = doc.get("debt")
        if not isinstance(debt, list):
            continue
        rel = _rel(root, path)
        for j, item in enumerate(debt):
            if not isinstance(item, dict):
                continue
            did = str(item.get("id", "") or f"item-{j}")
            desc = str(item.get("description", ""))[:240]
            stage = str(item.get("promotion_stage", "open"))
            sev = str(item.get("severity", ""))
            if item.get("blocks_verify") is True and stage in ("open", "in_review"):
                findings.append(
                    {
                        "id": f"debt:block:{rel}:{did}"[:120],
                        "category": "structured_debt",
                        "severity": "warning",
                        "message": f"Debt {did!r} blocks_verify while not resolved: {desc}",
                        "path": rel,
                        "code": "debt_blocks_verify",
                        "trust_rank_hint": 2,
                    }
                )
            elif sev == "critical" and stage == "open":
                findings.append(
                    {
                        "id": f"debt:critical:{rel}:{did}"[:120],
                        "category": "structured_debt",
                        "severity": "warning",
                        "message": f"Critical open debt {did!r}: {desc}",
                        "path": rel,
                        "code": "debt_critical_open",
                        "trust_rank_hint": 2,
                    }
                )
    return findings


def collect_ir_duplicate_title_findings(root: Path) -> List[Dict[str, Any]]:
    """Two units in the same graph with the same normalized title (likely duplicate responsibility)."""
    findings: List[Dict[str, Any]] = []
    for gp in _iter_architecture_graph_paths(root, include_fixtures=False):
        rel_g = _rel(root, gp)
        try:
            doc = json.loads(gp.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(doc, dict):
            continue
        buckets: Dict[str, List[str]] = {}
        for u in doc.get("units") or []:
            if not isinstance(u, dict):
                continue
            uid = str(u.get("id", "")).strip()
            title = str(u.get("title", "")).strip().lower()
            if not uid or not title:
                continue
            buckets.setdefault(title, []).append(uid)
        for title, uids in buckets.items():
            uniq = sorted(set(uids))
            if len(uniq) > 1:
                findings.append(
                    {
                        "id": f"ir:dup-title:{rel_g}:{title[:40]}",
                        "category": "semantic_contradiction",
                        "severity": "warning",
                        "message": (
                            f"Duplicate normalized architecture title {title!r} across units "
                            f"{', '.join(uniq)} — clarify responsibility ownership."
                        ),
                        "path": rel_g,
                        "code": "duplicate_responsibility_title",
                        "trust_rank_hint": 1,
                    }
                )
    return findings


def collect_manifest_lifecycle_contradictions(root: Path) -> List[Dict[str, Any]]:
    """Typed contradictions: lifecycle / prose signals that disagree with declared status."""
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings
    for path in sorted(_walk_manifests(root), key=lambda p: str(p)):
        if path.name != "CONTRACT.yaml":
            continue
        try:
            doc = _load_yaml(path)
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(doc, dict):
            continue
        rel = _rel(root, path)
        st = str(doc.get("status", "")).strip().lower()
        purpose = str(doc.get("purpose", "")).lower()
        if st == "stable" and ("[inferred" in purpose or "inferred — verify" in purpose):
            findings.append(
                {
                    "id": f"contradiction:stable-inferred-purpose:{rel}",
                    "category": "semantic_contradiction",
                    "severity": "warning",
                    "message": "CONTRACT status is stable but purpose still contains inferred / verify boilerplate.",
                    "path": rel,
                    "code": "stable_contract_inferred_purpose_language",
                    "trust_rank_hint": 2,
                }
            )
        tags = doc.get("tags") or []
        tag_l = [str(t).lower() for t in tags] if isinstance(tags, list) else []
        kind = str(doc.get("kind", "")).strip().lower()
        if kind == "library" and any("microservice" in t or "service-mesh" in t for t in tag_l):
            findings.append(
                {
                    "id": f"contradiction:kind-vs-tags:{rel}",
                    "category": "semantic_contradiction",
                    "severity": "info",
                    "message": "CONTRACT kind is library but tags suggest distributed service vocabulary — clarify boundary.",
                    "path": rel,
                    "code": "kind_tag_service_language_mismatch",
                    "trust_rank_hint": 2,
                }
            )
    return findings


_IR_KIND_TO_CONTRACT_KINDS: Dict[str, Set[str]] = {
    "domain": {"package", "module", "application", "library", "server", "service"},
    "feature": {"module", "component", "library", "package", "hook", "utility", "tooling"},
    "service": {"service", "application", "server", "module", "library"},
    "adapter": {"adapter", "library", "module", "provider"},
    "state_owner": {"store", "entity", "service", "module"},
    "slot_provider": {"provider", "module", "library", "package"},
    "slot_consumer": {"module", "component", "library", "hook", "package"},
    "workflow": {"tooling", "module", "op", "utility"},
}


def _workflow_step_unit_ids(steps: Any) -> List[str]:
    out: List[str] = []
    if not isinstance(steps, list):
        return out
    for s in steps:
        if not isinstance(s, dict):
            continue
        uid = str(s.get("unitId") or "").strip()
        if uid:
            out.append(uid)
    return out


def _rel_graph_has_edge_between(a: str, b: str, rels: List[Any]) -> bool:
    for r in rels:
        if not isinstance(r, dict):
            continue
        fr = str(r.get("from", "")).strip()
        to = str(r.get("to", "")).strip()
        if not fr or not to:
            continue
        if (fr == a and to == b) or (fr == b and to == a):
            return True
    return False


def load_runtime_evidence_context(root: Path) -> Dict[str, Any]:
    """
    Parse optional runtime evidence batch(es) for declared_unobserved refs and observation counts.
    Used to tighten coherence severity and summary.observed_truth.

    Report shape: the canonical list of repo-relative batch file paths is top-level
    ``runtime_evidence_sources``. The same list is mirrored under ``summary`` for backward
    compatibility (always kept in sync).
    """
    declared_unobserved: Set[str] = set()
    observation_count = 0
    sources: List[str] = []
    schema_path = _locate_asset(
        "schemas/architecture/runtime-evidence-batch.schema.json", root
    )
    batch_candidates = [
        root / ".chassis" / "runtime-evidence.batch.json",
        root / ".chassis" / "observed.runtime-evidence.batch.json",
    ]
    seen: Set[str] = set()
    for batch_path in batch_candidates:
        if not batch_path.is_file():
            continue
        key = str(batch_path.resolve())
        if key in seen:
            continue
        seen.add(key)
        rel_pb = batch_path.relative_to(root).as_posix()
        try:
            doc = json.loads(batch_path.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        obs = doc.get("observations") if isinstance(doc, dict) else []
        if not isinstance(obs, list):
            continue
        sources.append(rel_pb)
        for o in obs:
            if not isinstance(o, dict):
                continue
            observation_count += 1
            if str(o.get("kind", "")) == "declared_unobserved":
                ref = str(o.get("ref") or "").strip()
                if ref:
                    declared_unobserved.add(ref)
    # Legacy file
    leg = root / ".chassis" / "runtime-evidence.json"
    if leg.is_file() and str(leg.resolve()) not in seen:
        try:
            doc = json.loads(leg.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            doc = {}
        obs = doc.get("observations") if isinstance(doc, dict) else []
        if isinstance(obs, list):
            sources.append(".chassis/runtime-evidence.json")
            for o in obs:
                if not isinstance(o, dict):
                    continue
                observation_count += 1
                if str(o.get("kind", "")) == "declared_unobserved":
                    ref = str(o.get("ref") or "").strip()
                    if ref:
                        declared_unobserved.add(ref)
    # Build per-ref counts and confidence for runtime authority enrichment
    observations: List[Dict[str, Any]] = []
    ref_counts: Dict[str, int] = {}
    ref_confidence: Dict[str, str] = {}
    # Re-scan all collected sources for per-ref data
    for batch_path_2 in batch_candidates:
        if not batch_path_2.is_file():
            continue
        try:
            doc2 = json.loads(batch_path_2.read_text(encoding="utf-8"))
        except Exception:
            continue
        obs2 = doc2.get("observations") if isinstance(doc2, dict) else []
        if not isinstance(obs2, list):
            continue
        for o in obs2:
            if not isinstance(o, dict):
                continue
            observations.append(o)
            ref = str(o.get("ref") or "").strip()
            count = int(o.get("count", 1)) if isinstance(o.get("count"), (int, float)) else 1
            conf = str(o.get("confidence", "low"))
            if ref:
                ref_counts[ref] = ref_counts.get(ref, 0) + count
                conf_rank = {"high": 3, "medium": 2, "low": 1}
                if conf_rank.get(conf, 0) > conf_rank.get(ref_confidence.get(ref, ""), 0):
                    ref_confidence[ref] = conf

    return {
        "declared_unobserved": declared_unobserved,
        "observation_count": observation_count,
        "observations": observations,
        "ref_counts": ref_counts,
        "ref_confidence": ref_confidence,
        "sources": sources,
        "schema_path": str(schema_path) if schema_path is not None else "",
    }


def collect_ir_semantic_contradictions(root: Path) -> List[Dict[str, Any]]:
    """
    Typed semantic checks: IR unit kind vs linked CONTRACT kind, workflow adjacency vs relationships,
    state_owner I/O edges, slot requires vs fills.
    """
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings

    for gp in _iter_architecture_graph_paths(root, include_fixtures=False):
        rel_g = _rel(root, gp)
        try:
            doc = json.loads(gp.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(doc, dict):
            continue
        units = doc.get("units") or []
        rels = doc.get("relationships") or []
        if not isinstance(units, list):
            units = []
        if not isinstance(rels, list):
            rels = []

        unit_by_id: Dict[str, Dict[str, Any]] = {}
        for u in units:
            if not isinstance(u, dict):
                continue
            uid = str(u.get("id", "")).strip()
            if uid:
                unit_by_id[uid] = u

        for u in units:
            if not isinstance(u, dict):
                continue
            uid = str(u.get("id", "")).strip()
            uk = str(u.get("kind", "")).strip()
            mp = u.get("manifestPath")
            if not isinstance(mp, str) or not mp.strip():
                continue
            mpn = mp.strip().replace("\\", "/")
            cf = root / mpn
            if not cf.is_file():
                continue
            try:
                cdoc = _load_yaml(cf)
            except Exception:  # noqa: BLE001
                continue
            if not isinstance(cdoc, dict):
                continue
            ck = str(cdoc.get("kind", "")).strip().lower()
            expected = _IR_KIND_TO_CONTRACT_KINDS.get(uk)
            if expected is not None and ck and ck not in expected:
                findings.append(
                    {
                        "id": f"ir:kind-vs-contract:{uid}:{rel_g}",
                        "category": "semantic_contradiction",
                        "severity": "warning",
                        "message": (
                            f"Architecture unit {uid!r} kind={uk!r} but linked manifest {mpn!r} "
                            f"has kind={ck!r} (expected one of {sorted(expected)})."
                        ),
                        "path": rel_g,
                        "related_paths": [mpn],
                        "code": "ir_unit_kind_vs_contract_kind",
                        "trust_rank_hint": 2,
                    }
                )
            desc_l = str(u.get("description") or "").lower()
            purpose_l = str(cdoc.get("purpose") or "").lower()
            if uk == "service" and "library" in purpose_l and "api" not in purpose_l and "http" not in purpose_l:
                if "library" in desc_l or "sdk" in desc_l:
                    findings.append(
                        {
                            "id": f"ir:purpose-tension:{uid}:{rel_g}",
                            "category": "semantic_contradiction",
                            "severity": "info",
                            "message": (
                                f"IR unit {uid!r} is kind service but linked purpose reads like a library/SDK — "
                                "align architecture kind or CONTRACT classification."
                            ),
                            "path": rel_g,
                            "related_paths": [mpn],
                            "code": "ir_service_vs_library_purpose_language",
                            "trust_rank_hint": 3,
                        }
                    )

            if uk == "state_owner":
                touched = False
                for r in rels:
                    if not isinstance(r, dict):
                        continue
                    k = str(r.get("kind", "")).strip()
                    if k not in ("reads_state", "writes_state", "owns_state"):
                        continue
                    fr = str(r.get("from", "")).strip()
                    to = str(r.get("to", "")).strip()
                    if uid in (fr, to):
                        touched = True
                        break
                if not touched:
                    findings.append(
                        {
                            "id": f"ir:state-owner-io:{uid}:{rel_g}",
                            "category": "semantic_contradiction",
                            "severity": "warning",
                            "message": (
                                f"Architecture unit {uid!r} is state_owner but has no reads_state/writes_state/owns_state "
                                "relationship edge involving this id — declare readers/writers or adjust kind."
                            ),
                            "path": rel_g,
                            "related_paths": [mpn],
                            "code": "state_owner_without_state_io_edges",
                            "trust_rank_hint": 2,
                        }
                    )

        slots_required: Set[str] = set()
        slots_filled: Set[str] = set()
        for r in rels:
            if not isinstance(r, dict):
                continue
            k = str(r.get("kind", "")).strip()
            to = str(r.get("to", "")).strip()
            if not to:
                continue
            if k == "requires_slot":
                slots_required.add(to)
            elif k == "fills_slot":
                slots_filled.add(to)
        for slot in sorted(slots_required - slots_filled):
            findings.append(
                {
                    "id": f"ir:slot-unfilled:{slot}:{rel_g}",
                    "category": "semantic_contradiction",
                    "severity": "warning",
                    "message": (
                        f"Slot {slot!r} is required (requires_slot) but no fills_slot edge targets it in this graph."
                    ),
                    "path": rel_g,
                    "code": "slot_requires_without_provider",
                    "trust_rank_hint": 2,
                }
            )

        workflows = doc.get("workflows") or []
        if isinstance(workflows, list):
            for wf in workflows:
                if not isinstance(wf, dict):
                    continue
                wid = str(wf.get("id", "")).strip()
                steps = wf.get("steps") or []
                wuids = _workflow_step_unit_ids(steps)
                for i in range(len(wuids) - 1):
                    a, b = wuids[i], wuids[i + 1]
                    if not _rel_graph_has_edge_between(a, b, rels):
                        findings.append(
                            {
                                "id": f"ir:workflow-gap:{wid}:{a}->{b}:{rel_g}",
                                "category": "semantic_contradiction",
                                "severity": "info",
                                "message": (
                                    f"Workflow {wid!r} steps chain {a!r} → {b!r} but no relationship edge connects "
                                    "these units — add depends_on (or related) or fix step unitIds."
                                ),
                                "path": rel_g,
                                "code": "workflow_sequence_without_relationship_edge",
                                "trust_rank_hint": 3,
                            }
                        )

    return findings


def collect_compatibility_claim_contradictions(root: Path) -> List[Dict[str, Any]]:
    """compatibility.class=fully_compatible vs open blocking debt or inference contradictions."""
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings
    for unit_path in _walk_unit_files(root):
        try:
            doc = _load_yaml(unit_path)
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(doc, dict):
            continue
        comp = doc.get("compatibility")
        if not isinstance(comp, dict):
            continue
        cls = str(comp.get("class", "")).strip().lower()
        if cls != "fully_compatible":
            continue
        rel_u = _rel(root, unit_path)
        debt = doc.get("debt") or []
        blocked = False
        if isinstance(debt, list):
            for it in debt:
                if not isinstance(it, dict):
                    continue
                stg = str(it.get("promotion_stage", "open") or "open").lower()
                if it.get("blocks_verify") is True and stg in ("open", "in_review"):
                    blocked = True
                    break
        inf = doc.get("inference") if isinstance(doc.get("inference"), dict) else {}
        confl = inf.get("contradictions") if isinstance(inf.get("contradictions"), list) else []
        if blocked:
            findings.append(
                {
                    "id": f"compat:debt:{rel_u}",
                    "category": "semantic_contradiction",
                    "severity": "warning",
                    "message": (
                        "chassis.unit.yaml claims compatibility.class=fully_compatible but debt still "
                        "has blocks_verify items in open/in_review — reconcile claim or resolve debt."
                    ),
                    "path": rel_u,
                    "code": "compatibility_fully_compatible_with_blocking_debt",
                    "trust_rank_hint": 2,
                }
            )
        if confl:
            findings.append(
                {
                    "id": f"compat:infer-contra:{rel_u}",
                    "category": "semantic_contradiction",
                    "severity": "info",
                    "message": (
                        "compatibility.class=fully_compatible but inference.contradictions is non-empty — "
                        "treat compatibility as aspirational until cleared."
                    ),
                    "path": rel_u,
                    "code": "compatibility_fully_compatible_with_inference_contradictions",
                    "trust_rank_hint": 3,
                }
            )
    return findings


def collect_state_owner_reader_writer_contradictions(root: Path) -> List[Dict[str, Any]]:
    """Detect contradictions between declared state_owner units and actual code-level read/write patterns."""
    findings: List[Dict[str, Any]] = []
    import re
    STATE_IO_RE = re.compile(
        r"(?:"
        r"(?:import|require|from)\s+.*(?:store|state|reducer|repository|dao|cache|db)"
        r"|(?:\.get|\.set|\.put|\.delete|\.update|\.insert|\.query|\.find|\.save)\s*\("
        r"|(?:SELECT|INSERT|UPDATE|DELETE|CREATE)\s+"
        r"|(?:useSelector|useStore|useRecoilValue|useAtom)\s*\("
        r"|(?:dispatch|commit|mutate)\s*\("
        r")",
        re.IGNORECASE,
    )
    graphs = list(root.rglob("*.architecture-graph.json"))
    state_owners: Dict[str, Optional[str]] = {}
    declared_readers: Dict[str, Set[str]] = {}
    declared_writers: Dict[str, Set[str]] = {}
    for gp in graphs:
        try:
            g = json.loads(gp.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            continue
        for u in g.get("units", []):
            uid = u.get("id", "")
            if u.get("kind") == "state_owner":
                state_owners[uid] = u.get("manifestPath")
        for r in g.get("relationships", []):
            kind = r.get("kind", "")
            frm, to = r.get("from", ""), r.get("to", "")
            if kind == "reads_state":
                declared_readers.setdefault(to, set()).add(frm)
            elif kind in ("writes_state", "owns_state"):
                declared_writers.setdefault(to, set()).add(frm)
    # Scan source files for actual state I/O patterns
    skip = {"node_modules", "target", ".git", "dist", "bin", "obj", ".ci-venv", ".venv"}
    for so_id, manifest_path in state_owners.items():
        if not manifest_path:
            continue
        mod_dir = root / Path(manifest_path).parent
        if not mod_dir.is_dir():
            continue
        actual_importers: Set[str] = set()
        mod_name = mod_dir.name
        for src in root.rglob("*"):
            if not src.is_file() or src.suffix not in (".ts", ".tsx", ".js", ".rs", ".cs", ".py"):
                continue
            if any(s in src.parts for s in skip):
                continue
            if src.is_relative_to(mod_dir):
                continue
            try:
                text = src.read_text(encoding="utf-8", errors="ignore")[:8000]
            except OSError:
                continue
            if mod_name in text and STATE_IO_RE.search(text):
                rel = str(src.relative_to(root))
                # Find which unit this file belongs to
                parent = src.parent
                while parent != root:
                    contract = parent / "CONTRACT.yaml"
                    if contract.is_file():
                        actual_importers.add(str(parent.relative_to(root)))
                        break
                    parent = parent.parent
        declared = declared_readers.get(so_id, set()) | declared_writers.get(so_id, set())
        undeclared = actual_importers - {str(Path(manifest_path).parent)} - declared
        for imp in sorted(undeclared)[:5]:
            findings.append({
                "id": f"contradiction:state_io:{so_id}:{imp}",
                "category": "semantic_contradiction",
                "severity": "warning",
                "message": f"Module at `{imp}` appears to read/write state owned by `{so_id}` but has no reads_state/writes_state edge in architecture IR.",
                "path": imp,
                "related_paths": [manifest_path or ""],
                "code": "undeclared_state_reader_writer",
                "trust_rank_hint": 2,
            })
    return findings


def collect_slot_signature_contradictions(root: Path) -> List[Dict[str, Any]]:
    """Detect contradictions between slot capability requirements and actual provider exports."""
    findings: List[Dict[str, Any]] = []
    # Load domain instances to find slots and their requirements
    from repo_layout import resolve_chassis_config_path

    config_path = resolve_chassis_config_path(root)
    if config_path is None or not config_path.is_file():
        return findings
    try:
        if yaml is not None:
            cfg = yaml.safe_load(config_path.read_text(encoding="utf-8"))
        else:
            return findings
    except Exception:
        return findings
    project = (cfg or {}).get("project") or {}
    scan_roots = list(project.get("domain_instance_roots") or [])
    slots: Dict[str, Dict[str, Any]] = {}
    modules: Dict[str, Dict[str, Any]] = {}
    for sr in scan_roots:
        sr_path = root / sr if not Path(sr).is_absolute() else Path(sr)
        if not sr_path.is_dir():
            continue
        for f in sr_path.rglob("*.slot.json"):
            try:
                doc = json.loads(f.read_text(encoding="utf-8"))
                sid = doc.get("slotId", "")
                if sid:
                    slots[sid] = doc
            except (json.JSONDecodeError, OSError):
                continue
        for f in sr_path.rglob("*.module.json"):
            try:
                doc = json.loads(f.read_text(encoding="utf-8"))
                mid = doc.get("moduleId", "")
                if mid:
                    modules[mid] = doc
            except (json.JSONDecodeError, OSError):
                continue
    # Check: modules filling slots must provide required capabilities
    for mid, mod in modules.items():
        provides = {c.get("name") for c in mod.get("provides", []) if isinstance(c, dict)}
        for fill in mod.get("fills", []):
            sid = fill.get("slotId", "")
            slot = slots.get(sid)
            if not slot:
                continue
            for req in slot.get("requiredCapabilities", []):
                if not isinstance(req, dict):
                    continue
                cap_name = req.get("name", "")
                is_required = req.get("required", True)
                if is_required and cap_name and cap_name not in provides:
                    findings.append({
                        "id": f"contradiction:slot_cap:{mid}:{sid}:{cap_name}",
                        "category": "semantic_contradiction",
                        "severity": "warning",
                        "message": f"Module `{mid}` fills slot `{sid}` but does not provide required capability `{cap_name}`.",
                        "path": sid,
                        "code": "slot_capability_not_provided",
                        "trust_rank_hint": 1,
                    })
            # Also check: if slot declares expectedExports, verify module actually exports them
            expected = slot.get("expectedExports", [])
            if isinstance(expected, list) and expected:
                actual_exports = set(mod.get("provides", []) + mod.get("fills", [{}])[0].get("exports", []))
                # Fallback: check manifests for matching CONTRACT exports
                fill_path = fill.get("manifestPath", "")
                if fill_path:
                    contract_path = root / fill_path
                    if contract_path.is_file():
                        try:
                            if yaml is not None:
                                cdata = yaml.safe_load(contract_path.read_text(encoding="utf-8"))
                                if isinstance(cdata, dict):
                                    actual_exports |= set(cdata.get("exports", []))
                        except Exception:
                            pass
                for exp_name in expected:
                    if exp_name and exp_name not in actual_exports:
                        findings.append({
                            "id": f"contradiction:slot_export:{mid}:{sid}:{exp_name}",
                            "category": "semantic_contradiction",
                            "severity": "warning",
                            "message": f"Slot `{sid}` expects export `{exp_name}` from filler `{mid}` but it is not in the module's export surface.",
                            "path": sid,
                            "code": "slot_expected_export_missing",
                            "trust_rank_hint": 1,
                        })
    return findings


def collect_compatibility_diff_contradictions(root: Path) -> List[Dict[str, Any]]:
    """Detect contradictions between compatibility claims and actual contract diff semantics."""
    findings: List[Dict[str, Any]] = []
    compat_files = list(root.rglob("*.integration-compatibility-result.json"))
    for cp in compat_files:
        try:
            doc = json.loads(cp.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            continue
        compat_class = doc.get("compatibility_class", "")
        if compat_class != "fully_compatible":
            continue
        # fully_compatible claims should not have breaking changes in linked contracts
        host_contracts = doc.get("host_contracts", []) or doc.get("contracts", [])
        candidate_contracts = doc.get("candidate_contracts", []) or []
        for hc in host_contracts:
            if not isinstance(hc, str):
                continue
            hc_path = root / hc
            if not hc_path.is_file():
                continue
            try:
                if yaml is not None:
                    hc_data = yaml.safe_load(hc_path.read_text(encoding="utf-8"))
                else:
                    continue
            except Exception:
                continue
            if not isinstance(hc_data, dict):
                continue
            # Check if contract has superseded or deprecated status
            status = hc_data.get("status", "")
            if status in ("deprecated", "superseded"):
                findings.append({
                    "id": f"contradiction:compat_diff:{cp.name}:{hc}",
                    "category": "semantic_contradiction",
                    "severity": "warning",
                    "message": f"Compatibility result claims fully_compatible but host contract `{hc}` has status `{status}`. A deprecated/superseded contract cannot be fully compatible.",
                    "path": str(cp.relative_to(root)),
                    "related_paths": [hc],
                    "code": "compatibility_class_vs_contract_status",
                    "trust_rank_hint": 1,
                })
            # Check for exports surface changes
            exports = hc_data.get("exports", [])
            if isinstance(exports, list) and len(exports) == 0 and hc_data.get("kind") not in ("feature-flag", "type", "schema"):
                findings.append({
                    "id": f"contradiction:compat_empty_exports:{cp.name}:{hc}",
                    "category": "semantic_contradiction",
                    "severity": "info",
                    "message": f"Compatibility claims fully_compatible but host contract `{hc}` has empty exports. Without a declared surface, compatibility is vacuously true.",
                    "path": str(cp.relative_to(root)),
                    "code": "compatibility_vacuous_empty_exports",
                    "trust_rank_hint": 2,
                })
    return findings


def collect_declared_exports_vs_code(root: Path) -> List[Dict[str, Any]]:
    """Detect contradictions between CONTRACT.yaml declared exports and what the adapter would infer from code."""
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings
    try:
        _tools_infer = Path(__file__).resolve().parent / "infer"
        if str(_tools_infer.parent) not in sys.path:
            sys.path.insert(0, str(_tools_infer.parent))
        from infer.discover import detect_adapter_for_dir
        from infer.build_manifests import _run_adapter
    except ImportError:
        return findings
    for path in sorted(_walk_manifests(root), key=lambda p: str(p)):
        if path.name != "CONTRACT.yaml":
            continue
        try:
            doc = _load_yaml(path)
        except Exception:
            continue
        if not isinstance(doc, dict):
            continue
        status = str(doc.get("status", "")).strip().lower()
        if status in ("draft", "inferred"):
            continue
        inf = doc.get("inference")
        if isinstance(inf, dict) and str(inf.get("promotion_stage", "")).strip().lower() == "draft":
            continue
        declared_exports = set(str(x) for x in (doc.get("exports") or []) if x)
        declared_deps = set(str(x) for x in (doc.get("depends_on") or []) if x)
        if not declared_exports and not declared_deps:
            continue
        mod_dir = path.parent
        rel = _rel(root, path)
        try:
            data = _run_adapter(mod_dir, "auto")
        except Exception:
            continue
        actual_exports = set(str(x) for x in (data.get("exports") or []) if x)
        actual_deps = set(str(x) for x in (data.get("depends_on") or []) if x)
        if declared_exports and actual_exports:
            missing_in_code = declared_exports - actual_exports
            if missing_in_code and len(missing_in_code) > len(declared_exports) * 0.3:
                findings.append({
                    "id": f"contradiction:exports_vs_code:{rel}",
                    "category": "semantic_contradiction",
                    "severity": "warning",
                    "message": (
                        f"CONTRACT declares {len(declared_exports)} exports but {len(missing_in_code)} "
                        f"not found in code scan: {', '.join(sorted(missing_in_code)[:5])}. "
                        "Exports may have been removed, renamed, or are dynamically generated."
                    ),
                    "path": rel,
                    "code": "declared_exports_not_in_code",
                    "trust_rank_hint": 1,
                })
            new_in_code = actual_exports - declared_exports
            if new_in_code and len(new_in_code) >= 3:
                findings.append({
                    "id": f"contradiction:undeclared_exports:{rel}",
                    "category": "semantic_warning",
                    "severity": "info",
                    "message": (
                        f"{len(new_in_code)} export(s) found in code but not declared in CONTRACT: "
                        f"{', '.join(sorted(new_in_code)[:5])}. Consider updating exports[]."
                    ),
                    "path": rel,
                    "code": "undeclared_code_exports",
                    "trust_rank_hint": 2,
                })
        if declared_deps and actual_deps:
            stale_deps = declared_deps - actual_deps
            if stale_deps and len(stale_deps) >= 2:
                findings.append({
                    "id": f"contradiction:deps_vs_code:{rel}",
                    "category": "semantic_contradiction",
                    "severity": "info",
                    "message": (
                        f"{len(stale_deps)} declared dependency/dependencies not found in code imports: "
                        f"{', '.join(sorted(stale_deps)[:5])}. May be stale or build-only."
                    ),
                    "path": rel,
                    "code": "declared_deps_not_in_code",
                    "trust_rank_hint": 2,
                })
    return findings[:30]


def collect_kind_vs_code_contradictions(root: Path) -> List[Dict[str, Any]]:
    """Detect contradictions between CONTRACT kind and actual code patterns."""
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings
    import re as _re
    STATE_PATTERNS = _re.compile(
        r"(?:store|state|reducer|slice|atom|createStore|defineStore|DbContext|DbSet|"
        r"createSlice|dispatch|commit|mutate|Column\s*\(|relationship\s*\()",
        _re.IGNORECASE,
    )
    SERVICE_PATTERNS = _re.compile(
        r"(?:app\.listen|express\(|FastAPI\(|\.get\s*\([\"']/|\.post\s*\([\"']/|"
        r"@Controller|@RestController|HttpGet|HttpPost|router\.)",
        _re.IGNORECASE,
    )
    exts = {".ts", ".tsx", ".js", ".jsx", ".rs", ".cs", ".py", ".go", ".java"}
    skip = {"node_modules", "target", ".git", "dist", "bin", "obj", "__pycache__", "venv", ".venv"}
    for path in sorted(_walk_manifests(root), key=lambda p: str(p)):
        if path.name != "CONTRACT.yaml":
            continue
        try:
            doc = _load_yaml(path)
        except Exception:
            continue
        if not isinstance(doc, dict):
            continue
        kind = str(doc.get("kind", "")).strip().lower()
        if not kind or kind in ("module", "crate", "package"):
            continue
        status = str(doc.get("status", "")).strip().lower()
        if status in ("draft", "inferred"):
            continue
        mod_dir = path.parent
        rel = _rel(root, path)
        has_state = has_service = False
        file_count = 0
        for src in mod_dir.rglob("*"):
            if not src.is_file() or src.suffix not in exts:
                continue
            if any(s in src.parts for s in skip):
                continue
            file_count += 1
            if file_count > 30:
                break
            try:
                text = src.read_text(encoding="utf-8", errors="ignore")[:6000]
            except OSError:
                continue
            if STATE_PATTERNS.search(text):
                has_state = True
            if SERVICE_PATTERNS.search(text):
                has_service = True
        if kind == "store" and not has_state:
            findings.append({
                "id": f"contradiction:kind_store_no_state:{rel}",
                "category": "semantic_contradiction",
                "severity": "warning",
                "message": "CONTRACT kind is 'store' but no state management patterns found in code.",
                "path": rel,
                "code": "kind_store_without_state_patterns",
                "trust_rank_hint": 2,
            })
        elif kind == "service" and not has_service and file_count > 0:
            findings.append({
                "id": f"contradiction:kind_service_no_endpoints:{rel}",
                "category": "semantic_contradiction",
                "severity": "info",
                "message": "CONTRACT kind is 'service' but no HTTP/API endpoint patterns found in code.",
                "path": rel,
                "code": "kind_service_without_endpoint_patterns",
                "trust_rank_hint": 2,
            })
    return findings[:20]


def collect_workflow_evidence_contradictions(root: Path, rt_ctx: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Detect contradictions between workflow models and observed runtime evidence."""
    findings: List[Dict[str, Any]] = []
    if not rt_ctx.get("observation_count"):
        return findings
    observations = rt_ctx.get("observations", [])
    observed_paths: Set[str] = set()
    observed_endpoints: Set[str] = set()
    for obs in observations:
        if not isinstance(obs, dict):
            continue
        kind = obs.get("kind", "")
        ref = obs.get("ref", "")
        if kind == "workflow_path_seen" and ref:
            observed_paths.add(ref)
        elif kind == "endpoint_invocation" and ref:
            observed_endpoints.add(ref)
    if not observed_paths and not observed_endpoints:
        return findings
    graphs = list(root.rglob("*.architecture-graph.json"))
    for gp in graphs:
        try:
            g = json.loads(gp.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            continue
        for wf in g.get("workflows", []):
            wf_id = wf.get("id", "")
            steps = wf.get("steps", [])
            declared_step_ids = [s.get("unitId", "") for s in steps if isinstance(s, dict)]
            if not declared_step_ids:
                continue
            # Check if observed paths include this workflow but skip steps
            if wf_id in observed_paths:
                # Workflow was observed running — check if all steps were observed
                unobserved_steps = [s for s in declared_step_ids if s and s not in observed_paths]
                if unobserved_steps:
                    findings.append({
                        "id": f"contradiction:workflow_partial:{wf_id}",
                        "category": "semantic_contradiction",
                        "severity": "info",
                        "message": f"Workflow `{wf_id}` was observed at runtime but {len(unobserved_steps)} declared step(s) were never seen: {', '.join(unobserved_steps[:5])}. Steps may be dead code or conditionally skipped.",
                        "path": str(gp.relative_to(root)),
                        "code": "workflow_steps_unobserved",
                        "trust_rank_hint": 2,
                    })
    return findings


def collect_inference_promotion_findings(root: Path) -> List[Dict[str, Any]]:
    """verified promotion_stage requires reviewer acknowledgment; reviewed should follow draft evidence."""
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings
    for path in sorted(_walk_manifests(root), key=lambda p: str(p)):
        if path.name != "CONTRACT.yaml":
            continue
        try:
            doc = _load_yaml(path)
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(doc, dict):
            continue
        inf = doc.get("inference")
        if not isinstance(inf, dict):
            continue
        stage = str(inf.get("promotion_stage", "") or "").strip().lower()
        rel = _rel(root, path)
        acks = inf.get("reviewer_acknowledgments")
        ack_list = acks if isinstance(acks, list) else []
        verified_ack = any(
            isinstance(a, dict) and str(a.get("stage", "")).strip().lower() == "verified"
            for a in ack_list
        )
        reviewed_ack = any(
            isinstance(a, dict) and str(a.get("stage", "")).strip().lower() == "reviewed"
            for a in ack_list
        )
        if stage == "verified" and not verified_ack:
            findings.append(
                {
                    "id": f"promotion:verified-no-ack:{rel}",
                    "category": "semantic_contradiction",
                    "severity": "error",
                    "message": (
                        "inference.promotion_stage is verified but no reviewer_acknowledgments entry "
                        "with stage=verified — record reviewer + timestamp (chassis promote-manifest)."
                    ),
                    "path": rel,
                    "code": "verified_without_reviewer_acknowledgment",
                    "trust_rank_hint": 1,
                }
            )
        if stage == "reviewed" and not reviewed_ack:
            findings.append(
                {
                    "id": f"promotion:reviewed-no-ack:{rel}",
                    "category": "semantic_contradiction",
                    "severity": "warning",
                    "message": (
                        "inference.promotion_stage is reviewed but no reviewer_acknowledgments entry "
                        "with stage=reviewed — record acknowledgment for audit trail."
                    ),
                    "path": rel,
                    "code": "reviewed_without_reviewer_acknowledgment",
                    "trust_rank_hint": 2,
                }
            )
        if stage == "verified":
            debt = doc.get("debt") or []
            if isinstance(debt, list):
                for it in debt:
                    if not isinstance(it, dict):
                        continue
                    stg = str(it.get("promotion_stage", "open") or "open").lower()
                    if it.get("blocks_verify") is True and stg in ("open", "in_review"):
                        did = str(it.get("id", ""))
                        findings.append(
                            {
                                "id": f"promotion:verified-debt:{rel}:{did}"[:120],
                                "category": "semantic_contradiction",
                                "severity": "error",
                                "message": (
                                    f"inference.promotion_stage is verified but debt {did!r} still blocks_verify "
                                    "and is not resolved — cannot treat manifest as verified."
                                ),
                                "path": rel,
                                "code": "verified_with_open_blocking_debt",
                                "trust_rank_hint": 0,
                            }
                        )
                        break
    return findings


def collect_declared_vs_observed_findings(
    root: Path, rt_ctx: Dict[str, Any]
) -> List[Dict[str, Any]]:
    """Link runtime declared_unobserved refs to CONTRACT export surfaces (heuristic match)."""
    findings: List[Dict[str, Any]] = []
    unobs: Set[str] = rt_ctx.get("declared_unobserved") or set()
    if yaml is None:
        return findings

    if unobs:
        for path in sorted(_walk_manifests(root), key=lambda p: str(p)):
            if path.name != "CONTRACT.yaml":
                continue
            try:
                doc = _load_yaml(path)
            except Exception:  # noqa: BLE001
                continue
            if not isinstance(doc, dict):
                continue
            rel = _rel(root, path)
            name = str(doc.get("name", "")).strip()
            exports = [str(x) for x in (doc.get("exports") or []) if x]
            mod_dir = path.parent
            for ref in unobs:
                ref_l = ref.lower()
                hit = name and (name.lower() in ref_l or ref_l in name.lower())
                if not hit:
                    for e in exports:
                        el = e.lower()
                        if el in ref_l or ref_l in el:
                            hit = True
                            break
                if not hit and mod_dir.name.lower() in ref_l:
                    hit = True
                if not hit:
                    continue
                findings.append(
                    {
                        "id": f"runtime:unobserved-contract:{rel}:{ref}"[:120],
                        "category": "runtime_evidence_signal",
                        "severity": "warning",
                        "message": (
                            f"Runtime evidence declared_unobserved ref {ref!r} matches this CONTRACT export surface "
                            f"({rel}) — treat declared exports as lower observed-confidence until observed."
                        ),
                        "path": rel,
                        "related_paths": list(rt_ctx.get("sources") or [])[:3],
                        "code": "declared_surface_unobserved_runtime",
                        "trust_rank_hint": 2,
                    }
                )
                # Escalate severity when substantial evidence exists but this contract is unobserved
                if rt_ctx.get("observation_count", 0) >= 10:
                    findings[-1]["severity"] = "warning"
                    findings[-1]["message"] += (
                        " (elevated: repo has significant runtime evidence but this contract has none)"
                    )
                    findings[-1]["trust_rank_hint"] = 1

    # Boost confidence for contracts with positive runtime observations
    observed_refs = set()
    for obs in rt_ctx.get("observations", []):
        if isinstance(obs, dict) and obs.get("ref"):
            observed_refs.add(obs["ref"])
    if observed_refs:
        for mf_path in sorted(root.rglob("CONTRACT.yaml")):
            if any(s in mf_path.parts for s in ("node_modules", "target", ".git", "dist", "fixtures", "templates")):
                continue
            try:
                if yaml is not None:
                    data = yaml.safe_load(mf_path.read_text(encoding="utf-8"))
                else:
                    continue
            except Exception:
                continue
            if not isinstance(data, dict):
                continue
            name = data.get("name", "")
            exports = data.get("exports", [])
            matched = any(
                e in observed_refs or name in observed_refs
                for e in (exports if isinstance(exports, list) else [])
            ) or name in observed_refs
            if matched:
                rel = str(mf_path.relative_to(root))
                findings.append({
                    "id": f"observed:confidence_boost:{rel}",
                    "category": "runtime_evidence_signal",
                    "severity": "info",
                    "message": f"Contract `{name}` has matching runtime observations — elevated confidence.",
                    "path": rel,
                    "code": "observed_confidence_boost",
                    "trust_rank_hint": 3,
                })

    return findings


def collect_unlinked_local_architecture_docs(root: Path) -> List[Dict[str, Any]]:
    """
    Local explanatory markdown without explicit chassis projection markers should not be read as canonical IR.
    Markers: chassis:projection, chassis_projection, or HTML comment <!-- chassis:projection ... -->
    """
    findings: List[Dict[str, Any]] = []
    downgrade_projection = os.environ.get("CHASSIS_COHERENCE_PROJECTION_WARNINGS", "") == "1"
    markers = (
        "chassis:projection",
        "chassis_projection:",
        "<!-- chassis:projection",
        "chassis_projection",
    )
    doc_names = frozenset({"README.md", "ARCHITECTURE.md", "DESIGN.md", "OVERVIEW.md"})
    for path in sorted(_walk_manifests(root), key=lambda p: str(p)):
        if path.name != "CONTRACT.yaml":
            continue
        mod_dir = path.parent
        for name in doc_names:
            docp = mod_dir / name
            if not docp.is_file():
                continue
            try:
                txt = docp.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            if len(txt.strip()) < 120:
                continue
            low = txt.lower()
            if not any(
                h in low
                for h in (
                    "architecture",
                    "system design",
                    "component diagram",
                    "context diagram",
                    "c4 model",
                    "deployment",
                )
            ):
                continue
            if any(m in txt for m in markers):
                continue
            rel_d = _rel(root, docp)
            rel_c = _rel(root, path)
            severity = "warning" if downgrade_projection else "error"
            findings.append(
                {
                    "id": f"projection:local-doc:{rel_d}",
                    "category": "projection_drift",
                    "severity": severity,
                    "message": (
                        f"Architecture-relevant local doc `{rel_d}` has no chassis projection marker — "
                        "classify as explanatory note or debt artifact, not canonical IR; "
                        "add chassis:projection (or chassis_projection) linking to IR unit/workflow/manifest."
                        " Set CHASSIS_COHERENCE_PROJECTION_WARNINGS=1 to downgrade to warning."
                    ),
                    "path": rel_d,
                    "related_paths": [rel_c],
                    "code": "local_arch_doc_unlinked_strict",
                    "trust_rank_hint": 2,
                }
            )
    return findings


def summarize_contract_debt(root: Path) -> Dict[str, Any]:
    """Aggregate debt[] from CONTRACT.yaml files for coherence summary."""
    by_sev: Dict[str, int] = defaultdict(int)
    open_block = 0
    total = 0
    if yaml is None:
        return {"total_items": 0, "by_severity": {}, "open_blocks_verify": 0}
    for path in _walk_manifests(root):
        if path.name != "CONTRACT.yaml":
            continue
        try:
            doc = _load_yaml(path)
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(doc, dict):
            continue
        debt = doc.get("debt")
        if not isinstance(debt, list):
            continue
        for item in debt:
            if not isinstance(item, dict):
                continue
            total += 1
            sev = str(item.get("severity") or "unknown").lower()
            by_sev[sev] += 1
            stage = str(item.get("promotion_stage") or "open").lower()
            if item.get("blocks_verify") is True and stage in ("open", "in_review"):
                open_block += 1
    return {
        "total_items": total,
        "by_severity": dict(sorted(by_sev.items())),
        "open_blocks_verify": open_block,
    }


def collect_ir_manifest_projection_findings(root: Path) -> List[Dict[str, Any]]:
    """Manifest name vs IR unit title drift for linked manifestPath entries."""
    findings: List[Dict[str, Any]] = []
    if yaml is None:
        return findings
    for gp in _iter_architecture_graph_paths(root, include_fixtures=False):
        rel_g = _rel(root, gp)
        try:
            doc = json.loads(gp.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        if not isinstance(doc, dict):
            continue
        for u in doc.get("units") or []:
            if not isinstance(u, dict):
                continue
            mp = u.get("manifestPath")
            if not isinstance(mp, str) or not mp.strip():
                continue
            mpn = mp.strip().replace("\\", "/")
            cf = root / mpn
            if not cf.is_file():
                continue
            try:
                cdoc = _load_yaml(cf)
            except Exception:  # noqa: BLE001
                continue
            if not isinstance(cdoc, dict):
                continue
            name = str(cdoc.get("name", "")).strip().lower()
            title = str(u.get("title", "")).strip().lower()
            uid = str(u.get("id", "")).strip()
            if not name or not title:
                continue
            if name != title and name not in title and title not in name:
                findings.append(
                    {
                        "id": f"ir:proj-name:{uid}:{rel_g}",
                        "category": "projection_drift",
                        "severity": "warning",
                        "message": (
                            f"Architecture unit {uid!r} title {title!r} differs from linked manifest name {name!r} "
                            "— IR is authoritative for topology; update manifest name or IR title to align."
                        ),
                        "path": rel_g,
                        "related_paths": [mpn],
                        "code": "ir_manifest_name_projection_drift",
                        "trust_rank_hint": 1,
                    }
                )
            uk = str(u.get("kind", "")).strip()
            mk = str(cdoc.get("kind", "")).strip()
            if uk == "state_owner" and mk and mk not in ("store", "entity", "service"):
                findings.append(
                    {
                        "id": f"ir:state-owner-kind:{uid}:{rel_g}",
                        "category": "semantic_contradiction",
                        "severity": "warning",
                        "message": (
                            f"IR marks {uid!r} as state_owner but CONTRACT kind is {mk!r} "
                            "(expected store/entity/service-like boundary). "
                            "Update CONTRACT kind to store/entity/service or fix IR classification."
                        ),
                        "path": rel_g,
                        "related_paths": [mpn],
                        "code": "state_owner_kind_hint_mismatch",
                        "trust_rank_hint": 1,
                    }
                )
    return findings


def build_authority_ledger(root: Path, findings: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    stale_paths: Set[str] = set()
    for f in findings:
        if f.get("category") == "metadata_stale" and isinstance(f.get("path"), str):
            stale_paths.add(f["path"])

    records: List[Dict[str, Any]] = []
    if yaml is not None:
        for mf in sorted(_walk_manifests(root), key=lambda p: str(p)):
            rel = _rel(root, mf)
            cls = "verified_contract" if mf.name == "CONTRACT.yaml" else "verified_unit"
            fresh = "stale" if rel in stale_paths else "fresh"
            try:
                doc = _load_yaml(mf)
            except Exception:  # noqa: BLE001
                doc = None
            if isinstance(doc, dict) and str(doc.get("status", "")).strip() == "inferred":
                cls = "inferred_manifest"
                fresh = "unknown"
            records.append(
                {
                    "path": rel,
                    "authority_class": cls,
                    "freshness_status": fresh,
                    "derived_from": [],
                }
            )

    for gp in _iter_architecture_graph_paths(root, include_fixtures=False):
        records.append(
            {
                "path": _rel(root, gp),
                "authority_class": "architecture_ir",
                "freshness_status": "unknown",
                "derived_from": [],
            }
        )

    raw, _ = load_raw_authority_index(root)
    if raw:
        for ent in evaluate_entries(root, raw):
            p = str(ent.get("path") or "").strip().replace("\\", "/")
            if not p:
                continue
            records.append(
                {
                    "path": p,
                    "authority_class": str(ent.get("authority_class") or "verified_contract"),
                    "freshness_status": str(ent.get("freshness_status") or "unknown"),
                    "derived_from": list(ent.get("derived_from") or []),
                }
            )

    sorted_records = sorted(records, key=lambda r: r["path"])
    for entry in sorted_records:
        # Classify truth type explicitly
        ac = entry.get("authority_class", "")
        if ac in ("verified_contract", "verified_unit", "architecture_ir"):
            entry["truth_classification"] = "canonical"
        elif ac in ("inferred_manifest",):
            entry["truth_classification"] = "debt"
        elif ac in ("prose_doc", "generated_artifact"):
            entry["truth_classification"] = "explanatory"
        else:
            entry["truth_classification"] = "unknown"
    return sorted_records


def collect_runtime_evidence_findings(root: Path) -> List[Dict[str, Any]]:
    """Optional runtime evidence: batch file(s) under .chassis/ (schema) or legacy runtime-evidence.json."""
    findings: List[Dict[str, Any]] = []
    schema_path = _locate_asset(
        "schemas/architecture/runtime-evidence-batch.schema.json", root
    )
    batch_candidates = [
        root / ".chassis" / "runtime-evidence.batch.json",
        root / ".chassis" / "observed.runtime-evidence.batch.json",
    ]
    seen_paths: Set[str] = set()
    all_observations: List[Dict[str, Any]] = []
    for batch_path in batch_candidates:
        if not batch_path.is_file():
            continue
        key = str(batch_path.resolve())
        if key in seen_paths:
            continue
        seen_paths.add(key)
        rel_pb = batch_path.relative_to(root).as_posix()
        try:
            doc = json.loads(batch_path.read_text(encoding="utf-8"))
        except Exception as e:  # noqa: BLE001
            findings.append(
                {
                    "id": f"runtime-evidence-batch:parse:{rel_pb}",
                    "category": "schema_invalid",
                    "severity": "warning",
                    "message": f"Could not parse runtime evidence batch {rel_pb}: {e}",
                    "path": rel_pb,
                    "trust_rank_hint": 2,
                }
            )
            continue
        if schema_path is not None:
            try:
                schema = json.loads(schema_path.read_text(encoding="utf-8"))
                Draft7Validator(schema).validate(doc)
            except Exception as e:  # noqa: BLE001
                findings.append(
                    {
                        "id": f"runtime-evidence-batch:schema:{rel_pb}",
                        "category": "schema_invalid",
                        "severity": "warning",
                        "message": f"Runtime evidence batch {rel_pb} failed JSON Schema: {e}",
                        "path": rel_pb,
                        "trust_rank_hint": 2,
                    }
                )
        obs = doc.get("observations") if isinstance(doc, dict) else []
        if not isinstance(obs, list):
            continue
        all_observations.extend(o for o in obs if isinstance(o, dict))
        for o in obs:
            if not isinstance(o, dict):
                continue
            kid = str(o.get("kind", ""))
            ref = str(o.get("ref", ""))
            oid = str(o.get("id", ref))
            msg = f"Observed runtime signal [{kid}] ref={ref!r}"
            if kid == "declared_unobserved":
                msg += " (declared surface not yet observed — lower observed-confidence for that ref)"
            sev_rt = "warning" if kid == "declared_unobserved" else "info"
            findings.append(
                {
                    "id": f"runtime-evidence:{rel_pb}:{oid}"[:120],
                    "category": "runtime_evidence_signal",
                    "severity": sev_rt,
                    "message": msg,
                    "path": rel_pb,
                    "code": kid or "observation",
                    "trust_rank_hint": 2 if kid == "declared_unobserved" else 3,
                }
            )
    # Contracts with high observation counts get explicit confidence findings
    ref_counts: Dict[str, int] = {}
    for obs in all_observations:
        if isinstance(obs, dict) and obs.get("ref"):
            ref = obs["ref"]
            ref_counts[ref] = ref_counts.get(ref, 0) + obs.get("count", 1)
    for ref, count in sorted(ref_counts.items(), key=lambda x: -x[1])[:10]:
        if count >= 5:
            findings.append({
                "id": f"runtime-evidence:high-confidence:{ref}",
                "category": "runtime_evidence_signal",
                "severity": "info",
                "message": f"Runtime ref `{ref}` observed {count} times — high behavioral confidence for this surface.",
                "path": "",
                "code": "runtime_high_observation_count",
                "trust_rank_hint": 3,
            })

    if seen_paths:
        return findings

    p = root / ".chassis" / "runtime-evidence.json"
    if not p.is_file():
        return findings
    try:
        doc = json.loads(p.read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001
        return [
            {
                "id": "runtime-evidence:parse",
                "category": "schema_invalid",
                "severity": "warning",
                "message": f"Could not parse runtime evidence file: {e}",
                "path": ".chassis/runtime-evidence.json",
                "trust_rank_hint": 2,
            }
        ]
    obs = doc.get("observations") if isinstance(doc, dict) else []
    if not isinstance(obs, list):
        return findings
    for o in obs:
        if not isinstance(o, dict):
            continue
        if str(o.get("kind")) != "declared_unobserved":
            continue
        ref = str(o.get("ref") or "")
        findings.append(
            {
                "id": f"runtime-evidence:unobserved:{o.get('id', ref)}",
                "category": "runtime_evidence_signal",
                "severity": "warning",
                "message": f"Runtime evidence marks declared artifact as unobserved: {ref!r}",
                "path": ".chassis/runtime-evidence.json",
                "code": "declared_unobserved",
                "trust_rank_hint": 2,
            }
        )
    return findings


def collect_authority_index_findings(root: Path) -> List[Dict[str, Any]]:
    findings: List[Dict[str, Any]] = []
    raw, _ = load_raw_authority_index(root)
    if not raw:
        return findings
    for ent in evaluate_entries(root, raw):
        if ent.get("freshness_status") != "stale":
            continue
        p = str(ent.get("path") or "")
        srs = ent.get("stale_reasons") if isinstance(ent.get("stale_reasons"), list) else []
        msg = (
            f"Authority index entry for {p!r} is stale "
            "(invalidation trigger newer than verified_at)."
        )
        if srs:
            msg += " Reasons: " + "; ".join(str(x) for x in srs[:5])
        findings.append(
            {
                "id": f"authority-index:stale:{p}",
                "category": "metadata_stale",
                "severity": "warning",
                "message": msg,
                "path": p,
                "stale_reasons": [str(x) for x in srs][:20],
                "trust_rank_hint": 2,
            }
        )
    return findings


def collect_stale_contract_findings(root: Path) -> List[Dict[str, Any]]:
    findings: List[Dict[str, Any]] = []
    for path in sorted(_walk_manifests(root), key=lambda p: str(p)):
        if path.name != "CONTRACT.yaml":
            continue
        rel = _rel(root, path)
        reasons = stale_reasons_for_manifest(root, rel)
        if not reasons:
            continue
        newer = any_source_newer_than_manifest(root, path.parent, path)
        msg_bits: List[str] = []
        if "implementation_newer_than_manifest" in reasons and newer is not None:
            msg_bits.append(f"source newer than CONTRACT ({newer.name})")
        if "architecture_ir_newer_than_manifest" in reasons:
            msg_bits.append("architecture IR graph newer than this CONTRACT")
        if "workflow_spec_newer_than_manifest" in reasons:
            msg_bits.append("architecture workflow JSON newer than this CONTRACT")
        graphs = ir_graphs_newer_than_manifest(root, rel)
        related: List[str] = []
        if newer is not None:
            related.append(_rel(root, newer))
        related.extend(graphs[:5])
        related.extend(workflows_newer_than_manifest(root, rel)[:5])
        findings.append(
            {
                "id": f"stale:{rel}",
                "category": "metadata_stale",
                "severity": "warning",
                "message": "; ".join(msg_bits) if msg_bits else "Manifest stale vs sources or architecture IR",
                "path": rel,
                "related_paths": related,
                "code": "source_newer_than_contract",
                "stale_reasons": reasons,
                "trust_rank_hint": 2,
            }
        )
    return findings


def build_report(
    root: Path,
    *,
    include_validate: bool = True,
    no_semantics: bool = False,
    include_drift: bool = True,
    changed_base: "str | None" = None,
    profile: "object | None" = None,
) -> Dict[str, Any]:
    trust = load_trust_ladder(root)
    findings: List[Dict[str, Any]] = []
    rt_ctx = load_runtime_evidence_context(root)
    findings.extend(collect_authority_findings(root))
    findings.extend(collect_stale_contract_findings(root))
    findings.extend(collect_architecture_ir_findings(root))
    findings.extend(collect_inferred_truth_gap_findings(root))
    findings.extend(collect_structured_debt_findings(root))
    findings.extend(collect_ir_duplicate_title_findings(root))
    findings.extend(collect_ir_manifest_projection_findings(root))
    findings.extend(collect_manifest_lifecycle_contradictions(root))
    findings.extend(collect_ir_semantic_contradictions(root))
    findings.extend(collect_compatibility_claim_contradictions(root))
    findings.extend(collect_state_owner_reader_writer_contradictions(root))
    findings.extend(collect_slot_signature_contradictions(root))
    findings.extend(collect_compatibility_diff_contradictions(root))
    findings.extend(collect_declared_exports_vs_code(root))
    findings.extend(collect_kind_vs_code_contradictions(root))
    findings.extend(collect_workflow_evidence_contradictions(root, rt_ctx))
    findings.extend(collect_inference_promotion_findings(root))
    findings.extend(collect_unlinked_local_architecture_docs(root))
    findings.extend(collect_authority_index_findings(root))
    findings.extend(collect_runtime_evidence_findings(root))
    findings.extend(collect_declared_vs_observed_findings(root, rt_ctx))
    if include_drift:
        findings.extend(collect_drift_findings(root))

    if include_validate:
        contract_schema_path = _locate_asset("schemas/metadata/contract.schema.json", root)
        unit_schema_path = _locate_asset("schemas/metadata/chassis-unit.schema.json", root)
        if contract_schema_path is not None and unit_schema_path is not None:
            # Use the shared registry-aware builder so relative $refs like
            # "debt-item.schema.json" resolve against sibling schemas instead
            # of falling through to remote HTTP retrieval.
            from jsonschema_support import draft7_validator_for_schema_file
            v_contract = draft7_validator_for_schema_file(contract_schema_path)
            v_unit = draft7_validator_for_schema_file(unit_schema_path)
            v_findings, _, _ = collect_metadata_findings(
                root, v_contract, v_unit, no_semantics=no_semantics, profile=profile,
            )
            findings.extend(v_findings)
        else:
            findings.append(
                {
                    "id": "validate:missing-schema",
                    "category": "schema_invalid",
                    "severity": "error",
                    "message": "Missing metadata JSON Schema files; skipped validate slice.",
                    "trust_rank_hint": 0,
                }
            )

    # Incremental: filter findings to only affected modules when --changed is used
    if changed_base is not None:
        sys.path.insert(0, str(Path(__file__).parent))
        from incremental import changed_files, affected_modules as _affected_modules
        _changed = changed_files(root, changed_base)
        if _changed:
            _module_filter = set(_affected_modules(root, _changed))
            findings = [
                f for f in findings
                if not f.get("path")
                or any(f["path"].startswith(m) for m in _module_filter)
            ]

    err_n = sum(1 for f in findings if f["severity"] == "error")
    warn_n = sum(1 for f in findings if f["severity"] == "warning")
    info_n = sum(1 for f in findings if f["severity"] == "info")
    by_cat: Dict[str, int] = {}
    for f in findings:
        c = cast(str, f["category"])
        by_cat[c] = by_cat.get(c, 0) + 1

    debt_summary = summarize_contract_debt(root)
    contradiction_n = sum(1 for f in findings if f.get("category") == "semantic_contradiction")
    projection_n = sum(1 for f in findings if f.get("category") == "projection_drift")
    runtime_obs_n = sum(1 for f in findings if f.get("category") == "runtime_evidence_signal")
    du = rt_ctx.get("declared_unobserved") or set()
    declared_unobserved_list = sorted(du) if isinstance(du, set) else []
    promotion_gap_n = sum(
        1
        for f in findings
        if f.get("code")
        in (
            "verified_without_reviewer_acknowledgment",
            "reviewed_without_reviewer_acknowledgment",
            "verified_with_open_blocking_debt",
        )
    )

    # Single source of truth for batch paths; canonical location is report root (see schema + guides).
    runtime_evidence_sources = list(rt_ctx.get("sources") or [])

    summary: Dict[str, Any] = {
        "errors": err_n,
        "warnings": warn_n,
        "infos": info_n,
        "by_category": by_cat,
        "debt": debt_summary,
        "semantic_contradiction_count": contradiction_n,
        "projection_drift_count": projection_n,
        "runtime_evidence_observation_count": runtime_obs_n,
        "promotion_policy_gap_count": promotion_gap_n,
        # Deprecated mirror: always identical to runtime_evidence_sources (root) and observed_truth.
        "runtime_evidence_sources": list(runtime_evidence_sources),
        "observed_truth": {
            "declared_unobserved_ref_count": len(declared_unobserved_list),
            "declared_unobserved_refs": declared_unobserved_list[:50],
            "runtime_evidence_sources": list(runtime_evidence_sources),
            "observation_count": int(rt_ctx.get("observation_count", 0)),
        },
    }

    # Compute observed coverage ratio (fraction of manifests with runtime-evidence-backed boosts)
    total_contracts = _count_contract_manifests(root)
    observed_boost_count = len([
        f for f in findings
        if f.get("code") == "observed_confidence_boost"
    ])
    ratio = observed_boost_count / float(total_contracts)
    summary["observed_truth"]["observed_coverage_ratio"] = round(min(1.0, ratio), 2)
    summary["observed_truth"]["contracts_with_evidence"] = observed_boost_count
    obs_n = int(summary["observed_truth"].get("observation_count", 0))
    summary["observed_truth"]["confidence_impact"] = (
        "high" if summary["observed_truth"].get("observed_coverage_ratio", 0) >= 0.5
        else "medium" if summary["observed_truth"].get("observed_coverage_ratio", 0) >= 0.2
        else "low" if obs_n > 0 or observed_boost_count > 0
        else "none"
    )

    summary["assurance_ladder"] = _compute_assurance_ladder(root)

    return {
        "schema_version": "1.0.0",
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "runtime_evidence_sources": list(runtime_evidence_sources),
        "summary": summary,
        "trust_ladder": trust,
        "authority_ledger": build_authority_ledger(root, findings),
        "autonomy_hints": extended_autonomy_hints(root),
        "findings": findings,
    }


def emit_coherence_markdown(report: Dict[str, Any]) -> str:
    s = report.get("summary") or {}
    debt_s = s.get("debt") or {}
    obs = s.get("observed_truth") or {}
    rt_sources = report.get("runtime_evidence_sources")
    if not isinstance(rt_sources, list):
        rt_sources = obs.get("runtime_evidence_sources") or []
    promo_gaps = s.get("promotion_policy_gap_count", 0)
    lines = [
        "# Repository coherence",
        "",
        f"Generated: `{report.get('generated_at', '')}`",
        "",
        f"**Errors:** {s.get('errors', 0)} · **Warnings:** {s.get('warnings', 0)} · **Infos:** {s.get('infos', 0)}",
        "",
        f"**Debt (CONTRACT):** total_items={debt_s.get('total_items', 0)} · "
        f"open_blocks_verify={debt_s.get('open_blocks_verify', 0)} · "
        f"by_severity={debt_s.get('by_severity', {})}",
        f"**Observed truth (runtime):** coverage_ratio={obs.get('observed_coverage_ratio', 0)} "
        f"contracts_with_evidence={obs.get('contracts_with_evidence', 0)} "
        f"batch_observations={obs.get('observation_count', 0)} "
        f"declared_unobserved_refs={obs.get('declared_unobserved_ref_count', 0)} "
        f"(sources: {', '.join(str(x) for x in rt_sources) or '—'})",
        f"**Promotion policy gaps:** {promo_gaps}",
        "",
        "## Findings",
        "",
    ]
    contra: List[str] = []
    other: List[str] = []
    for f in report.get("findings") or []:
        if not isinstance(f, dict):
            continue
        sev = f.get("severity", "")
        cat = f.get("category", "")
        msg = f.get("message", "")
        p = f.get("path", "")
        line = f"- **{sev}** `[{cat}]` {msg}" + (f" — `{p}`" if p else "")
        if cat == "semantic_contradiction":
            contra.append(line)
        else:
            other.append(line)
    lines.extend(other)
    if contra:
        lines.extend(["", "### Semantic contradictions (typed)", ""])
        lines.extend(contra)
    if not other and not contra:
        lines.append("_No findings._")
    lines.extend(["", "## Authority ledger (excerpt)", ""])
    ledger = report.get("authority_ledger") or []
    for row in ledger[:40]:
        if isinstance(row, dict):
            truth = row.get('truth_classification', 'unknown')
            lines.append(
                f"- `{row.get('path','')}` — {row.get('authority_class','')}"
                f" / **{row.get('freshness_status','')}** | Truth: {truth}"
            )
    if len(ledger) > 40:
        lines.append(f"- _… {len(ledger) - 40} more rows_")
    return "\n".join(lines) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser(description="Emit Chassis repository coherence report (JSON).")
    ap.add_argument("--repo-root", default=str(_repo_root()))
    ap.add_argument(
        "--format",
        choices=("json", "text", "md"),
        default="json",
        help="json (default), markdown findings table, or short human summary",
    )
    ap.add_argument(
        "--json-out",
        metavar="FILE",
        help="Write JSON report to FILE (implies --format json).",
    )
    ap.add_argument(
        "--no-validate",
        action="store_true",
        help="Skip JSON Schema / semantics slice (authority + stale only).",
    )
    ap.add_argument(
        "--no-semantics",
        action="store_true",
        help="When validating, skip semantic lints (same as validate-metadata --no-semantics).",
    )
    ap.add_argument(
        "--no-drift",
        action="store_true",
        help="Skip drift engine aggregation (faster; smaller report).",
    )
    ap.add_argument(
        "--fail-on",
        choices=("never", "warning", "error"),
        default="never",
        help="Exit non-zero when threshold met (default: never).",
    )
    ap.add_argument(
        "--changed",
        nargs="?",
        const="HEAD",
        default=None,
        metavar="BASE_REF",
        help="Only report findings for modules affected by changes since BASE_REF (default: HEAD)",
    )
    # Import locally to avoid a top-level dependency on profile resolution when
    # coherence is imported as a library.
    sys.path.insert(0, str(Path(__file__).parent))
    from chassis_profile import add_profile_argument, resolve_profile

    add_profile_argument(ap)
    args = ap.parse_args()
    root = Path(args.repo_root).resolve()
    profile = resolve_profile(args.profile, repo_root=root)

    report = build_report(
        root,
        include_validate=not args.no_validate,
        no_semantics=args.no_semantics,
        include_drift=not args.no_drift,
        changed_base=args.changed,
        profile=profile,
    )
    report.setdefault("profile", profile.name)

    fmt = "json" if args.json_out else args.format
    if fmt == "json":
        text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
        if args.json_out:
            Path(args.json_out).write_text(text, encoding="utf-8", newline="\n")
        else:
            sys.stdout.write(text)
    elif fmt == "md":
        sys.stdout.write(emit_coherence_markdown(report))
    else:
        s = report["summary"]
        print(f"Coherence: {s['errors']} error(s), {s['warnings']} warning(s), {s['infos']} info")
        print(f"Categories: {s.get('by_category', {})}")

    # Profile-driven blocking: strict upgrades configured categories (stale
    # manifests, missing decisions, missing test linkage, unresolved objective
    # links, etc.) to errors — independent of the legacy --fail-on flag.
    profile_blocked = [
        f for f in report.get("findings", [])
        if profile.blocks(str(f.get("category", "")))
    ]
    if profile_blocked:
        print(
            f"coherence[{profile.name}]: {len(profile_blocked)} blocking finding(s) "
            f"under profile {profile.name}.",
            file=sys.stderr,
        )
        return 1

    if args.fail_on == "error" and report["summary"]["errors"] > 0:
        return 1
    if args.fail_on == "warning" and (
        report["summary"]["errors"] > 0 or report["summary"]["warnings"] > 0
    ):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
