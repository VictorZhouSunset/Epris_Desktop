# Security Baseline (V0.5)

V0.5 focuses on “safe to hand to friends” guardrails rather than a full OS-level sandbox.

## Frontend permissions (Tauri capabilities)

- No `shell:allow-all`
- No `process:allow-all`
- Updater uses `updater:default`

See: `apps/epris-tauri/src-tauri/capabilities/default.json`

## Provider execution boundary

- Providers are started with `cwd = projectRoot`.
- Writes are audited after provider runs; violations are rolled back (see `docs/sandbox.md`).

## Sensitive config

- Updater signing **private key** is never committed.
- CI uses GitHub Secrets for signing (see `docs/release-secrets.md`).

