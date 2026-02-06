# V1 UI Agent Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a right-panel UI Agent prompt that exposes `Main` props as prebuilt UI controls, updates preview live, and persists values for export (via explicit Save).

**Architecture:** Providers update `src/epris-controls.json` + `src/epris-props.json` and refactor `src/Composition.tsx` to accept those props. The app reads those JSON files and renders a control registry (slider/color/select/etc.), posts *draft* props into the preview iframe for instant updates, and writes draft props to `src/epris-props.json` only on Save (and automatically before export). Workspace also ships optional instrumented primitives (Route B) for future “Objects” UI.

**Tech Stack:** Tauri (Rust commands), React + TypeScript (Vite), Remotion (`@remotion/player`, `@remotion/cli`), JSON schema validation (Rust + TS).

**UI Libraries (preferred):**
- Use mature headless UI primitives where helpful (e.g. Radix UI) instead of bespoke widgets.
- For color picking (V1): use `react-colorful` (MIT, lightweight). Install via Windows `pnpm` in `apps/epris-tauri/`.
- For V1.5+ overlay/transform handles, consider `react-rnd` or similar (but keep it out of V1).

---

## Task 0: Confirm Contracts (no code)

**Decisions to confirm:**
- Store control spec in `src/epris-controls.json` (not in `.epris/`, not in provider output text).
- Store current values in `src/epris-props.json` to ensure export matches UI.
- V1 supports primitive controls; composite behavior is mainly achieved by AI refactors (V1.5 adds first-class composite controls).
- Confirm `id` regex constraint for prop keys: `^[A-Za-z_$][A-Za-z0-9_$]*$`.
- Confirm controls `target` scope shape: `{kind:"composition", id:"Main"}` reserved vs `{kind:"global"}` reserved.
- Persistence decision (V1): **Explicit Save** (draft in memory; write to disk only on Save and automatically before export).

**Deliverable:**
- Agree on the JSON schema fields and supported control types in V1.

---

## Task 1: Add Workspace Template Stubs

**Files:**
- Modify: `workspace-template/src/Preview.tsx`
- Modify: `workspace-template/src/Root.tsx`
- Modify: `workspace-template/src/Composition.tsx` (only minimal “prop-friendly” scaffold)
- Create: `workspace-template/src/epris-controls.json`
- Create: `workspace-template/src/epris-props.json`
- Create: `workspace-template/src/epris/*` (instrumented primitives, minimal registry)

**Steps:**
1. Add `postMessage` listener in `Preview.tsx` to accept `inputProps` updates and pass them to `<Player inputProps={...} />`.
2. Add a handshake to avoid initial desync:
   - On mount, `Preview.tsx` posts `epris:ready` to parent.
   - Parent replies with `epris:setInputProps`.
   - Optional: show “syncing” placeholder until first payload (or short timeout).
3. Update `Root.tsx` to load `epris-props.json` and pass `defaultProps` into `<Composition ... />`.
4. Update `Composition.tsx` to define an exported `MainProps` type that matches keys in `epris-props.json` (initially empty/optional) and accepts props.
5. Add stub JSON files with `schemaVersion: 1`, empty controls/values.
6. Add minimal instrumented primitives (Route B):
   - `EprisGroup` + a tiny registry module (V1 only; avoid many wrappers)
   - registry can return an object list snapshot for future “scan elements”
   - (do not implement overlay/timeline in V1)

**Manual verification:**
- Create a new project and confirm preview still loads.
- Run existing smoke scripts (Windows side): `pnpm -s run smoke:0` and `pnpm -s run smoke:mid`.

---

## Task 2: Backend IPC for Controls + Props Files

**Files:**
- Create: `apps/epris-tauri/src-tauri/src/ui_props.rs` (or similar)
- Modify: `apps/epris-tauri/src-tauri/src/lib.rs` (invoke handler)
- Modify: `apps/epris-tauri/src-tauri/src/sandbox.rs` (only if needed for audit/allowlist)

**Steps:**
1. Add `get_epris_controls(workspace_path)` and `get_epris_props(workspace_path)` commands.
2. Add `set_epris_props(workspace_path, values)` command that writes `src/epris-props.json`.
3. Add `get_epris_objects_snapshot(workspace_path)` (V1 minimal scan):
   - Sends a postMessage request to the preview iframe? (Not possible from Rust)
   - Therefore implement scan entirely in frontend via iframe postMessage (preferred for V1).
