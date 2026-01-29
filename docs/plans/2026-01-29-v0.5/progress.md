# Epris V0.5 (Updater + Sandbox) — progress

## In Progress
- Task 5: sandbox tightening

## Completed
- Confirmed GitHub repo: `VictorZhouSunset/Epris_Desktop`
- Task 1: added `schema_version` to state + projects; pinned Remotion; bumped app version to `0.5.0`
- Task 2 (partial): added updater plugin deps + config scaffolding + capability
- Task 3 (partial): added Settings “Updates” UI + updater wrapper + app-local updater log
- Pinned Node version: `.nvmrc` (`24.13.0`)
- Task 4: added release workflow + docs (`.github/workflows/release.yml`, `docs/release-secrets.md`, `docs/update-drill.md`)

## Blocked
- Need `pnpm install` in `apps/epris-tauri` after adding `@tauri-apps/plugin-updater`.
