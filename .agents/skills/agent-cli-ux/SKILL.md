---
name: agent-cli-ux
description: >-
  Designs minimal command-line interfaces optimized for scripting and AI agents
  (stable flags, machine-readable output, clear exit codes). Use when adding CLI
  commands to notion-calendar-cli or specifying how agents should invoke tools.
---

# Agent-oriented CLI UX

## Principles

1. **Stable interface:** Prefer long flags (`--json`) and backward-compatible additions; avoid breaking renamed flags without major version.
2. **Machine-readable primary:** Support `--json` or line-based records for parseability; human text as default or via `--pretty`.
3. **Exit codes:** `0` success; `2` usage/validation; `3` auth/config missing; `4` upstream/API error; avoid overloading `1` for everything.
4. **Idempotency:** Document which commands are safe to retry; use dedupe keys when mutating remote state.
5. **Secrets:** Read tokens from env (`NOTION_CALENDAR_TOKEN` pattern) or OS keychain helpers—never require pasting secrets on argv.
6. **Noise:** Progress to stderr; final artifact on stdout when scripting.

## Output shapes

- **One record per line** for lists (tab or `\0` delimited for weird titles).
- **JSON lines** for heterogeneous events if streaming status matters.

## Testing for agents

- Provide `--dry-run` when an operation would mutate data.
- Echo resolved config (`--show-config` redacted) to debug automation environments.
