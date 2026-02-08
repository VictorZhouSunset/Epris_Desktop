---
name: planning-with-files
description: Break down and execute complex tasks using structured Markdown files for planning, tracking, and deliverables.
metadata:
  short-description: Manage tasks via files-based planning system.
---

# Planning with Files

This skill helps you systematically plan and execute complex tasks by using three Markdown files to track the planning process, execution progress, and final deliverables.

## Files used

1. `task_plan.md`: Detailed breakdown and step-by-step plan.
2. `progress.md`: Current execution status.
3. `findings.md`: Final results, decisions, and deliverables.
4. `.epris/agent/active-plan.md`: User-facing high-level checklist (non-technical language only).

## Workflow

### 1) Create and maintain `task_plan.md`

When a task begins:

- Create `task_plan.md` (if it doesn't exist).
- Break down the task into a clear, structured plan.
- For each step, include:
  - A checkbox `- [ ]`
  - A short description
  - Any relevant file paths, commands, or sub-goals

Example:

```md
- [ ] Investigate existing API endpoints
- [ ] Add missing validation in backend
- [ ] Update frontend types from OpenAPI schema
- [ ] Run tests and verify behavior
```

Keep `task_plan.md` updated as the plan evolves.

### 2) Track work in `progress.md`

When execution begins:

- Create `progress.md` (if it doesn't exist).
- Update progress as tasks are worked on.
- Use sections:
  - **In Progress**
  - **Completed**
  - **Blocked**

Example:

```md
## In Progress
- Add missing backend validation

## Completed
- Investigate API endpoints

## Blocked
- Waiting for required external dependency installation
```

### 2.5) Maintain user-facing `.epris/agent/active-plan.md`

- Create or replace `.epris/agent/active-plan.md` before implementation starts.
- Keep this file high-level and structural only:
  - What visible element/scene/action is being built
  - In what order
  - What behavior is being added
- Do **not** include coding details:
  - No file paths
  - No commands
  - No APIs/libraries/tooling internals
- Use only checklist lines `- [ ] ...` / `- [x] ...`.
- As each step completes, immediately update the matching line to `- [x]`.

### 3) Record final output in `findings.md`

When the task is complete (or a checkpoint is requested):

- Create `findings.md` (if it doesn't exist).
- Summarize outcomes, changes made, and key decisions.
- Include:
  - What was done
  - Files modified
  - Commands run / tests executed
  - Open questions / follow-ups

Example:

```md
## Summary
Implemented backend validation and regenerated frontend types.

## Files Changed
- backend/app/routes/users.py
- web/src/types/api.ts

## Verification
- `pytest`
- `pnpm test`
```

## Operating rules

- Update `task_plan.md` before major execution begins.
- Update `progress.md` during execution.
- Keep `.epris/agent/active-plan.md` synced with actual execution status.
- Finalize with `findings.md` when the task completes.
- If the user requests updates mid-task, consult `progress.md` and report accurate status.
- Do **not** ask the user to confirm the plan or approve each step.
- Execute autonomously end-to-end unless blocked by a true external dependency or missing credential.
- If assumptions are needed, choose the safest reasonable one, document it in `progress.md`, and continue.
