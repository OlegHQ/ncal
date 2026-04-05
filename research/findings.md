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
- **API contract (static RE, pass 1):** see `research/notion-calendar-api-contract-spec.md` — documents `calendar-api.notion.so` auth (`/v1/auth`, bearer token, `refreshNotionSession`), headers, POST JSON RPC for **82** `/v2/*` operations, `GET /v1/status`, web bundle `cron-web@1.132.0`.
- **API contract (static RE, pass 2 — full field schemas):** Fetched web SPA entry bundle (`cron-f93efa5c11a98b9e9e62.js`, 591KB) + 28 lazy-loaded webpack chunks (~8.5MB total) from `calendar.notion.so/assets/`. Extracted per-endpoint request/response field schemas for all **85** `/v2/*` RPCs (3 new: `deleteFile`, `getUploadFileURL`, `getTranscriptionRecordAncestorChainForEvent`). Recovered Zod validator schemas for Event, Attendee, User, Contact, Hold, and multiple request types. Full spec updated in `research/notion-calendar-api-contract-spec.md`.
- **API contract (live traffic, pass 3 — ground truth):** mitmproxy capture of desktop app v1.132.0 startup + sync flow. Confirmed and enriched: Account object (with Capabilities, provider-polymorphic `info`), Calendar object (Google vs iCloud field differences), HoldGroup (with TimeRange, conflictFreeResources), ConferenceData (EntryPoint, ConferenceSolution), NotionWorkspace, NotionUser, UserPreferences (with CalendarListState, device prefs), Contact (full Google People API shape). Captures stored in `research/artifacts/captures/json/` (gitignored, tokens redacted).

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
