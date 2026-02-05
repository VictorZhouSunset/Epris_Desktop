## Summary
- New projects auto-link dependencies without showing FirstRunWizard (when toolchain/skills are already ready).
- “Baseline packages” are no longer installed silently during auto-link; instead the app prompts once per app update when baseline packages are missing, and can install them user-approved.
- Baseline packages can be installed into both the current workspace and the app-local workspace template (so future projects inherit them) when user approves.

## Files Changed
- `apps/epris-tauri/src-tauri/src/projects.rs`
- `apps/epris-tauri/src-tauri/src/environment.rs`
- `apps/epris-tauri/src-tauri/src/maintenance.rs`
- `apps/epris-tauri/src-tauri/src/lib.rs`
- `apps/epris-tauri/src/App.tsx`
- `apps/epris-tauri/src/lib/ipc.ts`
- `apps/epris-tauri/src-tauri/src/state_manager.rs`
- `task_plan.md`
- `progress.md`
- `findings.md`

## Verification
- WSL sandbox: `cargo check` couldn’t run due to rustup toolchain not configured in this environment.
- Recommended: validate by building on Windows / GitHub Actions as usual.
