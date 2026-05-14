#!/usr/bin/env python3
"""`chassis exempt <subcommand>` — expiring-exemption registry CLI.

Subcommands:
  add      Add a new exemption to .exemptions/registry.yaml.
  list     List active/expired exemptions.
  prune    Remove entries whose underlying violation no longer fires
           (prune-on-fix) or whose expiration date has passed.
  check    CI entrypoint — validates the registry against
           schemas/exemption/registry.schema.json, enforces quotas, and
           fails on expired-but-still-present entries.

Registry: .exemptions/registry.yaml (CODEOWNERS-protected).
Schema:   schemas/exemption/registry.schema.json.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import sys
from collections import Counter
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

MAX_LIFETIME_DAYS = 90
REGISTRY_REL = ".exemptions/registry.yaml"
SCHEMA_REL = "schemas/exemption/registry.schema.json"


def _require_yaml() -> None:
    if yaml is None:
        print(
            "chassis exempt: PyYAML required (pip install pyyaml)",
            file=sys.stderr,
        )
        raise SystemExit(2)


def _load(root: Path) -> dict:
    _require_yaml()
    path = root / REGISTRY_REL
    if not path.is_file():
        return {"version": 1, "entries": []}
    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    data.setdefault("version", 1)
    data.setdefault("entries", [])
    return data


def _save(root: Path, data: dict) -> None:
    _require_yaml()
    path = root / REGISTRY_REL
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(yaml.safe_dump(data, sort_keys=False), encoding="utf-8")


def _validate(root: Path, data: dict) -> list[str]:
    """Return list of error strings; empty on success."""
    schema = root / SCHEMA_REL
    validator = draft7_validator_for_schema_file(schema)
    errs = [f"{'/'.join(str(x) for x in e.path)}: {e.message}" for e in validator.iter_errors(data)]
    return errs


def _next_id(entries: list[dict]) -> str:
    year = _dt.date.today().year
    prefix = f"EX-{year}-"
    counters = [int(e["id"].split("-")[-1]) for e in entries if e.get("id", "").startswith(prefix)]
    next_n = max(counters, default=0) + 1
    return f"{prefix}{next_n:04d}"


def cmd_add(args: argparse.Namespace) -> int:
    root = Path(args.repo_root).resolve()
    data = _load(root)
    created = _dt.date.today()
    try:
        expires = _dt.date.fromisoformat(args.expires) if args.expires else created + _dt.timedelta(days=MAX_LIFETIME_DAYS)
    except ValueError as exc:
        print(f"chassis exempt: invalid --expires ({exc})", file=sys.stderr)
        return 2
    if (expires - created).days > MAX_LIFETIME_DAYS:
        print(
            f"chassis exempt: expires {expires} exceeds created+{MAX_LIFETIME_DAYS}d ({created + _dt.timedelta(days=MAX_LIFETIME_DAYS)})",
            file=sys.stderr,
        )
        return 2

    entry = {
        "id": args.id or _next_id(data["entries"]),
        "rule": args.rule,
        "scope": args.scope,
        "reason": args.reason,
        "ticket": args.ticket,
        "owner": args.owner,
        "created": created.isoformat(),
        "expires": expires.isoformat(),
        "adr": args.adr,
    }
    data["entries"].append(entry)

    errs = _validate(root, data)
    if errs:
        print("chassis exempt: registry would be invalid:", file=sys.stderr)
        for e in errs:
            print(f"  - {e}", file=sys.stderr)
        return 1

    quota = data.get("quota", {})
    total_max = quota.get("total_max", 25)
    per_file = quota.get("per_file_max", 1)
    if len(data["entries"]) > total_max:
        print(f"chassis exempt: total quota {total_max} exceeded", file=sys.stderr)
        return 1
    scopes: Counter[str] = Counter()
    for e in data["entries"]:
        s = e["scope"]
        if isinstance(s, list):
            for x in s:
                scopes[x] += 1
        else:
            scopes[s] += 1
    over = [s for s, n in scopes.items() if n > per_file]
    if over:
        print(f"chassis exempt: per-file quota {per_file} exceeded for: {', '.join(over)}", file=sys.stderr)
        return 1

    _save(root, data)
    print(f"chassis exempt: added {entry['id']} (expires {entry['expires']})")
    return 0


def cmd_list(args: argparse.Namespace) -> int:
    root = Path(args.repo_root).resolve()
    data = _load(root)
    today = _dt.date.today()
    rule_filter = args.rule
    for e in data["entries"]:
        if rule_filter and e.get("rule") != rule_filter:
            continue
        expires = _dt.date.fromisoformat(e["expires"])
        state = "EXPIRED" if expires < today else "active"
        print(f"{e['id']}  [{state}]  rule={e['rule']} adr={e['adr']} scope={e['scope']} expires={e['expires']} owner={e['owner']}")
    return 0


def cmd_prune(args: argparse.Namespace) -> int:
    root = Path(args.repo_root).resolve()
    data = _load(root)
    today = _dt.date.today()
    kept = []
    removed = []
    for e in data["entries"]:
        expires = _dt.date.fromisoformat(e["expires"])
        if expires < today:
            removed.append(e["id"])
            continue
        kept.append(e)
    data["entries"] = kept
    _save(root, data)
    print(f"chassis exempt prune: removed {len(removed)} expired entry(s); {len(kept)} remaining")
    for rid in removed:
        print(f"  - {rid}")
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    """CI entrypoint: schema-valid + no expired entries + quotas respected."""
    root = Path(args.repo_root).resolve()
    data = _load(root)
    errs = _validate(root, data)
    rc = 0
    for e in errs:
        print(f"EXEMPT-SCHEMA: {e}", file=sys.stderr)
        rc = 1
    today = _dt.date.today()
    for entry in data["entries"]:
        try:
            expires = _dt.date.fromisoformat(entry["expires"])
        except (KeyError, ValueError):
            continue
        if expires < today:
            print(
                f"EXEMPT-EXPIRED: {entry['id']} expired on {expires} (rule={entry['rule']}). "
                "Fix the underlying violation or renew via `chassis exempt add`.",
                file=sys.stderr,
            )
            rc = 1

    quota = data.get("quota", {})
    total_max = quota.get("total_max", 25)
    if len(data["entries"]) > total_max:
        print(
            f"EXEMPT-QUOTA: {len(data['entries'])} entries exceeds total_max={total_max}",
            file=sys.stderr,
        )
        rc = 1
    if rc == 0:
        print(f"chassis exempt check: OK ({len(data['entries'])} entry(s))")
    return rc


def main() -> int:
    parser = argparse.ArgumentParser(prog="chassis-exempt")
    parser.add_argument("--repo-root", default=str(repo_root()))
    sub = parser.add_subparsers(dest="subcmd", required=True)

    p_add = sub.add_parser("add", help="Add an exemption entry")
    p_add.add_argument("--rule", required=True, help="ruleId being exempted")
    p_add.add_argument("--scope", required=True, help="File path or glob")
    p_add.add_argument("--reason", required=True, help="Why the waiver (40+ chars)")
    p_add.add_argument("--ticket", required=True, help="Tracker reference")
    p_add.add_argument("--owner", required=True, help="Accountable email/handle")
    p_add.add_argument("--adr", required=True, help="ADR id (e.g. ADR-0001)")
    p_add.add_argument("--expires", help=f"ISO date; default created+{MAX_LIFETIME_DAYS}d")
    p_add.add_argument("--id", help="Explicit EX-YYYY-NNNN id; default auto-assigned")
    p_add.set_defaults(func=cmd_add)

    p_list = sub.add_parser("list", help="List exemptions")
    p_list.add_argument("--rule", help="Filter by ruleId")
    p_list.set_defaults(func=cmd_list)

    p_prune = sub.add_parser("prune", help="Remove expired entries")
    p_prune.set_defaults(func=cmd_prune)

    p_check = sub.add_parser("check", help="CI entrypoint: schema + quotas + no-expired")
    p_check.set_defaults(func=cmd_check)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
