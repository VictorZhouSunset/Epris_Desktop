# V1 UI Agent (Right Panel Prompt + Prop Controls) — Design

**Goal:** Add a right-side “UI Agent” prompt that can expose/edit Remotion `Main` props (colors, numbers, selects, etc.) and render them as prebuilt UI controls that instantly update the preview and are used for export.

**UI Reference:** The repo contains `web_apps/` (gitignored) with a former web editor implementation whose interaction model (objects list → property panel → timeline/overlay) is a reference point for future UX.

## Problem Statement

Today Epris can:
- Ask an AI provider (OpenCode/Gemini) to edit `src/Composition.tsx` (and sometimes `src/Root.tsx`) to generate/modify a Remotion video.
- Preview via an iframe that loads the workspace Vite dev server (`pnpm run dev`), which renders `src/Preview.tsx` using `@remotion/player`.
- Export via Remotion CLI (`remotion render src/Root.tsx Main ...`).

V1 adds:
- A second prompt area in the right panel: “UI Agent prompt”.
- The UI Agent uses the current provider CLI to refactor the workspace code so “tunable” values become `Main` props and a machine-readable control spec is updated.
- The app renders those props as prebuilt form widgets (slider, color picker, select, etc.).

## Non-Goals (for V1)

- Full “semantic composite controls” that programmatically derive from multiple primitives without AI code refactors (defer to V1.5).
- Automatic AST-based inference of tunables from arbitrary TS/JS (too brittle for V1).
- A full visual inspector / element picking in the preview.

## Core Design Choice: “Controls Spec + Values” as Workspace Files

To make UI rendering deterministic (and provider-agnostic), introduce two workspace files under `src/`:

1) `src/epris-controls.json` — “what controls exist + how to render them”
2) `src/epris-props.json` — “current values for those controls (and default export values)”

Why in `src/`?
- Already within the AI sandbox whitelist (`src/`, `public/`) and snapshot/backup systems.
- Easy for providers to read/write consistently.
- Avoids parsing provider chat output as a protocol.

## ID Naming Rule (Uniqueness + JS Identifier Constraint)

Because control ids become `Main` prop keys, require:
- `id` must be unique within the spec.
- `id` must be a valid JS identifier (no spaces/special chars).
  - Recommendation: `^[A-Za-z_$][A-Za-z0-9_$]*$`
  - Practical tip for the agent: use `camelCase` like `rectColor`, `particleSpeed`, `themeSize`.

## Contracts

### C1 — Prop Contract

- `Main` in `src/Composition.tsx` must accept a single props object.
- All tunable UI parameters should be hoisted into those props (instead of hard-coded constants in the component body).

### C2 — Controls Contract

- Every control in `src/epris-controls.json` maps to a key in `src/epris-props.json`.
- Every key in `src/epris-props.json` is a prop that `Main` can accept (directly or via a stable mapping layer in `Composition.tsx`).

### C2.1 — Targeting / Composition Scoping (Future-proofing)

V1 typically targets only `Main`, but reserve a target field now so multi-composition support doesn’t require a schema rewrite.

Design proposal:
- Controls spec contains a `target`:
  - `{ "kind": "composition", "id": "Main" }` (default)
  - `{ "kind": "global" }` (reserved for future shared/default controls)

### C3 — Export Contract

- `src/Root.tsx` must pass `defaultProps` to `<Composition ... />` using the parsed contents of `src/epris-props.json`.
- Export always uses the latest saved `src/epris-props.json` values.

### C4 — Preview Contract (fast feedback)

- Epris app posts messages to the iframe to update `inputProps` in `src/Preview.tsx` (instant updates without a full reload).
- The app also persists values by updating `src/epris-props.json` (debounced), so export matches preview.

### C4.1 — Preview Initial Sync (avoid “UI shows A, preview shows B”)

On iframe refresh, preview must receive initial props deterministically.

Recommendation:
- Implement a handshake:
  - `Preview.tsx` posts `epris:ready` to `window.parent` on mount.
  - The app responds with `epris:setInputProps` containing the current props.
- Optionally, show a lightweight “syncing props” placeholder until the first props payload arrives (or a short timeout fallback).

## Instrumented Primitives (Route B — Selected)

To support “scan elements”, a cleaner right-panel hierarchy, and eventually overlay/timeline, we introduce **optional** instrumented primitives in the workspace template.

