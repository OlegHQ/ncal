# Findings log

<!-- Append dated sections as you discover behavior. Include CFBundleShortVersionString from the analyzed bundle. -->

## 2026-04-05 — v1.132.0

### Environment
- macOS; app `/Applications/Notion Calendar.app`; `git` repo initialized at repo root; ASAR extracted to `research/artifacts/app-asar/` (gitignored).

### Artifacts
- `scripts/bundle_snapshot.sh` — confirmed bundle id `com.cron.electron`, version **1.132.0**, main binary `Notion Calendar`, `app.asar` ~57MB.
- `scripts/asar_list.sh` — layout includes `build/main/main.js`, `build/main/config.json`, `build/preload/preload-bundle.js`.
- `scripts/asar_extract.sh` — full tree under `research/artifacts/app-asar/`.

### Hosts / endpoints
- `https://calendar.notion.so` — referenced as primary web/calendar origin in packaged main bundle — confidence: **confirmed**
- `https://www.notion.so`, `https://notion.so` — general Notion web — confidence: **confirmed**
- `https://dev.notion.so` — dev host string present — confidence: **confirmed** (purpose: unknown)
- `https://calendar-desktop-release.notion-static.com` — Sparkle/generic updater base from `app-update.yml` — confidence: **confirmed**
- `https://accounts.google.com`, `https://login.microsoftonline.com` — OAuth-related surfaces in strings — confidence: **likely**

### Auth / session
- Custom URL schemes in `build/main/main.js` include `notion:`, `notiondev:`, `notionlocal:`, `notionmail:`, `notionmaildev:`, `notionmaillocal:` — confidence: **confirmed** (deep-link / auth handoff pattern; full flow unknown)

### Notes
- `build/main/config.json` includes `incrementalSyncInterval: 30` (seconds, likely) and minimum web/Electron version gates — confidence: **confirmed** for file contents; behavioral meaning **likely**.
- Most calendar API traffic is expected from the **embedded web app** (`calendar.notion.so`), not from the thin Electron `main` shell; next RE pass should target network patterns in that origin (or captured traffic with consent), not only `main.js`.

## Template

### YYYY-MM-DD — v0.0.0

#### Environment
- macOS / app path / git revision

#### Artifacts
- Scripts or extracts used

#### Hosts / endpoints
- URLs — notes — confidence: confirmed | likely | unknown

#### Auth / session
-

#### Notes
-
