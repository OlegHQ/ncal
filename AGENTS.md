# Agent guide: Notion Calendar CLI / reverse engineering

This repository exists to **document and understand** how Notion Calendar (desktop shell + web app) reaches backend services, and to ship a **small Rust CLI** with minimal UX aimed at **agent-driven automation** (scripts and AI agents calling stable commands). The CLI should call the **Notion Calendar / Notion HTTP API surface** we can characterize from evidence—**not** by repackaging Electron—preferring documented Notion APIs where they cover needed operations and filling gaps only with deliberately scoped, stability-labeled internal calls discovered here.

## Charter

- **CLI implementation:** **Rust** binary (crate layout TBD), talking to network APIs with machine-friendly output; design guidance lives in `.agents/skills/agent-cli-ux/`.
- **Primary RE target (local install):** `/Applications/Notion Calendar.app`
- **Immediate objective:** Map client architecture (Electron + packaged assets + loaded web origin), surface candidate endpoints, auth/session flows, and sync patterns—**without** guessing credentials or bypassing protections.
- **Longer objective:** Implement a thin CLI that wraps only what we can support safely (prefer public APIs and documented flows when available); treat any unpublished endpoints as **optional** and version-fragile.

Agents should prefer **evidence** (filesystem layout, strings, extracted `app.asar` sources, observed network) over speculation. When inferring behavior, label confidence: *confirmed / likely / unknown*.

## Compliance and scope

- Reverse engineering for **interoperability** (understanding wire formats to build a compatible client you control) differs from **circumventing security**, violating ToS, or attacking infrastructure. Stay within applicable law and Notion’s terms.
- Do **not** commit secrets, session tokens, cookies, or personal calendar data. Redact artifacts before committing.
- Treat unpublished internals as **unstable**: they can change every release (`CFBundleShortVersionString` in `Info.plist`).

## What we already know about the shipped app

From the installed bundle (snapshot on this machine):

- **Bundle ID:** `com.cron.electron` (Cron was Notion Calendar’s earlier product name.)
- **URL scheme:** `cron` (see `CFBundleURLTypes` in `Info.plist`).
- **Runtime:** **Electron** — `Contents/Resources/app.asar` (~57MB) is the packaged application JavaScript/assets.
- **Helpers:** Standard Electron helper apps under `Contents/Frameworks/`.

Implication for agents: **most protocol clues live inside `app.asar` and extracted JS**, not only in the main Mach-O binary.

## Repository layout

| Path | Purpose |
|------|---------|
| `AGENTS.md` | This file: goals, constraints, workflow |
| `.agents/skills/notion-calendar-reverse-engineering/` | Skill: how to RE this app efficiently |
| `.agents/skills/agent-cli-ux/` | Skill: minimal CLI patterns for scripts and agents |
| `scripts/` | Fast, composable shell helpers (bundle info, `strings`, `otool`, ASAR listing) |
| `research/` | Human/agent notes; see `research/findings.md` |
| `research/artifacts/` | Gitignored: extracted ASAR trees, large dumps (optional) |

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

## Next milestones (for humans/agents)

1. Pin the **shipping Electron** major and the **ASAR layout** (main entry, preload, chunks); scaffold the **Rust** crate(s) and dependency choices (HTTP client, auth storage) once API boundaries are clearer.
2. Extract high-signal **base URLs** and **WebSocket** patterns from packaged JS **and** from the `https://calendar.notion.so` web client (likely primary API surface).
3. Cross-check against **published Notion / Cron public docs** where they exist; prefer official APIs for the CLI surface.
4. Define CLI commands that map 1:1 to stable, testable operations.
