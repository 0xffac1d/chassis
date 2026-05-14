"""Cross-field semantic checks for CONTRACT.yaml / chassis.unit.yaml (after JSON Schema)."""
from __future__ import annotations

import re
from functools import lru_cache
from pathlib import Path
from typing import Any, Dict, List, Set, Tuple

# Vague single-word qualifiers that alone are not testable
_VAGUE_TERMS = frozenset(
    {
        "correct",
        "correctly",
        "valid",
        "properly",
        "appropriate",
        "appropriately",
        "handles",
        "handle",
        "works",
        "work",
        "good",
        "fine",
        "ok",
        "okay",
        "right",
        "proper",
        "managed",
        "acceptable",
    }
)

# Minimum meaningful length for an invariant statement
_MIN_INVARIANT_LEN = 15

# Patterns that indicate a placeholder was not replaced
def _claim_line_text(claim: Any) -> str:
    """Behavioral text for invariant or edge_case (string or {id?, text})."""
    if isinstance(claim, dict):
        t = claim.get("text")
        if t is not None:
            return str(t)
        return ""
    return str(claim) if claim is not None else ""


def _iter_invariants(doc: Dict[str, Any]) -> List[Any]:
    inv = doc.get("invariants")
    return list(inv) if isinstance(inv, list) else []


def _iter_edge_cases(doc: Dict[str, Any]) -> List[Any]:
    ec = doc.get("edge_cases")
    return list(ec) if isinstance(ec, list) else []


def _collect_claim_ids(items: List[Any]) -> Set[str]:
    ids: Set[str] = set()
    for item in items:
        if isinstance(item, dict):
            cid = item.get("id")
            if isinstance(cid, str) and cid.strip():
                ids.add(cid.strip())
    return ids


@lru_cache(maxsize=8)
def _load_objective_ids(repo_root_str: str) -> Set[str]:
    p = Path(repo_root_str) / "config" / "chassis.objectives.yaml"
    if not p.is_file():
        return frozenset()  # type: ignore[return-value]
    try:
        import yaml  # type: ignore

        raw = yaml.safe_load(p.read_text(encoding="utf-8"))
    except Exception:
        return frozenset()  # type: ignore[return-value]
    if not isinstance(raw, dict):
        return frozenset()  # type: ignore[return-value]
    out: Set[str] = set()
    for o in raw.get("objectives") or []:
        if isinstance(o, dict):
            oid = o.get("id")
            if isinstance(oid, str):
                out.add(oid.strip())
    return frozenset(out)  # type: ignore[return-value]


_PLACEHOLDER_PATTERNS = (
    "placeholder",
    "todo",
    "tbd",
    "fixme",
    "replace this",
    "fill in",
    "add invariant",
    "add edge case",
    "<invariant>",
    "[invariant]",
)


def check_invariant_quality(
    repo_root: Path,
    file_path: Path,
    doc: Dict[str, Any],
) -> List[Tuple[str, str]]:
    """
    Lint invariants and edge_cases for substance.
    Returns (code, message) issues.
    """
    issues: List[Tuple[str, str]] = []
    try:
        rel = str(file_path.relative_to(repo_root))
    except ValueError:
        rel = str(file_path)

    def _check_claim(field: str, idx: int, text: str) -> None:
        s = text.strip().lower()

        for pat in _PLACEHOLDER_PATTERNS:
            if pat in s:
                issues.append(
                    (
                        "CHASSIS_INV_PLACEHOLDER",
                        f"{rel}: {field}[{idx}] appears to be an unreplaced placeholder: {text!r}",
                    )
                )
                return

        if len(text.strip()) < _MIN_INVARIANT_LEN:
            issues.append(
                (
                    "CHASSIS_INV_TOO_SHORT",
                    f"{rel}: {field}[{idx}] is too short to be testable "
                    f"({len(text.strip())} chars < {_MIN_INVARIANT_LEN}): {text!r}",
                )
            )
            return

        words = re.findall(r"[a-z]+", s)
        stop_words = frozenset(
            {
                "the",
                "a",
                "an",
                "is",
                "are",
                "be",
                "been",
                "was",
                "were",
                "it",
                "its",
                "this",
                "that",
                "and",
                "or",
                "not",
                "no",
                "in",
                "on",
                "at",
                "to",
                "for",
                "of",
                "with",
                "by",
                "from",
                "as",
                "module",
                "function",
                "component",
                "service",
                "class",
                "method",
                "always",
                "never",
                "must",
                "should",
                "will",
                "when",
                "if",
                "all",
                "any",
                "every",
                "each",
                "returns",
                "return",
                "throws",
                "throw",
                "emits",
                "emit",
                "calls",
                "call",
                "data",
                "value",
                "values",
            }
        )
        substantive = [w for w in words if w not in stop_words and w not in _VAGUE_TERMS]
        if not substantive:
            issues.append(
                (
                    "CHASSIS_INV_VAGUE",
                    f"{rel}: {field}[{idx}] contains no testable specifics — only vague qualifiers: {text!r}",
                )
            )

    for field in ("invariants", "edge_cases"):
        claims = doc.get(field)
        if not isinstance(claims, list):
            continue
        for i, claim in enumerate(claims):
            text = _claim_line_text(claim)
            if not text.strip():
                continue
            _check_claim(field, i, text)

    return issues


