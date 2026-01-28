#!/usr/bin/env bash
set -euo pipefail

echo "[bootstrap-wsl] Starting (Ubuntu 24.04 / WSL assumed)..."

# Ensure we are in repo root (best effort)
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

echo "[bootstrap-wsl] Repo root: $REPO_ROOT"

# 1) APT packages
echo "[bootstrap-wsl] Installing system dependencies via apt..."
sudo apt update

sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  clang \
  cmake \
  git \
  curl \
  ca-certificates \
  patchelf \
  librsvg2-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev

# 2) Rust toolchain via rustup
if ! command -v rustup >/dev/null 2>&1; then
  echo "[bootstrap-wsl] Installing rustup..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y
fi

# Load cargo env for current shell
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

echo "[bootstrap-wsl] Updating Rust toolchain..."
rustup toolchain install stable >/dev/null 2>&1 || true
rustup default stable >/dev/null 2>&1 || true
rustup update stable

echo "[bootstrap-wsl] rustc: $(rustc -V)"
echo "[bootstrap-wsl] cargo: $(cargo -V)"

# 3) Prefetch crates for the Tauri backend (helps if later runs have restricted network)
TAURI_MANIFEST="apps/epris-tauri/src-tauri/Cargo.toml"
if [ -f "$TAURI_MANIFEST" ]; then
  echo "[bootstrap-wsl] Prefetching crates for: $TAURI_MANIFEST"
  export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
  cargo fetch --manifest-path "$TAURI_MANIFEST"
else
  echo "[bootstrap-wsl] Skip cargo fetch: not found $TAURI_MANIFEST"
fi

echo "[bootstrap-wsl] Done."
echo "[bootstrap-wsl] Next checks:"
echo "  cargo check --manifest-path apps/epris-tauri/src-tauri/Cargo.toml"
echo "  cargo test  --manifest-path apps/epris-tauri/src-tauri/Cargo.toml"
