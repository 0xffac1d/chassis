#!/usr/bin/env python3
"""`chassis adr <subcommand>` — Architecture Decision Record tooling.

Subcommands:
  validate     Validate every ADR frontmatter in docs/adr/ against
               schemas/decision/adr.schema.json. Also verifies status/supersedes
               consistency (an ADR listed in another's `supersedes` must itself
               have status: superseded and superseded_by pointing back).
  index        Emit docs/index.json — a registry of {id, title, status,
               enforces[].rule, applies_to, file}. Consumed by the
               binding-link gate and by MCP `listConventions`.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_SCRIPTS_CHASSIS = Path(__file__).resolve().parent
if str(_SCRIPTS_CHASSIS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_CHASSIS))

try:
    import yaml  # type: ignore
except ImportError:
    yaml = None  # type: ignore

from jsonschema_support import draft7_validator_for_schema_file
from repo_layout import repo_root

FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---\n", re.DOTALL)


def _adr_dirs(root: Path) -> list[Path]:
    out = []
    for rel in ("docs/adr", "docs/chassis/decisions"):
        d = root / rel
        if d.is_dir():
            out.append(d)
    return out


def _parse_frontmatter(path: Path) -> dict | None:
    text = path.read_text(encoding="utf-8")
    m = FRONTMATTER_RE.match(text)
    if not m:
        return None
    if yaml is None:
        raise RuntimeError(
            "PyYAML required for ADR frontmatter parsing (pip install pyyaml)"
        )
    return yaml.safe_load(m.group(1)) or {}


def _collect(root: Path) -> list[tuple[Path, dict]]:
    out = []
    for d in _adr_dirs(root):
        for p in sorted(d.rglob("*.md")):
            fm = _parse_frontmatter(p)
            if fm is None:
                continue
            out.append((p, fm))
    return out


def cmd_validate(args: argparse.Namespace) -> int:
    root = Path(args.repo_root).resolve()
    schema = root / "schemas" / "decision" / "adr.schema.json"
    if not schema.is_file():
        print(f"chassis adr: missing {schema}", file=sys.stderr)
        return 2
    validator = draft7_validator_for_schema_file(schema)

    errors = 0
    adrs = _collect(root)
    ids: dict[str, Path] = {}
    for p, fm in adrs:
        rel = p.relative_to(root)
        errs = sorted(validator.iter_errors(fm), key=lambda e: list(e.path))
        for e in errs:
            print(f"ERROR {rel}: {'.'.join(str(x) for x in e.path)}: {e.message}")
            errors += 1
        aid = fm.get("id")
        if aid:
            if aid in ids:
                print(f"ERROR {rel}: duplicate ADR id {aid} (also in {ids[aid].relative_to(root)})")
                errors += 1
            ids[aid] = p

    # Cross-check supersedes / superseded_by
    by_id = {fm["id"]: (p, fm) for p, fm in adrs if fm.get("id")}
    for p, fm in adrs:
        rel = p.relative_to(root)
        for ref in fm.get("supersedes", []) or []:
            target = by_id.get(ref)
            if target is None:
                print(f"ERROR {rel}: supersedes unknown ADR {ref}")
                errors += 1
                continue
            tp, tfm = target
            if tfm.get("status") != "superseded":
                print(
                    f"ERROR {rel}: supersedes {ref} but that ADR has status={tfm.get('status')!r} (expected 'superseded')"
                )
                errors += 1
            if tfm.get("superseded_by") != fm.get("id"):
                print(
                    f"ERROR {rel}: supersedes {ref} but target's superseded_by is {tfm.get('superseded_by')!r}"
                )
                errors += 1

    if errors:
        print(f"chassis adr validate: {errors} error(s) across {len(adrs)} ADR(s)")
        return 1
    print(f"chassis adr validate: OK ({len(adrs)} ADR(s))")
    return 0


def cmd_index(args: argparse.Namespace) -> int:
    root = Path(args.repo_root).resolve()
    adrs = _collect(root)
    entries = []
    for p, fm in adrs:
        entries.append(
            {
                "id": fm.get("id"),
                "title": fm.get("title"),
                "status": fm.get("status"),
                "date": str(fm.get("date")) if fm.get("date") else None,
                "enforces": [e.get("rule") for e in fm.get("enforces", []) or []],
                "applies_to": fm.get("applies_to", []),
                "file": str(p.relative_to(root)),
            }
        )
    entries.sort(key=lambda e: e.get("id") or "")
    out_path = root / "docs" / "index.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {"schema": "chassis-adr-index-v1", "adrs": entries}
    out_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"chassis adr index: wrote {out_path.relative_to(root)} ({len(entries)} entries)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="chassis-adr")
    parser.add_argument("--repo-root", default=str(repo_root()))
    sub = parser.add_subparsers(dest="subcmd", required=True)
    p_val = sub.add_parser("validate", help="Validate ADR frontmatter + supersedes chain")
    p_val.set_defaults(func=cmd_validate)
    p_idx = sub.add_parser("index", help="Emit docs/index.json")
    p_idx.set_defaults(func=cmd_index)
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
