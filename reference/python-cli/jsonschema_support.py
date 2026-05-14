"""Draft-07 validators using ``referencing`` (replaces deprecated jsonschema.RefResolver).

Registers sibling ``*.schema.json`` files with bounded, cycle-safe reference rules:

* Only same-directory ``*.schema.json`` basenames may be linked (no ``../``, no URLs).
* The cross-file ``$ref`` graph must be acyclic.
* Registry construction is cached per metadata directory.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Dict, Optional, Set

from jsonschema import Draft7Validator
from referencing import Registry
from referencing.jsonschema import DRAFT7

# Maximum JSON Schema files loaded from a single metadata directory (pathological guard).
_MAX_SIBLING_SCHEMA_FILES = 64

_ALLOWED_LOCAL_REF = re.compile(r"^[A-Za-z0-9._-]+\.schema\.json$")

_registry_cache: Dict[str, Registry] = {}


class SchemaRefError(RuntimeError):
    """Invalid or unsupported ``$ref`` wiring under ``schemas/metadata``."""


def _normalize_ref_target(ref: str) -> Optional[str]:
    """Return a sibling basename for graph purposes, or None if not a local cross-file ref."""
    if not isinstance(ref, str) or not ref:
        return None
    if "://" in ref:
        raise SchemaRefError(f"external $ref not allowed in Chassis metadata schemas: {ref!r}")
    base = ref.split("#", 1)[0]
    if not base:
        return None
    if "/" in base or "\\" in base or base.startswith("."):
        raise SchemaRefError(f"$ref must be a same-directory basename, got {ref!r}")
    if not _ALLOWED_LOCAL_REF.fullmatch(base):
        raise SchemaRefError(f"$ref must match *.schema.json basename pattern, got {ref!r}")
    return base


def _collect_ref_targets(doc: Any, out: Set[str]) -> None:
    """Collect local cross-file ``$ref`` targets (basenames) from *doc*."""
    if isinstance(doc, dict):
        ref = doc.get("$ref")
        if isinstance(ref, str):
            tgt = _normalize_ref_target(ref)
            if tgt:
                out.add(tgt)
        for v in doc.values():
            _collect_ref_targets(v, out)
    elif isinstance(doc, list):
        for v in doc:
            _collect_ref_targets(v, out)


def _ref_graph_for_dir(schema_dir: Path) -> Dict[str, Set[str]]:
    """Directed graph: schema basename -> set of referenced basenames."""
    graph: Dict[str, Set[str]] = {}
    for p in sorted(schema_dir.glob("*.schema.json")):
        if len(graph) >= _MAX_SIBLING_SCHEMA_FILES:
            raise SchemaRefError(
                f"too many *.schema.json in {schema_dir} "
                f"(limit {_MAX_SIBLING_SCHEMA_FILES})"
            )
        try:
            doc = json.loads(p.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            raise SchemaRefError(f"invalid JSON in {p}: {e}") from e
        targets: Set[str] = set()
        _collect_ref_targets(doc, targets)
        for tgt in targets:
            if not (schema_dir / tgt).is_file():
                raise SchemaRefError(f"{p.name} references missing schema {tgt!r}")
        graph[p.name] = targets
    return graph


def _assert_acyclic(graph: Dict[str, Set[str]]) -> None:
    visited: Set[str] = set()
    stack: Set[str] = set()

    def dfs(node: str) -> None:
        if node in stack:
            raise SchemaRefError(f"cyclic $ref graph involving {node!r}")
        if node in visited:
            return
        visited.add(node)
        stack.add(node)
        for nbr in graph.get(node, ()):
            dfs(nbr)
        stack.remove(node)

    for n in graph:
        if n not in visited:
            dfs(n)


def build_registry_for_metadata_dir(schema_dir: Path) -> Registry:
    """Load every sibling ``*.schema.json`` into a Draft-07 registry (cached)."""
    schema_dir = schema_dir.resolve()
    key = str(schema_dir)
    if key in _registry_cache:
        return _registry_cache[key]

    paths = sorted(schema_dir.glob("*.schema.json"))
    if not paths:
        raise FileNotFoundError(f"No JSON Schema files found in {schema_dir}")
    if len(paths) > _MAX_SIBLING_SCHEMA_FILES:
        raise SchemaRefError(
            f"too many *.schema.json in {schema_dir} (limit {_MAX_SIBLING_SCHEMA_FILES})"
        )

    graph = _ref_graph_for_dir(schema_dir)
    _assert_acyclic(graph)

    registry: Optional[Registry] = None
    for p in paths:
        doc = json.loads(p.read_text(encoding="utf-8"))
        uri = p.resolve().as_uri()
        resource = DRAFT7.create_resource(doc)
        registry = (registry or Registry()).with_resource(uri, resource)
        registry = registry.with_resource(p.name, resource)

    assert registry is not None
    _registry_cache[key] = registry
    return registry


def draft7_validator_for_schema_file(schema_path: Path) -> Draft7Validator:
    """Return a ``Draft7Validator`` for *schema_path* with bounded sibling ref registration."""
    schema_path = schema_path.resolve()
    registry = build_registry_for_metadata_dir(schema_path.parent)
    main = json.loads(schema_path.read_text(encoding="utf-8"))
    return Draft7Validator(main, registry=registry)


def clear_schema_registry_cache() -> None:
    """Test helper: drop cached registries."""
    _registry_cache.clear()
