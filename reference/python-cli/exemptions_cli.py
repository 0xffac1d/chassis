"""``chassis exemptions <subcommand>`` — strict exemption lifecycle CLI.

Subcommands
-----------
``list``           List entries (active / expired / revoked / invalid).
``add``            Append a new active entry, narrowly scoped, with an expiry.
``validate``       Validate the registry: schema, lifecycle, scope, quotas.
``prune-expired``  Revoke (default) or delete expired entries. Modifies the
                   registry only — never source files.
``explain``        Print a structured explanation of one exemption (active
                   reasons, lifecycle state, days remaining, owner, linked
                   issue, etc.).

Exit codes
----------
* ``0`` on success.
* ``1`` when any invalid / expired entry is detected (``validate`` and ``list``
  also honor ``--strict``).
* ``2`` for usage errors.

This CLI is intentionally distinct from the legacy ``chassis exempt`` command
so the lifecycle contract — visible, scoped, owned, expiring — can evolve
without breaking existing call sites.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import sys
from pathlib import Path
from typing import Any, Optional, Sequence

_SCRIPTS_CHASSIS = Path(__file__).resolve().parent
if str(_SCRIPTS_CHASSIS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_CHASSIS))

import exemptions as ex  # noqa: E402
from repo_layout import chassis_assets_root, target_repo_root  # noqa: E402


def _resolve_repo(value: Optional[str]) -> Path:
    if value:
        return Path(value).expanduser().resolve()
    try:
        return target_repo_root().resolve()
    except RuntimeError:
        return Path.cwd().resolve()


def _resolve_assets(value: Optional[str]) -> Path:
    if value:
        return Path(value).expanduser().resolve()
    try:
        return chassis_assets_root()
    except RuntimeError:
        return Path(__file__).resolve().parents[2]


# ---------------------------------------------------------------------------
# list
# ---------------------------------------------------------------------------


def cmd_list(args: argparse.Namespace) -> int:
    repo = _resolve_repo(args.repo_root)
    assets = _resolve_assets(args.assets_root)
    report = ex.load_registry(repo, assets_root=assets)
    today = _dt.date.today()

    if args.format == "json":
        sys.stdout.write(json.dumps(report.to_json(), indent=2, sort_keys=True) + "\n")
        return _list_exit_code(report, args)

    sys.stdout.write(f"chassis exemptions @ {repo}\n")
    sys.stdout.write(
        f"  active={len(report.active())}  expired={len(report.expired())}  "
        f"revoked={len(report.revoked())}  invalid={len(report.invalid)}\n\n"
    )
    for entry in report.entries:
        if entry.is_revoked:
            state = "REVOKED"
        elif entry.is_expired:
            state = "EXPIRED"
        else:
            state = "active"
        rule = entry.rule_id or entry.finding_id or "?"
        scope_str = ", ".join(entry.paths) if entry.paths else "?"
        days = (entry.expires_at - today).days
        sys.stdout.write(
            f"  {entry.id}  [{state}]  rule={rule}  scope={scope_str}  "
            f"expires={entry.expires_at.isoformat()} ({days:+d}d)  owner={entry.owner}\n"
        )
    if report.invalid:
        sys.stdout.write("\nInvalid entries:\n")
        for inv in report.invalid:
            sys.stdout.write(f"  - {inv.id or '(no id)'}: {inv.reason}\n")
    if report.schema_errors:
        sys.stdout.write("\nSchema errors:\n")
        for err in report.schema_errors:
            sys.stdout.write(f"  - {err}\n")
    return _list_exit_code(report, args)


def _list_exit_code(report: ex.RegistryReport, args: argparse.Namespace) -> int:
    if args.strict and (report.invalid or report.expired() or report.schema_errors):
        return 1
    return 0


# ---------------------------------------------------------------------------
# add
# ---------------------------------------------------------------------------


def cmd_add(args: argparse.Namespace) -> int:
    repo = _resolve_repo(args.repo_root)
    paths: Any = args.path or args.scope
    if isinstance(paths, list) and len(paths) == 1:
        paths = paths[0]
    if not paths:
        sys.stderr.write("chassis exemptions add: --path or --scope is required\n")
        return 2
    if args.expires_at:
        try:
            expires = _dt.date.fromisoformat(args.expires_at)
        except ValueError as exc:
            sys.stderr.write(f"chassis exemptions add: invalid --expires-at ({exc})\n")
            return 2
    else:
        expires = None

    if not (args.rule_id or args.finding_id):
        sys.stderr.write("chassis exemptions add: --rule-id or --finding-id is required\n")
        return 2

    try:
        entry = ex.add_entry(
            repo,
            rule_id=args.rule_id,
            finding_id=args.finding_id,
            path=paths,
            reason=args.reason,
            owner=args.owner,
            linked_issue=args.linked_issue,
            adr=args.adr,
            expires_at=expires,
            severity_override=args.severity_override,
            explicit_id=args.id,
            allow_global=bool(args.allow_global),
        )
    except SystemExit as exc:  # raised with a string message
        sys.stderr.write(f"chassis exemptions add: {exc}\n")
        return 2

    # Re-validate the resulting registry to surface any post-add issues.
    assets = _resolve_assets(args.assets_root)
    report = ex.load_registry(repo, assets_root=assets)
    matching = report.by_id(entry["id"])
    if matching is None:
        sys.stderr.write(
            f"chassis exemptions add: entry {entry['id']} was written but failed validation; "
            "see invalid_exemptions[] in `chassis exemptions validate --format json`\n"
        )
        return 1

    if args.format == "json":
        sys.stdout.write(json.dumps(entry, indent=2, sort_keys=True) + "\n")
    else:
        sys.stdout.write(
            f"chassis exemptions add: created {entry['id']} (expires {entry['expires_at']}, "
            f"owner={entry['owner']})\n"
        )
    return 0


# ---------------------------------------------------------------------------
# validate
# ---------------------------------------------------------------------------


def cmd_validate(args: argparse.Namespace) -> int:
    repo = _resolve_repo(args.repo_root)
    assets = _resolve_assets(args.assets_root)
    report = ex.load_registry(repo, assets_root=assets)
    invalid_count = len(report.invalid) + len(report.schema_errors)
    expired_count = len(report.expired())

    if args.format == "json":
        body = report.to_json()
        body["valid"] = invalid_count == 0
        body["expired_count"] = expired_count
        sys.stdout.write(json.dumps(body, indent=2, sort_keys=True) + "\n")
    else:
        sys.stdout.write(
            f"chassis exemptions validate @ {repo}\n"
            f"  active={len(report.active())}  expired={expired_count}  "
            f"invalid={len(report.invalid)}  schema_errors={len(report.schema_errors)}\n"
        )
        for inv in report.invalid:
            sys.stdout.write(f"  invalid: {inv.id or '(no id)'}: {inv.reason}\n")
        for err in report.schema_errors:
            sys.stdout.write(f"  schema:  {err}\n")
        for exp in report.expired():
            sys.stdout.write(
                f"  expired: {exp.id} (expired {exp.expires_at.isoformat()}, owner={exp.owner})\n"
            )
        if invalid_count == 0 and expired_count == 0:
            sys.stdout.write("  OK\n")

    if invalid_count > 0:
        return 1
    if expired_count > 0 and not args.allow_expired:
        return 1
    return 0


# ---------------------------------------------------------------------------
# prune-expired
# ---------------------------------------------------------------------------


def cmd_prune_expired(args: argparse.Namespace) -> int:
    repo = _resolve_repo(args.repo_root)
    if args.dry_run:
        report = ex.load_registry(repo, assets_root=_resolve_assets(args.assets_root))
        affected = [e.id for e in report.expired()]
        if args.format == "json":
            sys.stdout.write(
                json.dumps(
                    {
                        "mode": args.mode,
                        "dry_run": True,
                        "affected_ids": affected,
                        "registry_path": str(repo / ex.REGISTRY_REL),
                    },
                    indent=2,
                )
                + "\n"
            )
        else:
            sys.stdout.write(
                f"chassis exemptions prune-expired (dry-run, mode={args.mode}): "
                f"{len(affected)} expired entry(s) would be affected\n"
            )
            for eid in affected:
                sys.stdout.write(f"  - {eid}\n")
        return 0

    result = ex.prune_expired(repo, mode=args.mode)
    if args.format == "json":
        sys.stdout.write(json.dumps(result, indent=2) + "\n")
    else:
        verb = "deleted" if args.mode == "delete" else "revoked"
        sys.stdout.write(
            f"chassis exemptions prune-expired: {verb} {len(result['affected_ids'])} entry(s); "
            f"{result['remaining']} entry(s) remain. (registry only; no source files modified)\n"
        )
        for eid in result["affected_ids"]:
            sys.stdout.write(f"  - {eid}\n")
    return 0


# ---------------------------------------------------------------------------
# explain
# ---------------------------------------------------------------------------


def cmd_explain(args: argparse.Namespace) -> int:
    repo = _resolve_repo(args.repo_root)
    assets = _resolve_assets(args.assets_root)
    detail = ex.explain(repo, args.exemption_id, assets_root=assets)
    if args.format == "json":
        sys.stdout.write(json.dumps(detail, indent=2, sort_keys=True) + "\n")
        return 0 if detail.get("found") else 1

    if not detail.get("found"):
        sys.stderr.write(f"chassis exemptions explain: id {args.exemption_id} not found\n")
        if detail.get("invalid"):
            sys.stderr.write(
                f"  but is recorded as invalid: {detail['invalid'].get('reason')}\n"
            )
        return 1

    sys.stdout.write(f"Exemption {detail['id']}\n")
    sys.stdout.write(f"  state            {detail['state']}\n")
    sys.stdout.write(f"  rule_id          {detail.get('rule_id') or '-'}\n")
    if detail.get("finding_id"):
        sys.stdout.write(f"  finding_id       {detail['finding_id']}\n")
    sys.stdout.write(f"  paths            {', '.join(detail['paths'])}\n")
    sys.stdout.write(f"  owner            {detail['owner']}\n")
    if detail.get("linked_issue"):
        sys.stdout.write(f"  linked_issue     {detail['linked_issue']}\n")
    if detail.get("adr"):
        sys.stdout.write(f"  adr              {detail['adr']}\n")
    sys.stdout.write(f"  created_at       {detail['created_at']}\n")
    sys.stdout.write(
        f"  expires_at       {detail['expires_at']} "
        f"(lifetime {detail['lifetime_days']}d, {detail['days_remaining']:+d}d remaining)\n"
    )
    if detail.get("severity_override"):
        sys.stdout.write(f"  severity_override {detail['severity_override']}\n")
    if detail.get("allow_global"):
        sys.stdout.write("  allow_global      true\n")
    sys.stdout.write(f"  reason           {detail['reason']}\n")
    return 0


# ---------------------------------------------------------------------------
# Argument parser
# ---------------------------------------------------------------------------


def _add_common(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--repo-root", default=None, help="Target repo (default: discovered)")
    parser.add_argument("--assets-root", default=None, help="Chassis assets root (default: discovered)")
    parser.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
        help="Output format (default: text).",
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="chassis-exemptions", description=__doc__)
    sub = parser.add_subparsers(dest="subcmd", required=True)

    p_list = sub.add_parser("list", help="List exemptions")
    _add_common(p_list)
    p_list.add_argument("--strict", action="store_true", help="Exit 1 when invalid/expired entries are present.")
    p_list.set_defaults(func=cmd_list)

    p_add = sub.add_parser("add", help="Add a new exemption (active, scoped, expiring).")
    _add_common(p_add)
    p_add.add_argument("--rule-id", help="Rule id being exempted (e.g. PANIC-BUDGET-HARD-ZERO).")
    p_add.add_argument("--finding-id", help="Specific finding id (alternative to --rule-id).")
    p_add.add_argument("--path", action="append", help="Repo-relative path/glob (repeatable).")
    p_add.add_argument("--scope", action="append", help="Alias for --path.")
    p_add.add_argument("--reason", required=True, help="Why the waiver (>=40 chars).")
    p_add.add_argument("--owner", required=True, help="Email/handle accountable for resolving.")
    p_add.add_argument("--linked-issue", help="Issue tracker reference (recommended).")
    p_add.add_argument("--adr", help="ADR id that defines the rule (e.g. ADR-0009).")
    p_add.add_argument(
        "--expires-at",
        help=f"ISO date; default created+{ex.MAX_LIFETIME_DAYS}d (cap enforced).",
    )
    p_add.add_argument("--severity-override", choices=("info", "warning", "error"))
    p_add.add_argument("--id", help="Explicit EX-YYYY-NNNN id (default: auto).")
    p_add.add_argument(
        "--allow-global",
        action="store_true",
        help="Per-entry opt-in for wildcard / global scopes. Registry must also opt in.",
    )
    p_add.set_defaults(func=cmd_add)

    p_validate = sub.add_parser("validate", help="Validate the registry (schema + lifecycle + quotas).")
    _add_common(p_validate)
    p_validate.add_argument(
        "--allow-expired",
        action="store_true",
        help="Do not fail when expired entries are present (advisory mode).",
    )
    p_validate.set_defaults(func=cmd_validate)

    p_prune = sub.add_parser("prune-expired", help="Remove or revoke expired entries (registry only).")
    _add_common(p_prune)
    p_prune.add_argument(
        "--mode",
        choices=("revoke", "delete"),
        default="revoke",
        help="`revoke` keeps entries with status=revoked (default). `delete` removes them.",
    )
    p_prune.add_argument("--dry-run", action="store_true", help="Print affected ids without modifying the registry.")
    p_prune.set_defaults(func=cmd_prune_expired)

    p_explain = sub.add_parser("explain", help="Explain a single exemption.")
    _add_common(p_explain)
    p_explain.add_argument("exemption_id", help="EX-YYYY-NNNN id to inspect.")
    p_explain.set_defaults(func=cmd_explain)

    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(list(argv) if argv is not None else None)
    return args.func(args)


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
