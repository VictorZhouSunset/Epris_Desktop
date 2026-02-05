# macOS Support (Epris v0.5.x) — task_plan

## Goal

Ship a macOS build of Epris (Tauri) that is usable for V0.5 feature set, with a clear path to “production-quality” distribution (signed/notarized + updater).

## Why this is feasible (quick assessment)

The app is already a Tauri v2 app (`apps/epris-tauri`), so *building* for macOS is supported by the framework. The main work is (a) removing Windows-only assumptions in backend commands/tooling and (b) deciding how “self-contained” the mac build should be (toolchain bundling, signing/notarization, updater).

## Definition of Done (choose a target)

### DoD A — “Preview build runs on mac” (fastest)
- App builds on a Mac (Apple Silicon at minimum) and launches.
- Core flows work on mac:
  - Create/open project.
  - Run provider flow (OpenCode server or Gemini CLI) and see results in UI.
  - Preview server works.
- Known Windows-only features are either:
  - implemented cross-platform, or
  - explicitly disabled on mac with a clear UI message + workaround.

### DoD B — “Production mac release”
Everything in DoD A, plus:
- Self-contained toolchain install works on mac via **in-app download** (Option 3) (no separate Node/pnpm install needed by user).
- GitHub Releases contains mac artifacts (DMG/APP) and updater JSON supports mac updates.
- Signing/notarization is **deferred** (no Apple Developer Program account right now). Friend testing uses one-time Gatekeeper override.

## Inputs needed (from you)
- [ ] Target CPUs:
  - Apple Silicon only (arm64), or
  - Universal (arm64 + x64), or
  - two separate builds.
- [ ] Distribution target:
  - local/dev only, or
  - friend-test distribution, or
  - public releases.
- [ ] Signing/notarization:
  - **skip initially (decision made)** — no Apple Developer Program account for now.
- [ ] Toolchain policy on mac:
  - **Option 3 (decision made):** match Windows UX via in-app download/install of Node/pnpm/provider CLIs.
  - Include “insurance”: best-effort clear `com.apple.quarantine` on downloaded/extracted toolchain binaries (and provide a single manual fallback command if needed).
- [ ] Minimum macOS version to support (e.g. 12/13/14).

## Decisions locked (as of 2026-02-05)
- Target: **DoD B**, but **without signing/notarization initially**.
- Toolchain on mac: **Option 3 (in-app download)** + **quarantine-clearing insurance step**.

## Native Modules / “No Xcode” Policy (important)

### Why this matters

The biggest macOS adoption risk is **native Node modules** falling back to “build from source”, which would require your friend to install Xcode Command Line Tools (and sometimes additional build deps). This can happen when:
- a dependency ships prebuilt binaries only for certain Node versions / CPUs, or
- an npm package runs `node-gyp` during install because no matching prebuild is available.

### What we can (and cannot) guarantee

- We **cannot guarantee** “never needs Xcode” as long as we rely on `npm install` for third-party CLIs and their transitive dependencies.
- We **can design** for a “no Xcode” friend-test experience by constraining versions and adding a fallback path.

### Policy for mac toolchain (Option 3)

1) **Prefer Node LTS on mac toolchain**
   - Use a Node LTS version that has the broadest ecosystem prebuild coverage (typically reduces `node-gyp` fallbacks).
   - If we keep Node `24.13.0`, expect higher risk that some dependencies lack prebuilt binaries.

2) **Fail fast with actionable error**
   - During toolchain install, detect “building from source / node-gyp” patterns in logs.
   - If detected, stop and present a simple message: “This build requires Xcode CLT; we’re switching to prebuilt provider binaries (recommended)” (see fallback below).

3) **Fallback: use prebuilt provider binaries instead of npm**
   - If `npm -g install opencode-ai` or `@google/gemini-cli` triggers native builds on mac, switch to:
     - downloading a provider’s platform binary from GitHub Releases (true prebuilt), then
     - placing it in the toolchain `bin` directory.
   - This avoids `node-gyp` entirely for provider installation.

**Decision needed:** pick the mac toolchain Node version (recommend **LTS**) and whether we allow per-OS Node pinning (Windows can keep current pin short-term if needed).

## Plan (phased, with clear checkpoints)