3. Validate JSON shape before write:
   - schemaVersion supported
   - object types
   - `values` only contains keys that match the `id` regex (JS identifier)
   - payload size limits (to prevent accidental megabytes)
   - Validate `epris-controls.json` ids too (when adding a “set controls” command later, or when the UI Agent run completes and the app reloads)
4. Ensure writes stay within `projectRoot/src/**` (existing sandbox conventions).

**Manual verification:**
- Load project: app can fetch controls/props and shows “no controls” state without crashing.

---

## Task 3: Frontend Control Registry + Rendering

**Files:**
- Create: `apps/epris-tauri/src/components/controls/*` (prebuilt widgets)
- Create: `apps/epris-tauri/src/types/ui-controls.ts`
- Modify: `apps/epris-tauri/src/components/ParametersPanel.tsx`
- Modify: `apps/epris-tauri/src/lib/ipc.ts`

**Steps:**
1. Add frontend deps (Windows `pnpm`):
   - `react-colorful`
   - `zod`
2. Define TS types for `EprisControlsSpec` + `EprisPropsFile`.
3. Add Zod schemas for robustness:
   - Validate `epris-controls.json` and `epris-props.json` on load.
   - If invalid, show a non-fatal error panel with details and block rendering controls (avoid UI crash).
   - Enforce the `id` regex in Zod (same regex as backend).
4. Implement control components:
   - Slider (number)
   - Color picker (react-colorful)
   - Select
   - Toggle
   - Text input
5. Render controls below existing video config section (or add a tab/accordion).
6. On change:
   - Update in-memory `draftProps` immediately
   - Emit immediate preview update (via a callback to Task 4’s postMessage sender)
   - Do **not** write to disk here
7. Add Save UX:
   - “Save” button writes `draftProps` to `src/epris-props.json`
   - “Revert” button resets draft back to last saved file values
   - Unsaved indicator (draft != saved)
   - Export always does “Save & Export” automatically if there are unsaved draft props
8. Add empty/error states (missing JSON, invalid JSON).

**Manual verification:**
- Hand-edit `src/epris-controls.json` and `src/epris-props.json` in a workspace; the panel renders controls and can persist changes.

---

## Task 4: Preview Live Update via postMessage

**Files:**
- Modify: `apps/epris-tauri/src/App.tsx` (iframe ref + postMessage)
- Modify: `apps/epris-tauri/src/components/ParametersPanel.tsx` (emit updates)

**Steps:**
1. Keep an `iframeRef` to the preview iframe.
2. On iframe load and on `epris:ready`, `postMessage` `{type:"epris:setInputProps", payload:{...values}}`.
3. On props change, `postMessage` immediately for smooth updates.
4. In workspace `Preview.tsx`, update player `inputProps` state on message.
5. Ensure “video meta config” (duration/width/height) remains governed by the existing Apply flow and preview reload; do not mix those into `draftProps`.

**Manual verification:**
- Move a slider: preview changes instantly without waiting for full reload.

---

## Task 5: UI Agent Prompt (Right Panel)

**Files:**
- Modify: `apps/epris-tauri/src/components/ParametersPanel.tsx` (new prompt input)
- Modify: `apps/epris-tauri/src/lib/ipc.ts`
- Modify: `apps/epris-tauri/src/hooks/usePrompt.ts` (optional: new hook for UI agent)
- Modify: `apps/epris-tauri/src-tauri/src/opencode.rs` (new command, or reuse `send_prompt` with a `mode`)
- Modify: `apps/epris-tauri/src-tauri/src/provider.rs` / `REMOTION.md` generator (rules update)

**Steps:**
1. Add “UI Agent prompt” text area + send button in right panel.
2. Add backend command `SEND_UI_PROMPT` (or `SEND_PROMPT` with `mode: "ui"`):
   - Different system prompt emphasizing:
     - Update `src/epris-controls.json` + `src/epris-props.json`
     - Refactor `Main` props accordingly
     - Keep changes within `src/**`
     - Atomicity: must update `Composition.tsx` + controls + props together
     - ID rule: all ids must match JS identifier regex; if not possible, rename to a valid id and keep a human label.
     - Composite strategy: if one slider should drive many values, create a new prop and distribute inside `Composition.tsx` (e.g. `themeSize`, `particleSpeed`)
     - Route B: when creating major “objects”, prefer wrapping them with `<EprisGroup id label kind>` so the Objects/Scan UI can list them (optional; do not force).
3. When sending the UI Agent prompt, include the current `draftProps` as context in the request payload (do not force-write it to disk).
4. After completion, reload controls/props and update UI.
   - Set `draftProps` = newly loaded saved props so UI/preview are consistent.

