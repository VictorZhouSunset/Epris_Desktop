---
name: superpowers
description: Bootstrap and use the obra/superpowers skill system for Codex
---

# Superpowers (obra/superpowers)

This repo doesn't ship a standalone Codex `SKILL.md` you can install via the `skill-installer` script (it requires `SKILL.md` in the upstream path). This wrapper skill vendors the upstream setup docs into this repo so you can follow them consistently.

Upstream: `https://github.com/obra/superpowers/tree/main`

## Install (global, recommended by upstream)

```bash
mkdir -p ~/.codex/superpowers
git clone https://github.com/obra/superpowers.git ~/.codex/superpowers
```

Then add this section to `~/.codex/AGENTS.md`:

```markdown
## Superpowers System

<EXTREMELY_IMPORTANT>
You have superpowers. Superpowers teach you new skills and capabilities. RIGHT NOW run: `~/.codex/superpowers/.codex/superpowers-codex bootstrap` and follow the instructions it returns.
</EXTREMELY_IMPORTANT>
```

## Reference

- Setup doc: `./INSTALL.md`
- Bootstrap prompt: `./superpowers-bootstrap.md`

