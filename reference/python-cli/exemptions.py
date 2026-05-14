#!/usr/bin/env python3
"""``chassis exemptions …`` — thin alias over :mod:`exempt` (registry gate)."""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

_SCRIPTS_CHASSIS = Path(__file__).resolve().parent
if str(_SCRIPTS_CHASSIS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_CHASSIS))

from exempt import cmd_check  # noqa: E402
from repo_layout import repo_root  # noqa: E402


def main(argv: list[str] | None = None) -> int:
    argv = argv if argv is not None else sys.argv[1:]
    p = argparse.ArgumentParser(prog="chassis exemptions")
    p.add_argument("--repo-root", default=None, help="Repository root (default: auto-detect)")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("validate", help="Schema + quota + no-expired (CI entrypoint)")
    args = p.parse_args(argv)
    if args.cmd != "validate":
        return 2
    root = Path(args.repo_root).resolve() if args.repo_root else repo_root()
    ns = argparse.Namespace(repo_root=str(root))
    return cmd_check(ns)


if __name__ == "__main__":
    raise SystemExit(main())
