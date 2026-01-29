#!/usr/bin/env python3
"""
Repro: New projects can show "127.0.0.1 refused to connect" if the preview server
is started before workspace dependencies are installed (node_modules missing),
or if the UI loads the iframe before the server is actually ready.

This script emulates the "new workspace" state by copying `workspace-template`
into a temp folder (excluding node_modules), then attempts to run the same
preview command used by the app:

  pnpm run dev --port <PORT>

Usage:
  python3 scripts/repro_preview_server.py
  python3 scripts/repro_preview_server.py --port 3030 --keep
"""

from __future__ import annotations

import argparse
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
TEMPLATE_DIR = REPO_ROOT / "workspace-template"


def _copytree_excluding_node_modules(src: Path, dst: Path) -> None:
    def ignore(dirpath: str, names: list[str]) -> set[str]:
        if Path(dirpath).name == "node_modules":
            return set(names)
        return {"node_modules"} if "node_modules" in names else set()

    shutil.copytree(src, dst, ignore=ignore, dirs_exist_ok=False)


def _port_open(host: str, port: int, timeout: float = 0.2) -> bool:
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(timeout)
            try:
                s.connect((host, port))
                return True
            except OSError:
                return False
    except PermissionError:
        # Some sandboxed environments disallow creating sockets; treat as "not reachable".
        return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=3030)
    parser.add_argument("--keep", action="store_true", help="Do not delete temp workspace")
    args = parser.parse_args()

    if not TEMPLATE_DIR.exists():
        print(f"ERROR: missing template dir: {TEMPLATE_DIR}", file=sys.stderr)
        return 2

    pnpm = shutil.which("pnpm")
    if pnpm is None:
        print("ERROR: pnpm not found on PATH.", file=sys.stderr)
        print("Run this on the same machine where Epris runs (Windows).", file=sys.stderr)
        return 2

    tmp_root = Path(tempfile.mkdtemp(prefix="epris-repro-preview-"))
    workspace = tmp_root / "workspace"
    _copytree_excluding_node_modules(TEMPLATE_DIR, workspace)

    print(f"Workspace: {workspace}")
    print(f"node_modules exists: {(workspace / 'node_modules').exists()}")
    print("Starting preview dev server...")

    cmd = [pnpm, "run", "dev", "--port", str(args.port)]
    proc = subprocess.Popen(
        cmd,
        cwd=str(workspace),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={**os.environ},
    )

    try:
        start = time.time()
        while time.time() - start < 10:
            if proc.poll() is not None:
                print("Process exited early. Output:\n")
                out = (proc.stdout.read() if proc.stdout else "")[:4000]
                print(out)
                break
            if _port_open("127.0.0.1", args.port):
                print(f"Server is reachable on http://127.0.0.1:{args.port}")
                break
            time.sleep(0.25)
        else:
            print("Timed out waiting for the server to become reachable.")
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
        if not args.keep:
            shutil.rmtree(tmp_root, ignore_errors=True)
        else:
            print(f"Kept temp workspace: {tmp_root}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