**Manual verification:**
- Prompt: “Change the background color of the rectangle” results in:
  - new control in `epris-controls.json`
  - value in `epris-props.json`
  - `Composition.tsx` uses the prop
  - right panel shows a color picker

---

## Task 6: Main Code-Writing Prompt UX Improvements (Question #1)

**Files:**
- Modify: `apps/epris-tauri/src/App.tsx` (placeholder hint / helper text)
- Modify: `apps/epris-tauri/src-tauri/src/provider.rs` (REMOTION.md generator rules)

**Steps:**
1. Add a small hint near the main prompt input: “Prefer exposing tunables as props; UI Agent can surface them later.”
2. Add a short “Prop-first authoring” section into generated `REMOTION.md`.

---

## Task 7: Gate / Validation Extensions (Optional for V1, recommended)

**Files:**
- Modify: `apps/epris-tauri/src-tauri/src/gate.rs`

**Steps:**
1. Add a workspace contract validator script (must be inside `src/**` so it is within the sandbox/snapshot scope):
   - Create: `workspace-template/src/epris/validate-contract.mjs`
   - Add script in `workspace-template/package.json`:
     - `"epris:validate": "node src/epris/validate-contract.mjs"`
2. `epris:validate` checks (fail with non-zero exit code + clear error output):
   - `epris-controls.json`: schemaVersion, unique ids, id regex, per-type constraints (min/max/options).
   - `epris-props.json`: schemaVersion, values object, required keys for each control, per-type value constraints.
   - `Root.tsx`: passes saved props as `defaultProps` for the `Main` composition (regex/AST check).
   - Optional (warn-only): scan `Composition.tsx` text for control ids to detect obvious “controls don’t affect anything” cases.
3. Integrate into Gate:
   - Run `pnpm -s run epris:validate` before `pnpm -s run typecheck` and smoke stills.
4. If invalid, surface a clear gate error so the agent can fix it (feedback loop for UI Agent).

---

## V1.5 Follow-ups (Not in V1)

- Composite control protocol (one UI widget maps to many props).
- Nested prop paths (dot-path bindings) if needed.
- UI inspector / element picker from preview.
- Better diff + rollback UX if UI agent generates bad controls.
- “Scan elements” button that produces an object catalog grouped by categories; decide between:
  - Instrumented primitives registry (chosen direction),
  - AI-only static scan (fallback for non-instrumented paths).

## Task 8 (V1): Snapshot/DAG Integration for “Save Props”

**Goal:** Ensure prop saves are tracked in snapshot DAG and export is reproducible.

**Files:**
- Modify: `apps/epris-tauri/src-tauri/src/snapshot.rs` (if needed for ergonomics)
- Modify: `apps/epris-tauri/src-tauri/src/lib.rs`
- Modify: `apps/epris-tauri/src/hooks/useExport.ts`
- Modify: `apps/epris-tauri/src/components/ParametersPanel.tsx`

**Steps:**
1. When user clicks “Save” in the right panel:
   - Persist `draftProps` to `src/epris-props.json`
   - Create a `manual_save_snapshot` node with a name like `UI: Save props` (description includes a short diff summary if easy)
2. When user clicks Export:
   - If `draftProps` differs from saved, do the same “Save” flow first (auto Save & Export)
3. After `checkout_snapshot`, reload controls/props and reset `draftProps` so UI matches the snapshot state.

## Task 9 (V1): Minimal Scan UI (Objects Tab)

**Goal:** Provide a minimal “Scan” that lists instrumented objects in the right panel, inspired by `web_apps/` UX.

**Files:**
- Modify: `apps/epris-tauri/src/components/ParametersPanel.tsx` (tab + list UI)
- Modify: `apps/epris-tauri/src/App.tsx` (postMessage request/response wiring)
- Modify: `workspace-template/src/epris/*` (registry snapshot API)
- Modify: `workspace-template/src/Preview.tsx` (respond to `epris:getObjectsSnapshot`)

**Steps:**
1. Add an **Objects** tab in the right panel with a “Scan” button.
2. On Scan, send `postMessage` to iframe: `{type:"epris:getObjectsSnapshot"}`
3. Preview replies with `{type:"epris:objectsSnapshot", payload:{objects:[...]}}`
4. Render object list (collapsed by default); clicking an object shows its metadata (label/kind/tags).
5. Do not attempt timeline/overlay/keyframes in V1.
