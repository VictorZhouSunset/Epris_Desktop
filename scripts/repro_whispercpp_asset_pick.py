#!/usr/bin/env python3
"""
Repro/debug helper for the STT installer.

It fetches the latest whisper.cpp GitHub release metadata and prints:
- available asset names
- which asset would be selected for Windows x64

Usage:
  python3 scripts/repro_whispercpp_asset_pick.py
  python3 scripts/repro_whispercpp_asset_pick.py --json path/to/release.json
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.request


def fetch_latest_release() -> dict:
    url = "https://api.github.com/repos/ggerganov/whisper.cpp/releases/latest"
    req = urllib.request.Request(url, headers={"User-Agent": "epris-desktop"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def pick_windows_zip_asset(assets: list[dict]) -> str | None:
    names = [a.get("name", "") for a in assets]
    has_win32 = any("win32" in n.lower() for n in names)

    best: tuple[int, int, str] | None = None  # (score, name_len, name)
    for name in names:
        n = name.lower()
        if not n.endswith(".zip"):
            continue
        if "xcframework" in n or n.endswith(".jar.zip") or "android" in n:
            continue
        if "win32" in n:
            continue
        is_x64 = any(x in n for x in ("x64", "win64", "amd64", "x86_64"))
        if not is_x64:
            continue

        score = 0
        if "win" in n or "windows" in n:
            score += 50
        if has_win32 and not ("win" in n or "windows" in n):
            score += 40
        if "whisper" in n:
            score += 10
        if "bin" in n:
            score += 20
        if "whisper-bin-x64" in n:
            score += 30
        if "blas" in n:
            score -= 5
        if "cublas" in n or "cuda" in n:
            score -= 10

        candidate = (score, len(name), name)
        if best is None or candidate > best:
            best = candidate

    return best[2] if best else None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", help="Path to a saved GitHub release JSON")
    args = parser.parse_args()

    if args.json:
        data = json.loads(open(args.json, "r", encoding="utf-8").read())
    else:
        data = fetch_latest_release()

    assets = data.get("assets", [])
    names = sorted(a.get("name", "") for a in assets)
    print(f"assets: {len(names)}")
    for n in names:
        print(f"- {n}")

    picked = pick_windows_zip_asset(assets)
    print("\nselected:", picked or "(none)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

