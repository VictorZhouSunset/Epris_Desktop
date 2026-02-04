# Project Specific Instructions

## 1. Project Background

Name: Epris
Aim: To create a desktop application that can generate code videos using AI and Remotion. The users should be able to create code videos without any coding knowledge or experience any code-related impressions.

## 2. Project Plan

The full plan is in: docs\Epris_Desktop_Version_Plans.md, however, a lot of the plans need to be updated and restructured so don't take them as a final plan.

The previous implementation plan of this project (which is the V0 stage) is:

- docs\Archived\Epris_v0_development_plan.md
- docs\Archived\step_11_gemini_cli_open_code_provider_first_run_wizard_remotion_skills_spec.md
- docs\Archived\step_12_13_14.md
  You should read these documents carefully and research into the project's current state before starting to work on it. If you find any discrepancies or errors, please **notify me first** before proceeding.

I have put the two main references for step 11 in your skills: gemini-cli and opencode-server, these two give you a basic idea of how to integrate these two CLI into Epris.

Right now, I am implementing the V0.5 plan, which is in docs\epris_v_0.5.md.

# Modern Full-Stack Development Guidelines

**Key Notice**: You are working in a WSL sandbox, so some "pnpm" operation will be in conflict with my "pnpm" in Windows, such as pnpm i, pnpm reinstall, node-gyp, etc. Try not to do these by yourself, ask me to do it on Windows side.
I write a script in scripts folder (bootstrap-wsl.sh), running it should give you ability to run cargo tests.
You **should not** fix a problem by patches or circulate the problem, you need to first discuss with me, and study the codebase thorough so that we agree on the problem, then you can come up with a solid solution.

You **can** deviate from the requirements below (especially when you are explicitly asked to use the framework other than those listed below), but if you do so, you **must** provide detailed explanation and **seek approval or confirmation before** you execute on these.

## 1. Project Management & Tooling

- **Python:** **Exclusively use `uv`** for all package management and virtual environment tasks. Manage dependencies **solely** via `pyproject.toml` (Project Mode).
- **JavaScript:** **Exclusively use `pnpm`** for dependency management.
- **Project Structure:** For a webapp project, default to a Monorepo structure, maintaining strict separation between `/backend` (FastAPI) and `/web` (Next.js). If the project is other form, there is not yet any standard yet.

## 2. Backend Standard (Python)

- **Framework:** **Default to FastAPI**.
- **Linting:** **Enforce** code quality using **Ruff**.
- **Data:** **Define** data schemas using **Pydantic** models. **Create distinct** models for Requests (Input) and Responses (Output).
- **Paths:** **Utilize `pathlib`** for all file system interactions.

## 3. Frontend Standard (TS/Next.js)

- **Framework:** **Default to Next.js (App Router)**. But **Vite** is also acceptable when you think it is more appropriate or the user asks to use Vite.
- **Language:** **Write strictly in TypeScript(Strict)**.
- **Styling:** **Use Tailwind CSS** for all styling needs.
- **State/Fetching:** **Utilize** SWR or TanStack Query for client-side fetching.

## 4. API Integration

- **Truth Source:** **Treat** the FastAPI-generated OpenAPI JSON as the **single source of truth**.
- **Type Sync:** **Generate** frontend types directly from the OpenAPI schema (e.g., using `openapi-typescript`). **Do not** manually duplicate interfaces.

## 5. Development Protocol (Decision Gate)

**Before coding, assess the task type:**

- **Logic-Heavy Tasks:** (Algorithms, Data Processing, API Logic, Complex State)
  - **Action:** **Activate** the `tdd-workflow` skill.
  - **Constraint:** Do not write implementation code until the test case is defined.

- **Visual/Config Tasks:** (CSS, Animations, Simple Setup)
  - **Action:** Prototype directly and verify manually via browser.

- **Bug Fixes:**
  - **Action:** **Construct** a self-contained reproduction script (`repro.py` or `repro.ts`) **before** attempting fixes.

## 6. Security

- **Config:** **Retrieve** all sensitive configuration **strictly** from Environment Variables (`.env`).

## 7. Shell Environment Rules

- The default shell environment is **Windows PowerShell (PS)**.
- **Do NOT use Unix/Linux-only commands** in PowerShell, including but not limited to:
  - `grep`, `sed`, `awk`, `ls`, `cat`, `cp`, `rm`, `find`
- When searching text in files under PowerShell, **use `Select-String` instead of `grep`**.
- When listing files or directories, **use `Get-ChildItem` instead of `ls`**.
- If a command requires Unix tools, explicitly switch context and say:
  - “Run this in Git Bash / WSL”, and use Unix-style paths (`/`).
- Always match command syntax and path separators to the active shell.