def _legacy_claim_match(claim_id: str, doc: Dict[str, Any]) -> bool:
    """Whether claim_id matches a full string claim or excerpt (pre-stable-id linkage)."""
    s = claim_id.strip()
    if not s:
        return False
    for field in ("invariants", "edge_cases"):
        for item in _iter_invariants(doc) if field == "invariants" else _iter_edge_cases(doc):
            t = _claim_line_text(item).strip()
            if t == s:
                return True
            if len(s) >= 12 and (s in t or t.startswith(s)):
                return True
    return False


def _looks_like_stable_claim_id(claim_id: str) -> bool:
    """Slug-style id (no spaces) — mismatches are worth surfacing even without id: fields."""
    s = claim_id.strip()
    if not s or " " in s:
        return False
    return bool(re.fullmatch(r"[a-z][a-z0-9_.-]*", s))


def check_test_linkage_claim_ids(
    repo_root: Path,
    file_path: Path,
    doc: Dict[str, Any],
) -> List[Tuple[str, str]]:
    """Prefer stable claim ids in test_linkage; warn on orphan or mixed migration state."""
    issues: List[Tuple[str, str]] = []
    try:
        rel = str(file_path.relative_to(repo_root))
    except ValueError:
        rel = str(file_path)

    inv_items = _iter_invariants(doc)
    ec_items = _iter_edge_cases(doc)
    id_set = _collect_claim_ids(inv_items) | _collect_claim_ids(ec_items)

    dict_inv = sum(1 for x in inv_items if isinstance(x, dict))
    str_inv = sum(1 for x in inv_items if isinstance(x, str))
    has_partial_ids = dict_inv > 0 and any(
        isinstance(x, dict) and not (isinstance(x.get("id"), str) and str(x["id"]).strip())
        for x in inv_items
    )
    if has_partial_ids and id_set:
        issues.append(
            (
                "CHASSIS_CLAIM_PARTIAL_IDS",
                f"{rel}: some dict-form invariants lack `id` while others have ids — "
                "add stable id to every dict invariant or use plain strings until migration",
            )
        )

    links = doc.get("test_linkage")
    if not isinstance(links, list) or not links:
        return issues

    for i, entry in enumerate(links):
        if not isinstance(entry, dict):
            continue
        cid = entry.get("claim_id")
        if not isinstance(cid, str) or not cid.strip():
            issues.append(
                ("CHASSIS_CLAIM_LINK_SHAPE", f"{rel}: test_linkage[{i}] missing claim_id"),
            )
            continue
        cid = cid.strip()
        if id_set and cid in id_set:
            continue
        if id_set and _legacy_claim_match(cid, doc):
            issues.append(
                (
                    "CHASSIS_CLAIM_ID_LEGACY",
                    f"{rel}: test_linkage claim_id {cid!r} matches text excerpt, not a stable "
                    "`id` field — migrate to invariant/edge_case ids",
                )
            )
            continue
        if id_set:
            issues.append(
                (
                    "CHASSIS_CLAIM_ID_UNRESOLVED",
                    f"{rel}: test_linkage claim_id {cid!r} does not match any invariant/edge_case id",
                )
            )
            continue
        # No stable ids on claims yet: only flag slug-style claim_ids that do not resolve.
        if _looks_like_stable_claim_id(cid) and not _legacy_claim_match(cid, doc):
            issues.append(
                (
                    "CHASSIS_CLAIM_ID_UNRESOLVED",
                    f"{rel}: test_linkage claim_id {cid!r} does not match any invariant or edge_case text",
                )
            )

    return issues


