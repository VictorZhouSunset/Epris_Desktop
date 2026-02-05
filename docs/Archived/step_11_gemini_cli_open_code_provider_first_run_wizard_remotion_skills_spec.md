# Step 11 — Provider Orchestration + First‑Run Wizard + Remotion Skills

## Goal
Enable the app to run **either OpenCode or Gemini CLI** inside a user-selected **workspace directory**, with a **first-run environment initializer** that installs required dependencies *locally* (no global pollution), and then **installs Remotion skills** for both providers.

## Scope
- UI: provider selection, environment initialization wizard, progress + logs, retry.
- Backend (Rust/Tauri): dependency detection, install orchestration, PATH isolation, launching provider process in workspace.
- Providers:
  - OpenCode (local CLI process)
  - Gemini CLI (local CLI process, Google login flow)
- Skills:
  - Add Remotion skills from `remotion-dev/remotion` (installed into each provider’s supported skills mechanism).

Non-goals (for Step 11):
- Building Remotion compositions or rendering pipeline beyond verifying preview works.
- Account management beyond “provider requires login”.

---

## Definitions
- **Workspace**: user project folder (must contain `package.json` and Remotion project structure).
- **App Data Dir**: per-user app directory used for local installs.
  - Windows: `%APPDATA%/<AppName>/` or `%LOCALAPPDATA%/<AppName>/`
  - macOS: `~/Library/Application Support/<AppName>/`
  - Linux: `~/.local/share/<AppName>/`
- **Toolchain Dir**: `<AppDataDir>/toolchain/` (local Node/pnpm/CLIs).
- **Bin Dir**: `<ToolchainDir>/bin/` (shim/executables to prepend to PATH).
- **Provider Process**: the spawned OpenCode/Gemini CLI process.

---

## Requirements
### R1 — Provider Selection
- UI must allow selecting provider: **OpenCode** or **Google (Gemini CLI)**.
- Selection must be persisted per-workspace.

### R2 — Launch Provider in Workspace
- When user clicks “Start”, the app must spawn the provider process with:
  - Working directory = workspace
  - Environment variables set for isolated toolchain:
    - `PATH` prepended with `<ToolchainDir>/bin` and any local Node dir
    - Optional provider-specific env vars
- Provider must be restartable without re-initializing if dependencies are present.

### R3 — First-Run Wizard
- On first open (or when missing deps), app must present an “Environment Initialization” screen.
- Wizard checks:
  - `node --version` (>= required)
  - `pnpm --version` (>= required)
  - provider CLI present (OpenCode/Gemini)
  - basic Remotion project sanity checks (if workspace provided):
    - `package.json` exists
    - `node_modules` present OR can be installed
- Wizard must:
  - install missing deps into Toolchain Dir
  - show a progress bar and live logs
  - allow retry

### R4 — No Global Pollution
- Do not require user to install Node/pnpm globally.
- All installs go into Toolchain Dir.
- Runtime PATH must be modified only for spawned processes.

### R5 — Remotion Skills Installation
- After toolchain ready, app must install Remotion skills from:
  - `https://github.com/remotion-dev/remotion`
- Skills must be installed/registered for:
  - OpenCode provider
  - Gemini CLI provider

---

## Strictness & Guardrails
### G1 — File Touch Policy
When using provider-based code modification flows for Remotion projects:
- Default rule: **Only modify `src/Composition.tsx`** unless user explicitly overrides.
- Do not create/remove compositions in `src/Root.tsx`.
- Keep TypeScript compilation clean.

### G2 — Stepwise Execution
Provider must follow a stable loop:
1) Read relevant files
2) Propose a plan
3) Apply minimal diff
4) Run `typecheck` and `remotion preview` (or render if asked)
5) Fix failures; repeat

### G3 — Deterministic Tooling
- Prefer `pnpm` scripts from `package.json` when available.
- If scripts missing, use conservative defaults (documented below).

---

## Implementation Plan

### 11.1 UI: Provider + Wizard
**Screens**
- Provider selection dropdown: `OpenCode | Google (Gemini CLI)`
- Workspace selector: choose folder
- Status panel:
  - Dependency status (Node/pnpm/provider CLI)
  - Skills status (installed / pending)
  - Logs viewer
- “Initialize Environment” button appears when missing deps.

**Events**
- Frontend listens for backend events:
  - `env_check_started`
  - `env_missing_deps` + list
  - `env_install_progress` (stage, percent)
  - `env_install_log` (line)
  - `env_ready`
  - `provider_started` / `provider_exited`


### 11.2 Rust/Tauri: Dependency Detection
**Check order**
1) Resolve `AppDataDir` and `ToolchainDir`.
2) Build an **effective PATH** for checks:
   - `<ToolchainDir>/bin` first
   - then system PATH
