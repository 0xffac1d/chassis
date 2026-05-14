#!/usr/bin/env python3
"""Validate ``.chassis/claims.yaml`` — public readiness registry for this checkout."""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

try:
    import yaml  # type: ignore
except ImportError:
    yaml = None  # type: ignore


_ALLOWED_STATUS = frozenset({"implemented", "partial", "planned", "unsupported"})
_REQUIRED_KEYS = frozenset(
    {"id", "status", "statement", "implementation_paths", "test_paths", "validation_command"}
)


def _err(msg: str) -> None:
    print(f"claims validate: ERROR: {msg}", file=sys.stderr)


def _normalize_paths(repo: Path, rels: object) -> list[str]:
    if rels is None:
        return []
    if isinstance(rels, str):
        candidates = [rels]
    elif isinstance(rels, list) and all(isinstance(x, str) for x in rels):
        candidates = list(rels)
    else:
        raise ValueError("paths must be a string or list of strings")
    repo_resolved = repo.resolve()
    for rel in candidates:
        if not rel:
            raise ValueError("paths must be non-empty strings")
        # Reject absolute paths and traversal that would escape repo. We
        # resolve against the repo root and ensure the result is within the
        # repo subtree before passing it on to existence checks.
        if Path(rel).is_absolute():
            raise ValueError(f"path must be repo-relative, got absolute: {rel!r}")
        target = (repo_resolved / rel).resolve()
        if (
            target != repo_resolved
            and repo_resolved not in target.parents
        ):
            raise ValueError(f"path escapes repository root: {rel!r}")
    return candidates


def validate_registry(repo: Path) -> int:
    if yaml is None:
        _err("PyYAML is required (pip install pyyaml)")
        return 2

    path = repo / ".chassis" / "claims.yaml"
    if not path.is_file():
        _err(f"missing {path.relative_to(repo)}")
        return 1

    try:
        doc = yaml.safe_load(path.read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001
        _err(f"could not parse YAML: {e}")
        return 1

    if not isinstance(doc, dict):
        _err("root must be a mapping")
        return 1

    version = doc.get("claims_registry_version")
    if version != "1.0":
        _err("claims_registry_version must be '1.0'")
        return 1

    claims = doc.get("claims")
    if not isinstance(claims, list) or not claims:
        _err("claims must be a non-empty list")
        return 1

    errors = 0
    for i, c in enumerate(claims):
        prefix = f"claims[{i}]"
        if not isinstance(c, dict):
            _err(f"{prefix}: must be an object")
            errors += 1
            continue
        missing = _REQUIRED_KEYS - c.keys()
        if missing:
            _err(f"{prefix}: missing keys {sorted(missing)}")
            errors += 1
            continue
        cid = c.get("id")
        if not isinstance(cid, str) or not cid.strip():
            _err(f"{prefix}: id must be a non-empty string")
            errors += 1
            continue

        st = c.get("status")
        if st not in _ALLOWED_STATUS:
            _err(f"{prefix} ({cid}): status must be one of {sorted(_ALLOWED_STATUS)}")
            errors += 1
            continue

        stmt = c.get("statement")
        if not isinstance(stmt, str) or not stmt.strip():
            _err(f"{prefix} ({cid}): statement must be non-empty prose")
            errors += 1

        val_cmd = c.get("validation_command")
        if not isinstance(val_cmd, str) or not val_cmd.strip():
            _err(f"{prefix} ({cid}): validation_command must be a non-empty string")
            errors += 1

        try:
            impl = _normalize_paths(repo, c.get("implementation_paths"))
            tests = _normalize_paths(repo, c.get("test_paths"))
        except ValueError as e:
            _err(f"{prefix} ({cid}): {e}")
            errors += 1
            continue

        if st == "implemented":
            for rel in impl:
                p = repo / rel
                if not p.exists():
                    _err(f"{prefix} ({cid}): implementation path missing: {rel}")
                    errors += 1
            for rel in tests:
                p = repo / rel
                if not p.exists():
                    _err(f"{prefix} ({cid}): test path missing: {rel}")
                    errors += 1
            if not impl:
                _err(f"{prefix} ({cid}): implemented claims require non-empty implementation_paths")
                errors += 1
            if not tests:
                _err(f"{prefix} ({cid}): implemented claims require non-empty test_paths")
                errors += 1
        else:
            gap = c.get("missing_requirement")
            if not isinstance(gap, str) or not gap.strip():
                _err(
                    f"{prefix} ({cid}): non-implemented status requires "
                    f"non-empty missing_requirement"
                )
                errors += 1
            for rel in impl:
                p = repo / rel
                if not p.exists():
                    _err(f"{prefix} ({cid}): referenced implementation path missing: {rel}")
                    errors += 1

    if errors:
        _err(f"{errors} issue(s) in {path.relative_to(repo)}")
        return 1

    print(f"claims validate: OK ({len(claims)} claim(s))")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--repo-root",
        type=Path,
        default=REPO_ROOT,
        help="Repository root (default: auto-detected)",
    )
    args = ap.parse_args()
    return validate_registry(args.repo_root.resolve())


if __name__ == "__main__":
    raise SystemExit(main())
