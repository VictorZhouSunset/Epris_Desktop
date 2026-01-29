# Update Drill (0.5.0 → 0.5.1)

Goal: verify that a user installed on **v0.5.0** can upgrade to **v0.5.1** via the in-app updater.

## Preconditions

- `apps/epris-tauri/src-tauri/tauri.conf.json` has:
  - `plugins.updater.endpoints` pointing at `.../releases/latest/download/latest.json`
  - a valid `plugins.updater.pubkey`
  - `bundle.createUpdaterArtifacts = true`
- GitHub Actions release workflow is present: `.github/workflows/release.yml`
- GitHub Secrets are configured: `docs/release-secrets.md`

## Drill steps

1) Tag and push `v0.5.0`
   - Ensure app version is `0.5.0` in:
     - `apps/epris-tauri/package.json`
     - `apps/epris-tauri/src-tauri/tauri.conf.json`
2) Wait for CI to create a **draft** GitHub Release.
3) Publish the `v0.5.0` release.
4) Install `v0.5.0` on a test machine.
5) Bump app version to `0.5.1`, commit, tag and push `v0.5.1`.
6) Wait for CI to create the `v0.5.1` draft release; publish it.
7) On the `v0.5.0` install, open Settings → Updates:
   - Click **Check**
   - Confirm it detects `v0.5.1`
   - Click **Install**
8) Confirm post-update:
   - The app starts successfully
   - Existing projects still appear and open
   - Logs show updater progress (see app data dir `logs/updater.log`)