def check_linked_objectives(
    repo_root: Path,
    file_path: Path,
    doc: Dict[str, Any],
) -> List[Tuple[str, str]]:
    """Resolve linked_objectives against config/chassis.objectives.yaml."""
    issues: List[Tuple[str, str]] = []
    lo = doc.get("linked_objectives")
    if not isinstance(lo, list) or not lo:
        return issues
    try:
        rel = str(file_path.relative_to(repo_root))
    except ValueError:
        rel = str(file_path)

    known = set(_load_objective_ids(str(repo_root.resolve())))
    if not known:
        issues.append(
            (
                "CHASSIS_OBJ_REGISTRY_MISSING",
                f"{rel}: linked_objectives set but config/chassis.objectives.yaml missing or empty",
            )
        )
        return issues

    for i, oid in enumerate(lo):
        if not isinstance(oid, str) or not oid.strip():
            issues.append(("CHASSIS_OBJ_LINK_SHAPE", f"{rel}: linked_objectives[{i}] must be a non-empty string"))
            continue
        if oid.strip() not in known:
            issues.append(
                (
                    "CHASSIS_OBJ_UNKNOWN",
                    f"{rel}: linked_objectives references unknown objective id {oid.strip()!r}",
                )
            )
    return issues


def check_precedence_signals(
    repo_root: Path,
    file_path: Path,
    doc: Dict[str, Any],
    is_contract: bool,
) -> List[Tuple[str, str]]:
    """
    Warn when a stable contract has no test_linkage (precedence model: tests > CONTRACT claims).
    See PROTOCOL.md source-of-truth precedence section.
    """
    issues: List[Tuple[str, str]] = []
    if not is_contract:
        return issues

    try:
        rel = str(file_path.relative_to(repo_root))
    except ValueError:
        rel = str(file_path)

    status = doc.get("status", "")

    # AR-11: superseded_by required when status=superseded (not expressible in draft-07 without if/then).
    if status == "superseded" and not doc.get("superseded_by"):
        issues.append(
            (
                "CHASSIS_PREC_SUPERSEDED_NO_REF",
                f"{rel}: status=superseded requires superseded_by to be set.",
            )
        )

    if status == "stable":
        test_linkage = doc.get("test_linkage", [])
        invariants = doc.get("invariants", [])
        if invariants and not test_linkage:
            issues.append(
                (
                    "CHASSIS_PREC_NO_LINKAGE",
                    f"{rel}: status=stable with {len(invariants)} invariant(s) but no test_linkage entries. "
                    "Per precedence model: tests outrank CONTRACT claims — link tests to invariants via "
                    "test_linkage field.",
                )
            )

    return issues


def check_manifest_semantics(
    repo_root: Path,
    file_path: Path,
    doc: Dict[str, Any],
    is_contract: bool,
) -> List[Tuple[str, str]]:
    """
    Returns list of (code, message) issues (warnings or soft errors).
    Caller decides severity.
    """
    issues: List[Tuple[str, str]] = []
    try:
        rel = str(file_path.relative_to(repo_root))
    except ValueError:
        rel = str(file_path)

    todos = doc.get("todos")
    if isinstance(todos, list):
        for i, t in enumerate(todos):
            if not isinstance(t, dict):
                issues.append(("CHASSIS_SEM_TODO_SHAPE", f"{rel}: todos[{i}] must be an object"))
                continue
            if not t.get("description"):
                issues.append(("CHASSIS_SEM_TODO_DESC", f"{rel}: todos[{i}] missing description"))
            pri = t.get("priority")
            if pri is not None and pri not in ("low", "medium", "high", "critical"):
                issues.append(("CHASSIS_SEM_TODO_PRI", f"{rel}: todos[{i}] invalid priority {pri!r}"))

    depends_on = doc.get("depends_on")
    if isinstance(depends_on, list) and depends_on:
        parent_contract = file_path.parent / "CONTRACT.yaml"
        if file_path.name == "chassis.unit.yaml" and parent_contract.is_file():
            try:
                import yaml  # type: ignore

                parent = yaml.safe_load(parent_contract.read_text(encoding="utf-8"))
            except Exception:
                parent = None
            if isinstance(parent, dict):
                pdeps = parent.get("depends_on")
                if isinstance(pdeps, list):
                    for dep in depends_on:
                        if isinstance(dep, str) and dep and dep not in pdeps:
                            issues.append(
                                (
                                    "CHASSIS_SEM_UNIT_DEP",
                                    f"{rel}: depends_on entry {dep!r} not listed in parent CONTRACT.yaml depends_on",
                                )
                            )

    if is_contract:
        exports = doc.get("exports")
        drift = doc.get("drift")
        if isinstance(drift, dict) and drift.get("ignore_paths") and exports in (None, []):
            issues.append(
                (
                    "CHASSIS_SEM_DRIFT_EXPORTS",
                    f"{rel}: drift.ignore_paths set but exports empty — verify drift config is intentional",
                )
            )

    issues.extend(check_invariant_quality(repo_root, file_path, doc))
    issues.extend(check_precedence_signals(repo_root, file_path, doc, is_contract))
    if is_contract:
        issues.extend(check_linked_objectives(repo_root, file_path, doc))
        issues.extend(check_test_linkage_claim_ids(repo_root, file_path, doc))

    return issues
