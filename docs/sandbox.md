# Sandbox (V0.5)

Epris V0.5 treats the active project folder (`projectRoot`) as the execution boundary for AI/provider runs.

## Goals

- Provider processes run with `cwd = projectRoot`.
- After each provider run, Epris audits the workspace and **rolls back** if it detects writes to forbidden locations.

## Allowed / ignored paths

These paths are **not** considered a security violation during the audit:

- Allowed provider-edit locations:
  - `src/**`
  - `public/**`
  - `.gemini/**` (provider config/skills)
  - `.opencode/**` (provider config/skills)
- App-internal paths (ignored by audit):
  - `.epris/**` (snapshots/backups/temp)
  - `logs/**`

## Forbidden paths

Any detected writes under these are treated as a security violation:

- `node_modules/**`
- `.git/**`
- Any other top-level file/folder outside the allow/ignore lists.

## Rollback behavior

Epris takes a pre-flight backup (`.epris/backups/pre_flight`) before running the provider.

If the audit detects a violation:

1) The violation is logged to `logs/security.log`
2) The workspace is restored from `pre_flight`
3) The UI shows an error message that changes were reverted

## Limitations (best-effort)

`node_modules` can be extremely large; V0.5’s node_modules audit is best-effort to avoid blocking the UI for long periods.

