# Data Layout (V0.5)

Epris stores user data under the OS application data directory (Tauri `appLocalDataDir`) so it survives app updates.

## App data root

`<appLocalDataDir>/`

- `state.json` (project registry + settings)
- `projects/` (default projectsRoot)
  - `<project-id-or-slug>/` (projectRoot)
    - Remotion workspace (includes `src/`, `public/`, `node_modules/`, etc.)
    - `logs/` (per-project logs)
    - `.epris/` (snapshots/backups/temp)
- `logs/`
  - `updater.log` (app-level updater log)

## Notes

- The installer/update process must not rely on files in the install directory for user data.
- Projects can be moved by changing “Projects Folder” in Settings (the app will move directories on disk).

