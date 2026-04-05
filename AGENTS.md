# Agent guide: Notion Calendar CLI / reverse engineering

This repository exists to **document and understand** how Notion Calendar (desktop shell + web app) reaches backend services, and to ship a **small Rust CLI** with minimal UX aimed at **agent-driven automation** (scripts and AI agents calling stable commands). The CLI should call the **Notion Calendar / Notion HTTP API surface** we can characterize from evidence—**not** by repackaging Electron—preferring documented Notion APIs where they cover needed operations and filling gaps only with deliberately scoped, stability-labeled internal calls discovered here.

## Charter

- **CLI implementation:** **Rust** binary (`ncal-api` library + `ncal-cli` binary), talking to network APIs with dual human/machine output; design guidance lives in `.agents/skills/agent-cli-ux/`.
- **Primary RE target (local install):** `/Applications/Notion Calendar.app`
- **Immediate objective:** Map client architecture (Electron + packaged assets + loaded web origin), surface candidate endpoints, auth/session flows, and sync patterns—**without** guessing credentials or bypassing protections.
- **Longer objective:** Implement a thin CLI that wraps only what we can support safely (prefer public APIs and documented flows when available); treat any unpublished endpoints as **optional** and version-fragile.

Agents should prefer **evidence** (filesystem layout, strings, extracted `app.asar` sources, observed network) over speculation. When inferring behavior, label confidence: *confirmed / likely / unknown*.

## Compliance and scope

- Reverse engineering for **interoperability** (understanding wire formats to build a compatible client you control) differs from **circumventing security**, violating ToS, or attacking infrastructure. Stay within applicable law and Notion’s terms.
- Do **not** commit secrets, session tokens, cookies, or personal calendar data. Redact artifacts before committing.
- Treat unpublished internals as **unstable**: they can change every release (`CFBundleShortVersionString` in `Info.plist`).

## What we know about the shipped app (v1.132.0)

From the installed bundle and full RE (static + live traffic):

- **Bundle ID:** `com.cron.electron` (Cron was Notion Calendar’s earlier product name.)
- **URL scheme:** `cron` (see `CFBundleURLTypes` in `Info.plist`).
- **Runtime:** **Electron 33.2.0** (Chromium 130) — `Contents/Resources/app.asar` (~57MB).
- **Web SPA:** Entry bundle + 28 lazy-loaded webpack chunks (~8.5MB total) from `calendar.notion.so/assets/`.
- **API base:** `https://calendar-api.notion.so` — 85 POST `/v2/*` JSON RPC endpoints + `/v1/auth`, `/v1/status`.
- **Auth:** OAuth via Notion → `preAuthToken` → `createNotionSession` → JWT access/refresh tokens. Tokens stored **unencrypted** in Chromium LocalStorage LevelDB.
- **Providers:** Google, iCloud, Outlook, Notion (as calendar sources).

## Key reference documents

| Document | What it covers |
|----------|---------------|
| `research/notion-calendar-api-contract-spec.md` | **Complete API spec**: 85 RPC endpoints with per-field request/response schemas, 20 shared object schemas (Event, Calendar, Account, User, HoldGroup, etc.), auth flow with sequence diagram, credential storage, transport envelope |
| `research/findings.md` | Chronological RE log with app version, artifacts used, confidence levels |
| `.agents/skills/agent-cli-ux/SKILL.md` | CLI design principles for agent-driven automation |
| `.agents/skills/notion-calendar-reverse-engineering/SKILL.md` | How to RE the Electron app efficiently |
| `IMPLEMENTATION_PLAN.md` | Rust CLI implementation plan with crate structure, feature map, and milestones |

## Repository layout

| Path | Purpose |
|------|---------|
| `AGENTS.md` | This file: goals, constraints, workflow |
| `.agents/skills/notion-calendar-reverse-engineering/` | Skill: how to RE this app efficiently |
| `.agents/skills/agent-cli-ux/` | Skill: minimal CLI patterns for scripts and agents |
| `scripts/` | Fast, composable shell helpers (bundle info, `strings`, `otool`, ASAR listing) |
| `research/` | Human/agent notes; see `research/findings.md` |
| `research/notion-calendar-api-contract-spec.md` | Full API contract (85 RPCs, all schemas) |
| `research/artifacts/` | Gitignored: extracted ASAR trees, web bundles, mitmproxy captures |

