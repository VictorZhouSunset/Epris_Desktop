# Epris V0.5 (Updater + Sandbox) — findings

## Initial assessment (2026-01-29)
- Spec paths in `docs/epris_v_0.5.md` reference `src-tauri/...`; current codebase uses `apps/epris-tauri/src-tauri/...`.
- `workspace-template/package.json` already pins `remotion` + `@remotion/*` to `4.0.409` (good), but `apps/epris-tauri/package.json` currently uses `^` ranges and mismatched Remotion versions (needs alignment for reproducibility).
- Tauri updater is not yet configured:
  - Missing `tauri-plugin-updater` in Rust.
  - Missing `plugins.updater` configuration in `apps/epris-tauri/src-tauri/tauri.conf.json`.
  - Missing updater capability permissions.
- Sandbox posture today:
  - Provider/OpenCode processes are started with `cwd = projectRoot` (good baseline).
  - Snapshot/backup system only tracks whitelisted dirs (`src`, `public`) + a small forbidden-file set; it does not currently guarantee “no writes to `node_modules`/`.git`”.

## Implemented so far
- Added `schema_version` fields to state + project metadata for future migrations.
- Pinned app-shell Remotion deps and bumped app version to `0.5.0`.
- Added scaffolding for Tauri updater (plugin deps, config, capability) + Settings UI section.
- Added GitHub Actions release pipeline (Windows) to build installer + updater artifacts and upload `latest.json`.
- Verified Rust backend builds under WSL via `cargo check` (using `CARGO_HOME=/tmp/cargo-home` and `CARGO_TARGET_DIR=/tmp/epris-tauri-target` to stay within sandbox writeable roots).

## Remaining blockers
- Need the JS deps installed (`pnpm install` in `apps/epris-tauri`) after adding `@tauri-apps/plugin-updater`.