3) Execute:
   - `node --version`
   - `pnpm --version`
   - provider executable `opencode --version` or `gemini --version` (actual binary name may vary)

**Result model**
- `present: bool`
- `version: Option<String>`
- `source: System | Local`


### 11.3 Rust/Tauri: Local Installs
#### 11.3.0 Wizard Stages (Recommended)
Implement the First-Run Wizard as explicit, resumable stages:
1) Detect environment (system + local toolchain)
2) Install/repair toolchain (Node → pnpm → provider CLI)
3) Bootstrap workspace deps (`pnpm install`)
4) Install Remotion skills (cache → workspace projections)
5) Verify (typecheck + provider skill discovery)

Each stage must:
- Emit start/progress/end events
- Write stage outcome into `state.json`
- Be retryable independently

#### 11.3.1 Node.js (Windows)
- Preferred: `winget install OpenJS.NodeJS.LTS` only if user opts in.
- Default: download a portable Node distribution to Toolchain Dir.
- Create a wrapper/shim in `<ToolchainDir>/bin/node(.exe)` if needed.

#### 11.3.2 pnpm
- Use local Node to run:
  - `npm install -g pnpm` is not allowed globally.
  - Instead:
    - `npm install pnpm --prefix <ToolchainDir>`
    - Add `<ToolchainDir>/node_modules/.bin` into PATH
- Alternatively use `corepack` if available (still keep it local).

#### 11.3.3 Provider CLIs
- OpenCode:
  - Install locally via npm (or bundled) into Toolchain Dir.
  - Ensure executable is exposed in `<ToolchainDir>/bin`.
- Gemini CLI:
  - Install per its official method into Toolchain Dir if possible.
  - Login step: spawn interactive login; surface instructions in UI.

**Constraints**
- All commands executed with:
  - working dir = Toolchain Dir
  - env PATH = effective PATH
- Each install step must emit:
  - start/end events
  - logs


### 11.4 Workspace Dependency Bootstrap
If workspace has `package.json` but missing `node_modules`:
- Run:
  - `pnpm install` (in workspace)
- Then sanity:
  - `pnpm -s run typecheck` if script exists
  - `pnpm -s exec remotion --help` or `pnpm -s remotion --help` depending on setup


### 11.5 Skills: Remotion Repo

**Where it runs**
- Remotion skills installation is part of the **First-Run Wizard** (Stage 4).
- It may also be re-run later from settings: “Reinstall/Update Skills”.

**Trigger conditions**
- Skills not installed for the selected provider in the current workspace.
- The pinned commit in `state.json` differs from what is installed in workspace projections.
- User explicitly clicks “Update Skills”.

#### 11.5.1 Source
- Skills package to install (Remotion official):
  - `remotion-dev/skills`

#### 11.5.1.1 Fetch Strategy (No global dependencies)
Support two fetch methods to avoid requiring system `git`:

A) **Git-based** (preferred when available)
- If `git` is present, fetch exact commit into cache.
- Record resolved commit hash.

B) **Archive-based** (fallback)
- Download repo archive (zip/tarball) for the pinned ref.
- Extract into `<AppDataDir>/toolchain/remotion-skills-cache/<commit>/...`
- Record resolved commit hash.

#### 11.5.1.2 Pinning
- Always resolve to a commit hash and store it.
- Do not install from a floating ref (e.g., branch) without resolving + recording the commit.

#### 11.5.2 Install Targets

**Principle**: Skills are **workspace-visible** so the provider can discover them deterministically. Toolchain stores an immutable cache; workspace gets a provider-specific projection.

- **Toolchain cache (immutable, version-pinned)**
  - Store the resolved skills source at:
    - `<AppDataDir>/toolchain/remotion-skills-cache/<commit>/...`
  - Record `<commit>` in `state.json`.

- **Gemini CLI (workspace projection)**
  - Install/sync into:
    - `<Workspace>/.gemini/skills/<skill-name>/SKILL.md`

- **OpenCode (workspace projection)**
  - Install/sync into:
    - `<Workspace>/.opencode/skills/<skill-name>/SKILL.md`
  - Optional compatibility projection:
    - `<Workspace>/.claude/skills/<skill-name>/SKILL.md` (some ecosystems/tools expect this path)

**Sync strategy**
- Prefer **copy** for maximum portability.
- Allow **symlink** only if the platform and user permissions allow it, and if you can guarantee links won’t break when the cache is cleaned.

#### 11.5.3 Skill Set
Minimum required Remotion-oriented skills:
- `remotion-edit-composition`:
  - Only touch `src/Composition.tsx`
  - Must run typecheck + preview
  - Must provide a short diff summary