Key principle: **do not restrict freeform AI code generation**.
- If the AI can use primitives, we get better inspectability.
- If the AI needs raw React/Remotion/Canvas/WebGL tricks, it can still do them; that part just won’t be fully inspectable.

Design sketch:
- Provide wrappers like:
  - `<EprisGroup id label kind>...</EprisGroup>`
  - `<EprisRect id label ... />`, `<EprisText ... />`, `<EprisImage ... />` (minimal set)
- Each primitive registers metadata into an in-iframe registry:
  - `id` (JS identifier, stable)
  - `label` (human friendly)
  - `kind` (`shape | text | image | video | background | particles | group | other`)
  - optional tags/categories
- `Preview.tsx` can answer requests from the desktop app via postMessage:
  - “give me the current object registry snapshot”
  - (later) “for object X, give me bounds” / “highlight object X”

This gives us a path to UI like your former webapp (object list → property panel → optional overlay),
without forcing a schema-driven renderer.

## UI Controls: Prebuilt Component Registry (Epris App)

The Epris app ships a small “control registry”:
- `number/slider`
- `number/input`
- `color`
- `select`
- `boolean/toggle`
- `text`

Later (V1.5+):
- vector2 / vector3
- file pickers (asset binding)
- composite controls (see below)

## Proposed JSON Schema (V1)

File: `src/epris-controls.json`

High level:
- `schemaVersion` (number)
- `compositionId` (string, typically `"Main"`)
- `controls[]` (ordered list)

Each control:
- `id` (string, unique key; also prop key)
- `label` (string)
- `type` (`"number" | "color" | "select" | "boolean" | "text"`)
- `ui` (render hint, e.g. `"slider"` for numbers)
- optional constraints (min/max/step/options)
- optional grouping (`group`)

File: `src/epris-props.json`
- `schemaVersion` (number)
- `values` (object keyed by control `id`)

This format keeps V1 simple:
- “Flat props” are the default.
- “Composite” behavior is achieved in V1 mostly by AI refactoring (e.g., introduce a single `squareSize` prop used for both width & height in code).

## Persistence & Real-time (Debounce vs Save Button)

Observations:
- UI updates and preview response should be immediate (smooth slider drag).
- Writing to disk too frequently can be noisy and may trigger dev-server/HMR work depending on tooling.

Two viable models:
1) **Debounced autosave (fallback, simpler to implement)**:
   - UI state: immediate
   - postMessage: immediate
   - disk write (`src/epris-props.json`): debounce 500ms–1000ms
2) **Explicit Save (stronger UX, more work)**:
   - UI + postMessage: immediate (draft state)
   - disk write only on “Save”
   - Export must either (a) force-save or (b) ask user to save first
   - Snapshot DAG semantics become cleaner: “Save” creates a snapshot boundary; undo can revert to last save

**Recommendation (given “freeform code generation first” + smooth UX): Choose Explicit Save for V1.**

Concretely:
- `src/epris-props.json` is the **saved/export** source of truth.
- The app maintains an in-memory `draftProps`:
  - UI updates: immediate
  - postMessage to preview: immediate
  - disk write: only on **Save** (and/or “Save before export”)

Implications:
- The app must surface “unsaved changes” state (draft != saved).
- Export must auto-save draft props first, or require the user to save.
- When triggering UI Agent again, include `draftProps` in the prompt context (so the agent doesn’t rely only on on-disk values).

## Provider (AI) Responsibilities

When the UI Agent prompt runs, the provider must:
- Identify the relevant tunable(s) from the user request (e.g. rectangle color).
- Refactor `src/Composition.tsx` so the tunable becomes a prop on `Main` (or a prop consumed by child components).
- Add/update entries in `src/epris-controls.json` (control definition).
- Add/update defaults in `src/epris-props.json` (current value).
- Ensure `src/Root.tsx` continues to render `Main` and uses `epris-props.json` for `defaultProps`.
- Keep `pnpm-lock.yaml`, `package.json`, etc. unchanged (existing guardrails).

## System Prompt: Atomicity Requirement (avoid half-updates)

To prevent UI breakage, the UI-Agent system prompt should explicitly require:
- Update `src/Composition.tsx` (logic)
- Update `src/epris-controls.json` (definitions)
- Update `src/epris-props.json` (initial values)

“Missing any one of these is a failure.”

## App Responsibilities

- Load `src/epris-controls.json` + `src/epris-props.json` via backend IPC.
- Render controls in the right panel using prebuilt components.
- On change:
  - Update in-memory `draftProps` immediately.
  - PostMessage `draftProps` to iframe for instant preview update.
