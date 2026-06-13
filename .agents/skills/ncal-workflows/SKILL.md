---
name: ncal-workflows
description: >-
  Run practical Notion Calendar workflows with the local `ncal` CLI. Use this
  whenever the user asks to authenticate `ncal`, use Notion Calendar from SSH or
  headless Linux, inspect accounts/calendars, list/search/create/update/delete
  events, sync calendar state, work with scheduling holds, or script calendar
  automation. Prefer this over research docs unless the user is reverse
  engineering Notion Calendar or changing the CLI implementation.
---

# ncal Workflows

Use `ncal` as the stable interface. Keep context small: run
`ncal <command> --help` for exact flags, and avoid loading research docs unless
the task is about internals.

## Rules

- Use `--json` for scripts and agents; parse stdout only.
- Treat stderr hints and `auto:` lines as human guidance.
- Do not paste tokens into chat, commits, logs, or shell history when avoidable.
- Discover IDs first, then pass explicit `--account`, `--calendar`, and
  `--provider` when ambiguity is possible.
- Read before mutating, mutate once, then verify with `get`, `list --refresh`,
  or the matching list command.
- `events delete` soft-cancels by default. Use `--hard` only when the user
  explicitly asks for permanent deletion.
- Use `--config <PATH>` or `NCAL_CONFIG` for isolated automation.

Exit codes: `0` success, `2` usage/validation, `3` auth/credentials, `4`
API/network/I/O.

## Discovery

Start most workflows with:

```sh
ncal auth status --json
ncal whoami --json
ncal accounts list --json
ncal calendars list --json
```

If a command auto-resolution fails or the user mentions a specific calendar,
rerun with explicit selectors:

```sh
--account <ACCOUNT_ID_OR_EMAIL> --calendar <CALENDAR_ID> --provider <PROVIDER>
```

## Auth

Desktop app import:

```sh
ncal auth from-app
ncal auth status
```

SSH/headless browser import when Notion finishes at
`https://calendar.notion.so/`: copy browser LocalStorage key
`user.auth.currentUser`, then import it.

```sh
ncal auth import-user-json --file user.auth.currentUser.json
pbpaste | ncal auth import-user-json --stdin
xclip -selection clipboard -o | ncal auth import-user-json --stdin
ncal auth import-user-json --raw-json '<JSON>'
```

Prefer `--file` or `--stdin`; `--raw-json` can leak through shell history. After
import:

```sh
ncal auth status
ncal whoami
```

`Source: file` is expected on headless Linux when no OS keychain is available.
Do not diagnose it as a macOS Keychain problem. Refresh or clear credentials:

```sh
ncal auth refresh
ncal auth logout
```

## Events

List/search events:

```sh
ncal events list --json
ncal events list --refresh --json
ncal events list --from-time 2026-06-13T00:00:00Z --to-time 2026-06-14T00:00:00Z --json
ncal events list --query "standup" --limit 10 --json
```

Useful list flags: `--all`, `--include-deleted`, `--limit <N>`, `--refresh`,
`--account <ACCOUNT>`, `--calendar <CALENDAR>`, `--provider <PROVIDER>`.

Inspect, create, update, delete:

```sh
ncal events get <EVENT_ID> --json

ncal events create \
  --summary "Project sync" \
  --start 2026-06-13T16:00:00Z \
  --end 2026-06-13T16:30:00Z \
  --account <ACCOUNT> \
  --calendar <CALENDAR> \
  --json

ncal events update <EVENT_ID> --summary "New title" --json
ncal events update <EVENT_ID> --start 2026-06-13T17:00:00Z --end 2026-06-13T17:30:00Z --json

ncal events delete <EVENT_ID> --json
ncal events delete <EVENT_ID> --hard --json
```

Create/update times are RFC3339. List range times accept RFC3339 or epoch
milliseconds. Optional create/update fields include `--description` and
`--location`.

## Accounts And Calendars

```sh
ncal accounts list --json
ncal accounts set-primary <ACCOUNT_ID>
ncal accounts remove <ACCOUNT_ID>

ncal calendars list --json
ncal calendars list --account <ACCOUNT_ID> --json
ncal calendars colors --json
```

Only set primary or remove an account after confirming the target ID.

## Sync And Support Data

```sh
ncal sync --json
ncal sync --tokens-file /path/to/sync-tokens.json --json
ncal contacts --json
ncal preferences get --json
```

Use `events list --refresh --json` when the immediate goal is fresh event data.
Use `sync` when the goal is to update incremental sync state.

## Scheduling Holds

```sh
ncal holds list --json
ncal holds get <HOLD_ID> --json
ncal holds check-alias <ALIAS> --json
ncal holds delete <HOLD_ID> --json

ncal holds create \
  --title "Intro call" \
  --alias "intro-call" \
  --duration 30 \
  --timezone "America/New_York" \
  --account <ACCOUNT_ID> \
  --calendar <CALENDAR_ID> \
  --json
```

Optional create flags: `--description <TEXT>`, `--hold-type <TYPE>`,
`--min-lead-time <MINUTES>`, `--max-lead-time <MINUTES>`.

## Troubleshooting

- Auth error: `ncal auth status`, then `ncal auth refresh`, then re-import only
  if refresh fails.
- Empty/stale events: `ncal events list --refresh --json`.
- Ambiguous account/calendar: discover with `accounts list --json` and
  `calendars list --json`, then pass explicit IDs.
- Script sees extra text: confirm `--json` and parse stdout, not stderr.
- Flag uncertainty: run `ncal <command> --help`, not broad docs.
