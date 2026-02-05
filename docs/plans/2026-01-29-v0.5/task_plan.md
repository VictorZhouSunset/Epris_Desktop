# Epris V0.5 (Updater + Sandbox) — task_plan

> Source spec: `docs/epris_v_0.5.md`

## Inputs needed (from human)
- [ ] Confirm GitHub repo for releases (format: `<owner>/<repo>`) for updater endpoint + CI.
- [ ] Decide target Node version (recommend LTS) for `.nvmrc` / docs.
- [ ] Generate Tauri updater signing keypair; keep private key out of repo; provide pubkey for config.

## Step 0 — Reproducible baseline
- [ ] Lock Remotion deps in `apps/epris-tauri/package.json` (remove `^`, align `remotion` + `@remotion/*` versions).
- [ ] Add Node version pin file (e.g. `.nvmrc`) + doc (`docs/versioning.md`).
- [ ] Add `schema_version` to project metadata in `apps/epris-tauri/src-tauri/src/state_manager.rs`.

## Step 1–2 — Tauri updater integration + signing
- [ ] Add Rust updater plugin (`tauri-plugin-updater`) and init it in `apps/epris-tauri/src-tauri/src/lib.rs`.
- [ ] Add updater config in `apps/epris-tauri/src-tauri/tauri.conf.json`:
  - endpoints: `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`
  - pubkey
  - `bundle.createUpdaterArtifacts = true`
- [ ] Add updater permissions in `apps/epris-tauri/src-tauri/capabilities/default.json` (`updater:default`).

## Step 3–4 — Capabilities tighten + update UI
- [ ] Review capabilities for “allow-all” (no `shell:allow-all`, no `process:allow-all`).
- [ ] Add update UI panel + state machine in `apps/epris-tauri/src/components/Settings.tsx` (check / available / downloading / installing / done / error).
- [ ] Add small updater wrapper `apps/epris-tauri/src/lib/updater.ts`.
- [ ] Persist/update logs to app data dir (new `updater.log` under appLocalDataDir).

## Step 5–6 — GitHub Actions release + drill
- [ ] Add `.github/workflows/release.yml` using `tauri-apps/tauri-action` for Windows build + `latest.json`.
- [ ] Add docs: `docs/release-secrets.md`, `docs/update-drill.md`.

## Step 7 — Data layout safety
- [ ] Confirm projectsRoot lives in `appLocalDataDir` by default (already implemented); add `docs/data-layout.md`.

## Step 8 — Sandbox tighten (projectRoot boundary)
- [ ] Add backend path guard module (e.g. `apps/epris-tauri/src-tauri/src/sandbox.rs`) with canonicalize + starts-with root.
- [ ] Enforce “no writes” for `node_modules/**` and `.git/**` in any backend file operations (and audit logs).
- [ ] Add post-run audit for provider runs: detect and revert if forbidden paths changed.

## Step 9 — Friend trial stability
- [ ] Ensure first-run wizard covers deps + demo project (already partially implemented); add `docs/first-run.md`.
- [ ] Add “Export diagnostics” button (collect logs + state into zip).