- On Save:
  - Persist `draftProps` into `src/epris-props.json` via backend IPC.
  - (Optional) create a snapshot/DAG checkpoint to support “revert to last save”.

## Composite / “Compository” Props (Your Question #2)

Recommendation:
- **V1:** Do not add a separate “composite control type” protocol yet.
  - Instead, teach the UI Agent to *prefer creating a single new prop* that drives multiple internals (e.g. `squareSize` used for both width and height; `particleSpeed` used across the system).
- **V1.5:** Add first-class composite controls that map to multiple props without forcing code refactors every time, e.g.:
  - `type: "composite:number"` with `targets: [{path:"rectWidth", scale:1}, {path:"rectHeight", scale:1}]`
  - “semantic” groups like `particleSystem.speed` across multiple components.

This keeps V1 shippable while still enabling “one slider changes many things” via AI refactors.

## Code Organization (Your Question #3)

V1 guidance:
- Keep `src/Composition.tsx` as the stable entrypoint exporting `Main`.
- Allow optional extraction into `src/scenes/*` / `src/components/*` when code grows.
- Keep the “UI contract files” stable:
  - `src/epris-controls.json`
  - `src/epris-props.json`
  - `src/Root.tsx` imports `epris-props.json` for `defaultProps`
  - `src/Preview.tsx` listens for `postMessage` to update `inputProps`

This gives the agent structure without forcing it to juggle many moving entrypoints.

## Prompt UX (Your Question #1)

Recommend adding a lightweight “hint” to the **main** code-writing prompt UI:
- Encourage “prop-first” authoring for tunables (colors/sizes/text/fonts/speeds).
- Encourage stable naming and human-friendly labels.
- Mention that these props can be exposed later by the UI agent (so it should not hardcode everything).

Also update `REMOTION.md` generator so providers are reminded of the `epris-controls.json` / `epris-props.json` contract.

## Related Future Features (Likely V1.5+): Object Scan + Inspector + Timeline/Overlay

You described two larger features:
1) A “scan video elements” button that produces an object list grouped by categories (style/layout/content/motion/…), with optional “only show currently visible at time t”.
2) A selection-based overlay + timeline with keyframes (diamonds), and only editable at keyframes.

These are a significant scope jump because they require a stable “scene graph” representation and (ideally) runtime visibility/selection metadata.

Three possible product directions (recorded for completeness):
- **A) Structured mode (schema-driven rendering):** A JSON scene graph is the source of truth; `Composition.tsx` renders from it. This makes timeline/keyframes first-class, but constrains freeform code generation (not chosen).
- **B) Instrumented primitives (hybrid):** Optional wrapper components register metadata at runtime (chosen).
- **C) AI-only static scan:** Provider outputs an “object catalog” JSON (fallback when primitives aren’t used).

My recommendation for roadmap:
- V1: UI Agent + primitive prop controls (this doc).
- V1.5: Use (B) registry to power “Objects” UI; use (C) as fallback for non-instrumented code paths.
- Later: overlay + timeline + true keyframes only if we introduce a sufficiently reliable object model (strong B, or eventually A if you ever decide to pivot).

### V1.5 Proposal: “Scan Elements” Output Format + UI (B + C combo)

If we do (C) first, the scan button can ask the provider to generate a catalog file, for example:
- `src/epris-objects.json`

High-level shape (draft):
- `schemaVersion`
- `objects[]` where each object has:
  - `id` (JS identifier; stable; used for selection)
  - `label` (human friendly)
  - `kind` (e.g. `shape | text | image | video | particles | background | group`)
  - `time` (optional): `{ startFrame?: number, endFrame?: number }` (best-effort)
  - `properties[]` (what user can edit), each property references existing controls:
    - `controlId` (must exist in `epris-controls.json`)
    - `category` (`style | layout | content | motion | effects | audio | other`)
    - `labelOverride?` (optional)

UI recommendation (right panel):
- Add an **Objects** tab (separate from “Props/Parameters”).
- Show an object list (collapsed by default).
  - Clicking an object shows its properties grouped by category (so it doesn’t feel chaotic).
  - Default view shows “currently visible” objects first.
    - With pure (C), “visibility” is best-effort (based on `time` if present).
    - With (B) primitives, preview can report a live set of “registered objects” + visibility at the current frame.
- Advanced: allow “show hidden / not currently visible” toggle, matching your idea.
