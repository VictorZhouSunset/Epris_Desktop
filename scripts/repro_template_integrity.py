#!/usr/bin/env python3
"""
Diagnose missing workspace-template files in an installed Epris app.

Usage:
  python scripts/repro_template_integrity.py --template "C:\\Path\\To\\AppData\\...\\workspace-template"
  python scripts/repro_template_integrity.py --project "D:\\Epris\\projects\\my-video-xxxx"
"""

from __future__ import annotations

import argparse
from pathlib import Path


REQUIRED_SRC_FILES = [
    Path("src/Composition.tsx"),
    Path("src/Preview.tsx"),
    Path("src/Root.tsx"),
    Path("src/VideoConfig.ts"),
    Path("src/index.tsx"),
]


def check_root(label: str, root: Path) -> int:
    print(f"== {label} ==")
    print(f"root: {root}")
    if not root.exists():
        print("ERROR: path does not exist")
        return 2

    missing = []
    for rel in REQUIRED_SRC_FILES:
        p = root / rel
        if not p.exists():
            missing.append(rel.as_posix())

    if missing:
        print("MISSING:")
        for m in missing:
            print(f"  - {m}")
        print("")
        print("Top-level listing:")
        for child in sorted(root.iterdir(), key=lambda p: (p.is_file(), p.name.lower())):
            suffix = "/" if child.is_dir() else ""
            print(f"  - {child.name}{suffix}")
        return 1

    print("OK: required src files are present")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--template", type=Path, help="Path to <appLocalDataDir>\\workspace-template")
    ap.add_argument("--project", type=Path, help="Path to a projectRoot")
    args = ap.parse_args()

    rc = 0
    if args.template is not None:
        rc = max(rc, check_root("workspace-template", args.template))
    if args.project is not None:
        rc = max(rc, check_root("projectRoot", args.project))
    if args.template is None and args.project is None:
        ap.error("provide --template and/or --project")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())

