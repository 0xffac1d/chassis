#!/usr/bin/env python3
"""``chassis doctor`` — verify the local Chassis distribution tree and Python deps."""
from __future__ import annotations

import argparse
import json
import os
import shutil
import stat
import sys
from importlib.util import find_spec
from pathlib import Path

_SCRIPTS_CHASSIS = Path(__file__).resolve().parent
if str(_SCRIPTS_CHASSIS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_CHASSIS))


def _assets_root() -> Path:
    from repo_layout import chassis_assets_root

    return chassis_assets_root()


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="chassis doctor")
    ap.add_argument("--format", choices=("text", "json"), default="text")
    args = ap.parse_args(argv if argv is not None else sys.argv[1:])

    issues: list[str] = []
    notes: list[str] = []

    try:
        root = _assets_root()
    except Exception as exc:
        if args.format == "json":
            sys.stdout.write(
                json.dumps(
                    {
                        "ok": False,
                        "issues": [f"chassis assets root: {exc}"],
                        "notes": [],
                        "dispatcher": None,
                        "chassis_root": None,
                        "python_executable": sys.executable,
                        "imports": {},
                    },
                    indent=2,
                )
                + "\n"
            )
            return 1
        print(f"chassis doctor: {exc}", file=sys.stderr)
        return 1

    dispatcher = (_SCRIPTS_CHASSIS / "chassis").resolve()
    exe_ok = dispatcher.is_file() and os.access(dispatcher, os.X_OK)
    if not dispatcher.is_file():
        issues.append(f"missing dispatcher script: {dispatcher}")
    elif not exe_ok:
        issues.append(f"dispatcher not executable: {dispatcher}")
        try:
            st = dispatcher.stat()
            dispatcher.chmod(st.st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
            if os.access(dispatcher, os.X_OK):
                notes.append(f"chmod +x restored on {dispatcher}")
                exe_ok = True
                issues.pop()
        except OSError:
            pass

    marker = root / "schemas" / "metadata" / "contract.schema.json"
    if not marker.is_file():
        issues.append(f"missing schema marker: {marker}")
    tmpl = root / "templates"
    if not tmpl.is_dir():
        issues.append(f"missing templates directory: {tmpl}")

    bash = shutil.which("bash")
    if bash:
        notes.append(f"bash: {bash}")
    else:
        issues.append("bash not found on PATH (needed for dispatcher subcommands)")

    imports_ok: dict[str, bool] = {}
    for pkg in ("jsonschema", "yaml", "referencing"):
        ok = find_spec(pkg) is not None
        imports_ok[pkg] = ok
        if not ok:
            issues.append(f"missing Python import for {pkg!r}")

    ok = len(issues) == 0
    dispatcher_entry = {"path": str(dispatcher), "executable": exe_ok}

    if args.format == "json":
        sys.stdout.write(
            json.dumps(
                {
                    "ok": ok,
                    "chassis_root": str(root),
                    "dispatcher": dispatcher_entry,
                    "python_executable": sys.executable,
                    "imports": imports_ok,
                    "issues": issues,
                    "notes": notes,
                },
                indent=2,
            )
            + "\n"
        )
        return 0 if ok else 1

    print(f"chassis doctor — distribution root: {root}")
    print(f"  dispatcher: {dispatcher} ({'ok' if exe_ok else 'not executable'})")
    print(f"  python: {sys.executable}")
    for k, v in imports_ok.items():
        print(f"  import {k}: {'ok' if v else 'MISSING'}")
    if notes:
        print("Notes:")
        for n in notes:
            print(f"  - {n}")
    if issues:
        print("Problems:", file=sys.stderr)
        for i in issues:
            print(f"  - {i}", file=sys.stderr)
        return 1
    print("Doctor: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
