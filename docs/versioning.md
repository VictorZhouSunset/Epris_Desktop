# Versioning & Reproducibility

This repo targets a reproducible build environment so that renders and app behavior don’t drift across machines.

## Toolchain

- Node: `24.13.0` (pinned in `.nvmrc`)

## App versions

- `apps/epris-tauri`: `0.5.0` (see `apps/epris-tauri/package.json` and `apps/epris-tauri/src-tauri/tauri.conf.json`)

## Render/tooling versions

- Remotion (workspace template): `4.0.409` (see `workspace-template/package.json`)
- Remotion (app shell deps): `4.0.409` (see `apps/epris-tauri/package.json`)