### Phase 0 — Audit & scope lock (0.5 day)
- [ ] List V0.5 features that must work on mac (project management, provider run, preview/render, updater, etc.).
- [ ] Decide DoD target (A vs B) and the 5 “must-have” acceptance tests on a real Mac.
- [ ] Inventory Windows-only code paths and decide per-item: port vs disable.
  - Known items to address:
    - `apps/epris-tauri/src-tauri/src/toolchain.rs`: local toolchain install is Windows-only.
    - `apps/epris-tauri/src-tauri/src/projects.rs`: folder picker is Windows-only.
    - `apps/epris-tauri/src-tauri/src/provider.rs`: PATH separator currently hardcoded as `;`.
    - `apps/epris-tauri/src-tauri/src/stt.rs`: explicitly Windows-only.
    - `.github/workflows/release.yml`: Windows-only build job.
    - `apps/epris-tauri/src-tauri/tauri.conf.json`: `bundle.targets` currently only `nsis`.

**Checkpoint:** confirm DoD + “must-have” list before coding.

### Phase 1 — “Preview build runs on mac” (1–2 days)
- [ ] Enable mac bundling targets in `apps/epris-tauri/src-tauri/tauri.conf.json` (add macOS targets alongside `nsis`).
- [ ] Add a macOS GitHub Actions job to `.github/workflows/release.yml` (runs-on `macos-latest`) to produce mac artifacts.
- [ ] Fix cross-platform runtime blockers in backend:
  - [ ] Provider start: use `:` on non-Windows when prepending toolchain `bin` to `PATH`.
  - [ ] Folder picker: implement cross-platform folder selection (prefer a Tauri dialog-based approach).
  - [ ] Toolchain: ensure mac path uses the same “download/install into appLocalDataDir” flow as Windows (even if feature-incomplete at first).
- [ ] Manual acceptance test on a Mac:
  - [ ] create/open project
  - [ ] run provider and see output
  - [ ] preview server start/stop

**Checkpoint:** you validate the `.dmg`/`.app` launches on your Mac and the acceptance tests pass.

### Phase 2 — Toolchain install on mac (Option 3) (2–5 days)
- [ ] Implement mac support in `apps/epris-tauri/src-tauri/src/toolchain.rs` (download-after-install):
  - [ ] Download pinned Node.js for `darwin-arm64` and/or `darwin-x64` (and verify SHA256).
  - [ ] Install into app-local toolchain dir (mirrors Windows layout as much as possible).
  - [ ] Provide mac shims for `npm`, `npx`, `pnpm` (shell scripts, not `.cmd`).
  - [ ] Install pinned pnpm version into toolchain-global prefix.
  - [ ] Install provider CLIs (OpenCode / Gemini) into toolchain-global prefix **only if** it does not trigger native builds (no Xcode).
  - [ ] If native builds are detected (node-gyp), switch to “prebuilt provider binary” install path (download platform binaries instead of `npm -g`).
  - [ ] Ensure the app uses local toolchain first (PATH + “where”/“which” detection).
- [ ] Add quarantine “insurance” on mac for toolchain binaries:
  - [ ] Best-effort clear `com.apple.quarantine` on downloaded/extracted toolchain dirs (no user interaction).
  - [ ] If clearing fails, show a single actionable fallback command in UI/logs (friend can run once).
- [ ] Update environment checks in `apps/epris-tauri/src-tauri/src/environment.rs` to correctly classify “local” tools on mac (avoid Windows-only `node.exe` assumptions).

**Checkpoint:** a fresh mac machine (no Node installed) can install toolchain from inside Epris and run provider flows.

### Phase 3 — Updater + release quality (1–3 days, if distributing)
- [ ] Ensure mac artifacts are included in GitHub Releases and referenced by `latest.json`.
- [ ] Extend `docs/update-drill.md` with a macOS drill.
- [ ] Document friend-test Gatekeeper steps (no dev account):
  - [ ] One-time `Epris.app` override steps (right-click open / Privacy & Security “Open Anyway”).
  - [ ] Toolchain fallback command only if needed (should be rare with insurance).

**Checkpoint:** install older version → updater installs newer version successfully on mac.

### Phase 4 — Optional: STT on mac (time-boxed)
- [ ] Decide whether STT must be supported on mac for V0.5.
- [ ] If yes, implement mac asset selection + install in `apps/epris-tauri/src-tauri/src/stt.rs` (similar to the Windows path, but picking darwin artifacts).

## Risks / unknowns (to resolve early)
- Apple signing/notarization requires Apple Developer Program access and secrets handling in CI.
- “Universal” builds may increase CI time and complexity; separate arch builds may be simpler.
- Provider CLIs and Remotion compositor dependencies need a real Mac smoke-test to confirm no hidden native dependency issues.

## Suggested sequencing (recommendation)

1) DoD A first (fast mac preview), then decide whether to invest in DoD B.
2) If distributing to friends: prioritize signing + notarization early to reduce support load.