## Environment conventions

- `NOTION_CALENDAR_APP` — override app path; default in scripts is `/Applications/Notion Calendar.app`.

## Recommended workflow

1. Read `.agents/skills/notion-calendar-reverse-engineering/SKILL.md` when doing RE work.
2. Run `scripts/bundle_snapshot.sh` for a quick structured dump (paths, plist summary, ASAR presence).
3. Use `scripts/asar_list.sh` to enumerate packaged files; extract to `research/artifacts/app-asar/` only when you need to search file contents.
4. Record **versioned** notes in `research/findings.md` (include app version from `Info.plist`).

## Findings log format

Append sections like:

```markdown
## YYYY-MM-DD — v1.x.x (build from Info.plist)

### Artifacts
- (e.g. ASAR paths, scripts run)

### Endpoints / hosts
- `https://…` — context — confidence: confirmed|likely|unknown

### Auth / session
- (cookies, tokens, headers observed or inferred)

### Open questions
- …
```

## Scripts

All scripts are POSIX-friendly bash, run from repo root or any cwd (they resolve paths relative to repo when needed).

| Script | Purpose |
|--------|---------|
| `scripts/bundle_snapshot.sh` | Plist summary, bundle tree hints, ASAR path check |
| `scripts/strings_app.sh` | `strings` on main binary and key frameworks (cap output) |
| `scripts/otool_deps.sh` | Dynamic libraries / rpaths for main executable |
| `scripts/codesign_info.sh` | `codesign` display and entitlements XML if present |
| `scripts/asar_list.sh` | List files inside `app.asar` via `npx @electron/asar` |
| `scripts/asar_extract.sh` | Extract `app.asar` to `research/artifacts/app-asar` (or path arg) |

## API surface summary

**85 RPC endpoints** documented with full field-level schemas. Key groups:

| Group | Endpoints | CLI priority |
|-------|-----------|-------------|
| Events | getEvents, getEvent, createEvent, updateEvents, deleteEvents, exportEvents, getEventInbox | **P0** — core CRUD |
| Calendars | getCalendarLists, getCalendars, insertCalendarList, deleteCalendarList, updateCalendars, getColors | **P0** — discovery |
| Sync | incrementalSync | **P0** — efficient polling |
| Auth | createNotionSession, refreshNotionSession, getNotionLoginUrl, getUser | **P0** — required |
| Holds | getHolds, getHold, createHold, updateHold, deleteHold, createHoldEvent, updateHoldEvent, getHoldEvent, getHoldAliasAvailable | **P1** — scheduling |
| User/Prefs | getUserPreferences, updateUserPreferences, getUserSettings, updateUserSettings, updateUser | **P1** |
| Contacts | getContacts, getGroupMembers, getGroupMemberAttendees | **P2** |
| Notion | getNotionPages, getNotionSearch, createNotionPage, upsertNotionMeetingNote, and 12 more | **P2** |
| Conferencing | createConferencing, updateConferencing, deleteConferencing, + accounts | **P2** |
| Telemetry | logToSplunk, incrementMetrics, identifyDevice, meetingNotificationHeartbeat | **skip** |
| AI Transcripts | runNotionInferenceTranscript, getNotionInferenceTranscript, + 2 more | **P3** |

## Auth model

- **OAuth flow:** `getNotionLoginUrl` → browser → OTP on notion.so → redirect with `preAuthToken` → `createNotionSession` → JWT tokens
- **Token storage (desktop app):** Plain JSON in Chromium LocalStorage LevelDB at `~/Library/Application Support/Notion Calendar/Local Storage/leveldb/` — key `user.auth.currentUser`
- **Token refresh:** `/v2/refreshNotionSession` with `refreshToken` → new access/refresh pair; access token ~5h TTL, refresh ~180d
- **CLI strategy:** `auth --from-app` reads tokens from the desktop app's LevelDB; `auth login` does the full OAuth flow

See `research/notion-calendar-api-contract-spec.md` §3 for full auth protocol documentation.

## CLI output conventions

The CLI is designed for **both humans and agents**. Rules for all command output:

### Dual-mode output

- **Default (no flag):** Human-friendly tables, key-value pairs, plain text. Errors and hints to stderr, data to stdout.
- **`--json`:** Machine-readable JSON (compact single-line for agents). Errors also JSON to stderr with `{error, code, hint}` structure. No tables, no decorative text.

### Auto-resolution

Commands that need `--account` and `--calendar` **auto-resolve** them:
- If only one account exists → use it (log to stderr: `auto: using account ...`).
- If multiple → fail with a copy-pasteable list of `--account <ID>` flags.
- For calendars → prefer the `primary` calendar, else sole calendar, else fail with list.

Config file (`defaults.account`, `defaults.calendar`) overrides auto-resolution.

### Hints (HATEOAS for CLIs)

After every successful command, print **actionable next-step hints** to stderr:
```
hint: ncal events list      — list upcoming events
      ncal events get <ID>  — show event details