- `remotion-debug-build`:
  - Inspect build/runtime errors
  - Propose fixes, apply minimal changes

(If the Remotion repo provides skills directly, mirror them; otherwise, build wrappers that call into your stable workflow.)

#### 11.5.4 Wizard Integration: Stage Details
Stage 4 (“Install Remotion skills”) should execute:
1) Resolve desired pin (commit) according to app policy
2) Ensure cache present at `<AppDataDir>/toolchain/remotion-skills-cache/<commit>/...`
3) Sync projections into the current workspace:
   - `.gemini/skills/...`
   - `.opencode/skills/...` (and optional `.claude/skills/...`)
4) Verify discovery:
   - Gemini: skills list/reload capability
   - OpenCode: skills discovery check
5) Persist results to `state.json` (installed_at, commit, projections)

Stage 5 (“Verify”) must include at least:
- Workspace typecheck
- A minimal Remotion preview check (or provider-driven validation)


---

## File/Directory Layout
```
<AppDataDir>/
  toolchain/
    bin/
    node/
    pnpm/
    opencode/
    gemini/
    remotion-skills-cache/
      <commit>/
        (skills source snapshot)
  logs/
  state.json

<Workspace>/
  .gemini/
    skills/
      remotion-.../
  .opencode/
    skills/
      remotion-.../
  .claude/              (optional compatibility)
    skills/
      remotion-.../
  (project files)
```

---

## Commands (Defaults)
When scripts exist in workspace `package.json`, prefer them.

Fallback commands:
- Install deps: `pnpm install`
- Typecheck:
  - `pnpm -s run typecheck` (if present)
  - else `pnpm -s exec tsc -p tsconfig.json --noEmit`
- Remotion preview:
  - `pnpm -s exec remotion preview`
  - or `pnpm -s remotion preview` (depending on how remotion is exposed)

---

## Versioning & Reproducibility

### V1 — Toolchain Version Locks
All runtime toolchain components must be version-pinned and recorded:
- Node.js version
- pnpm version
- OpenCode CLI version
- Gemini CLI version
- Remotion skills source **commit hash** (or immutable release reference)

**Rules**
- The app must not “auto-upgrade” any toolchain component without an explicit user action.
- When installing/upgrading any component, record the exact resolved version (and installer source) in `state.json`.
- When spawning provider processes, prefer the locally pinned versions from `<AppDataDir>/toolchain/`.

### V2 — Skills Version Pinning
- Remotion skills must be installed from an immutable reference:
  - Prefer: `owner/repo#<commit>`
  - Acceptable: tagged releases that map to a commit
- The app must store the resolved commit hash even when the user installs via a branch-like reference.

### V3 — Workspace State Tracking (`state.json`)
Maintain a durable state file at `<AppDataDir>/state.json` (or equivalent) that includes per-workspace records.

**Minimum schema (conceptual)**
- `workspaces[<workspace_id>].path`
- `workspaces[<workspace_id>].provider` = `opencode | gemini`
- `workspaces[<workspace_id>].last_env_check_at` (ISO timestamp)
- `workspaces[<workspace_id>].toolchain`:
  - `node.version`
  - `pnpm.version`
  - `opencode.version` (if used)
  - `gemini.version` (if used)
- `workspaces[<workspace_id>].skills`:
  - `remotion.source` (e.g., `remotion-dev/skills`)
  - `remotion.commit` (resolved)
  - `remotion.installed_at`

**Behavior**
- On app startup and on workspace open, read `state.json` to decide whether checks/installs are needed.
- If the local toolchain deviates from recorded versions, surface a clear “drift” status in UI and offer:
  - Reinstall pinned versions
  - Update pins (explicit action)

---

## Acceptance Criteria
- Selecting OpenCode launches OpenCode in workspace with isolated PATH.
- Selecting Google launches Gemini CLI in workspace; login can complete.
- First-run wizard correctly detects missing deps, installs locally, and updates UI.
- No global installs are required for a clean user machine.
- Remotion skills are installed and discoverable for both providers.
- Provider can follow the strict loop to modify `src/Composition.tsx` and validate by preview/typecheck.
- Toolchain versions and Remotion skills commit are pinned and recorded; workspace state is persisted in `state.json`.

---

## Risks & Mitigations
- Windows portable Node quirks → ship a known-good distribution + add PATH shims.
- Gemini CLI install/login variations → treat as a separate install stage with clear UI guidance.
- Skill format differences between OpenCode and Gemini → provide an adapter layer that maps “Remotion skill” into each provider’s mechanism.

---

## Open Questions (Track but do not block drafting)
- Exact binary name and install method for Gemini CLI on Windows.
- OpenCode skills install path in your embedded runtime.
- Whether to install Gemini skills globally or per-workspace.

