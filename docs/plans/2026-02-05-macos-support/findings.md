## Summary (current state)

Epris is a Tauri v2 app (`apps/epris-tauri`), so macOS support is realistic. However, several critical runtime paths are currently Windows-only, and the release pipeline only produces a Windows installer.

## Key blockers found

- Toolchain installation is Windows-only:
  - `apps/epris-tauri/src-tauri/src/toolchain.rs` returns an error on non-Windows.
- Folder picker is Windows-only:
  - `apps/epris-tauri/src-tauri/src/projects.rs` returns “Folder picker not implemented for this OS yet” on non-Windows.
- Provider PATH handling is currently Windows-biased:
  - `apps/epris-tauri/src-tauri/src/provider.rs` prepends to `PATH` using `;` even on non-Windows.
- STT is Windows-only:
  - `apps/epris-tauri/src-tauri/src/stt.rs` explicitly rejects non-Windows.
- Bundling + CI are Windows-only:
  - `apps/epris-tauri/src-tauri/tauri.conf.json` bundles only `nsis`.
  - `.github/workflows/release.yml` has a single `build-windows` job.

## Recommendation

Ship a “mac preview” first (DoD A) to validate real Mac behavior, then decide if we invest in full parity (DoD B: self-contained toolchain + signing/notarization + updater drill).