```
Hints are suppressed in `--json` mode. Agents can parse them or ignore them (they're on stderr).

### Error reporting

Every error includes:
1. **What failed** — operation and resource, not internal details.
2. **Why** — specific reason (file path, HTTP status, field name).
3. **What to do** — actionable hint with a command to try.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Usage / validation error |
| 3 | Auth / credentials error |
| 4 | API / network / I/O error |

### Adding new commands

When adding a new command:
1. Add a `/// description` doc comment on the enum variant (shows in `--help`).
2. Add arg-level `/// description` for non-obvious flags.
3. Use `print_<type>(cli, &data)` from `output.rs` — it handles JSON vs table routing.
4. Add `hint(&[...])` after the output for next-step suggestions (skip if `cli.json`).
5. For commands needing account/calendar, use `resolve_context()` from `events.rs` or the pattern in `calendars.rs`.

## CLI commands (implemented)

| Command | Description | Auto-resolve |
|---------|-------------|-------------|
| `ncal auth login` | OAuth sign-in flow | — |
| `ncal auth from-app` | Import tokens from desktop app | — |
| `ncal auth status` | Show credential source and expiry | — |
| `ncal auth refresh` | Force token refresh | — |
| `ncal auth logout` | Remove stored credentials | — |
| `ncal whoami` | Show current user | — |
| `ncal accounts list` | List connected accounts (table) | — |
| `ncal accounts set-primary <ID>` | Set primary account | — |
| `ncal accounts remove <ID>` | Disconnect account | — |
| `ncal calendars list` | List all calendars (table) | accounts auto |
| `ncal calendars colors` | Color palette (JSON) | — |
| `ncal events list` | List events (table) | account + calendar |
| `ncal events get <ID>` | Event detail (key-value) | account + calendar |
| `ncal events create` | Create event | account + calendar |
| `ncal events update <ID>` | Update event fields | account + calendar |
| `ncal events delete <ID>` | Soft-cancel (or `--hard` delete) | account + calendar |
| `ncal sync` | Incremental sync | — |
| `ncal contacts` | List contacts (table) | — |
| `ncal preferences get` | User preferences (JSON) | — |

## Build and install

```sh
make build     # release build (LTO, stripped)
make install   # install to ~/.local/bin
make lint      # clippy -D warnings
make test      # cargo test
```

## Next milestones (for humans/agents)

1. ~~Pin the shipping Electron major and ASAR layout~~ — *done* (v1.132.0, Electron 33.2.0).
2. ~~Extract base URLs and endpoint patterns~~ — *done* (85 RPCs fully documented).
3. ~~Cross-check against published Notion docs~~ — *done* (Notion public API is separate; calendar API is internal).
4. ~~Scaffold Rust crate~~ — *done* (workspace with ncal-api + ncal-cli).
5. ~~Implement P0 commands~~ — *done* (auth, events CRUD, calendar listing, sync, contacts).
6. **Implement P1 commands** — holds/scheduling, user preferences update.
7. **Publish and iterate** — CI, integration tests against a test account.
