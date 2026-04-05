---
name: notion-calendar-reverse-engineering
description: >-
  Reverse engineers the Notion Calendar macOS Electron app bundle to discover
  hosts, auth patterns, and sync behavior. Use when analyzing
  /Applications/Notion Calendar.app, app.asar, cron:// URL scheme, com.cron.electron,
  or building the notion-calendar-cli; run repo scripts under scripts/ before ad-hoc tooling.
---

# Notion Calendar reverse engineering

## Target

- Default path: `/Applications/Notion Calendar.app` (override with `NOTION_CALENDAR_APP`).
- Treat the app as **Electron**: packaged logic lives in `Contents/Resources/app.asar`.
- Bundle ID: `com.cron.electron`; custom scheme: `cron`.

## Fast path (use repo scripts)

From repository root:

1. `./scripts/bundle_snapshot.sh` — confirm version, ASAR presence, executable name.
2. `./scripts/asar_list.sh` — see packaged file tree (fast, no full extract).
3. When content search is required: extract ASAR to `research/artifacts/app-asar/` (gitignored), then use `rg` on URLs, `api.`, `graphql`, `oauth`, `wss`.

Full extract example (agent may run in a shell):

```bash
mkdir -p research/artifacts/app-asar
npx --yes @electron/asar extract "$NOTION_CALENDAR_APP/Contents/Resources/app.asar" research/artifacts/app-asar
```

## Where answers usually hide

| Signal | Where to look |
|--------|----------------|
| API hosts, paths | Extracted JS/JSON inside ASAR, `*.env*`, build-time constants |
| Auth | Keychain usage names in strings; OAuth redirect URLs; `cron://` handlers |
| Transport | `fetch`, `axios`, `WebSocket`, gRPC-web patterns in JS |
| Local persistence | `app.getPath`, SQLite paths, IndexedDB mentions |

## macOS-native clues (secondary)

Run `./scripts/strings_app.sh` and `./scripts/otool_deps.sh` for the native shell. Electron apps often show **Chromium + Node** dylibs; most calendar API logic remains in JS.

## Documentation hygiene

- Log discoveries in `research/findings.md` with **app version** from `Info.plist`.
- Mark confidence: *confirmed* (seen in capture or code), *likely* (single strong hint), *unknown*.
- Never paste live tokens into the repo.

## Skill-boundary

This skill covers **local static analysis and structured discovery**. For dynamic analysis (proxy TLS, instrumentation), prefer separate task planning and explicit user consent—those methods can capture sensitive data.
