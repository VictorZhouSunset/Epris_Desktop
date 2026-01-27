# Plan: Fix Export Progress & Open Folder Source

## Context

User reported:

1. Export progress jumps from 0% to 100% instantly (no intermediate updates).
2. "Open Folder" button fails or opens wrong path.
3. Need to reset export state after opening folder.

## Root Cause Analysis

1. **Export Progress**:
   - `export.rs` uses `child.stderr.lines()`, which is a buffered reader.
   - CLI tools (like Remotion/FFmpeg) often use `\r` (Carriage Return) to update progress on the same line.
   - `lines()` waits for `\n`, so it buffers everything until the end.
   - **Fix**: Read stream byte-by-byte or chunks and parse `\r`.

2. **Open Folder**:
   - `tauri-plugin-opener` capabilities in `src-tauri/capabilities/default.json` are too restrictive.
   - Need to allow opening any path (or specifically the output directory).
   - **Fix**: Update `default.json` with wildcard permission.

3. **Reset Logic**:
   - `useExport.ts` stays in `done` state.
   - **Fix**: Reset to `idle` after "Open Folder" is clicked.

## Work Checklist

- [x] **Backend Refactor** (`export.rs`)
  - Switch to `tokio::io::AsyncRead` manually.
  - Parse `stdout` AND `stderr` for percentage (e.g. `(45%)`).
  - Emit events for every update.

- [x] **Permissions** (`capabilities/default.json`)
  - Add `opener:allow-open-path` with `path: "**"`.

- [x] **Frontend Logic** (`useExport.ts`)
  - Add `setStatus('idle')` in `openOutputFolder`.
  - Ensure type definitions match new backend event payload.

- [x] **Verification**
  - Build and tested manually.
