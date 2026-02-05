---
name: superpowers
description: Use the obra/superpowers skill system (already bootstrapped)
---

# Superpowers (obra/superpowers)

This project assumes Superpowers is already installed + bootstrapped in your Codex environment. This repo-local skill exists to document the day-to-day commands and point to the vendored reference docs so contributors follow the same workflow.

Upstream: `https://github.com/obra/superpowers`

## Day-to-day usage

- Load a Superpowers skill into the current session:
  ```bash
  ~/.codex/superpowers/.codex/superpowers-codex use-skill <skill-name>
  ```

- Re-run the bootstrap (usually only needed after updating your global setup):
  ```bash
  ~/.codex/superpowers/.codex/superpowers-codex bootstrap
  ```

## How this interacts with project skills

- Project-local skills live in `.codex/skills/` (available as soon as you open this repo).
- Superpowers skills live in `~/.codex/superpowers/skills/` and are loaded on demand via `use-skill`.
- If a project-specific skill exists for the task, prefer it; otherwise load the relevant Superpowers skill.

## Reference (vendored into this repo)

- Codex-specific bootstrap prompt + tool mapping: `./superpowers-bootstrap.md`
- First-time installation guide (only if you *haven't* already bootstrapped): `./INSTALL.md`

## If you have not installed Superpowers yet

Follow `./INSTALL.md`. This repo intentionally does not vendor the entire upstream Superpowers repository.
