# Release Secrets (GitHub Actions)

This repo uses Tauri’s updater signing to publish updates to GitHub Releases.

## Required repository secrets

Create these in **GitHub → Settings → Secrets and variables → Actions**:

- `TAURI_SIGNING_PRIVATE_KEY`
  - Your updater signing private key **content** (do not use a local file path in CI).
  - If you generated a key file (example: `$HOME\\.tauri\\epris-updater.key`), copy its full contents (the text in the file) into this secret.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  - The password for the private key **if you set one**.
  - If your key is not password-protected, set this to an empty string.

The workflow also exports compatibility env vars (`TAURI_PRIVATE_KEY`, `TAURI_KEY_PASSWORD`) from these same secrets.

## Local builds

For local builds on Windows, setting:

- `TAURI_SIGNING_PRIVATE_KEY` to the key **path** (as you did), and
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if needed,

is fine — but CI runners do not have your local key file, so CI must use secret **contents**.
