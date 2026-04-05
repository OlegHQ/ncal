# Notion Calendar — internal HTTP API contract (reverse-engineered)

**Analyzed client:** Notion Calendar web shell `cron-web@1.132.0` (Webpack bundle lineage matches desktop **1.132.0**).
**Method:** Static analysis of shipped JavaScript from `https://calendar.notion.so` — entry bundle + 28 lazy-loaded webpack chunks (~8.5 MB total).
**Coverage:** **85** `/v2` RPC endpoints with per-operation request/response field schemas extracted from minified call sites, Zod validators, and response destructuring patterns.

**Confidence key:** *confirmed* = seen in client source (string literal, Zod schema, or call site); *likely* = one strong indirect cue; *unknown* = not extracted.

---

## 1. Scope and compliance

- This describes the **undocumented** HTTP surface used by the **official** Notion Calendar web/Electron app to talk to **`calendar-api.notion.so`**. It is intended for **interoperability research** (e.g. a Rust CLI) and may violate Notion's terms if used against production accounts without authorization.
- Prefer **[Notion's public REST API](https://developers.notion.com/)** (`api.notion.com`) where it meets your needs; it does **not** mirror this contract.
- **Do not** commit live tokens, cookies, or captured traffic into this repository.

---

## 2. Service topology (hosts)

| Host | Role | Confidence |
|------|------|------------|
| `https://calendar.notion.so` | SPA origin (`WEB_URL`), serves static assets, loads the calendar UI, hosts `config.json` for remote feature/version gating | *confirmed* |
| `https://calendar-api.notion.so` | Primary **JSON API** base (`API_URL`); public root returns HTML contact/marketing page; real APIs are under `/v1/*` and `/v2/*` | *confirmed* |
| `https://api.cron.com` | **Legacy** default in code when `API_URL` is unset (Cron-era); production web build **embeds** `calendar-api.notion.so` | *confirmed* (code), *unknown* (still used in prod clients) |
| `https://www.notion.so` | Linked content, deep links prefixed when path starts with `/` | *confirmed* |
| `https://exp.notion.so/v1/` | Experiments / feature infrastructure (URLs appear in client) | *confirmed* (string only); behavior *unknown* |
| `https://calendar-te.notion.so/2/httpapi` | Appears in bundle (likely analytics/test endpoint fragment) | *likely* telemetry; treat as *unknown* |

Supporting third parties (non-calendar-API but referenced): Amplitude, Statsig, Sentry, Google/Microsoft OAuth endpoints, etc.

---

## 3. Authentication and session

### 3.1 OAuth / linking — browser redirect (`GET`)

The client navigates the user to:

- **Standard:** `https://calendar-api.notion.so/v1/auth?<query>`
- **Google Workspace "EDU" variant:** `https://calendar.notion.so/google-permissions?<query>` (then continues into Google OAuth)

Query parameters (*confirmed* from auth helper in client) commonly include:

- `provider` — e.g. `google`, `notion`, `zoom`, `outlook`, `icloud`
- `client` — client kind / build identifier
- `popup` — boolean string when using popup flow
- `referralCode`, `userId`, `requiredEmail`, `loginHint`, `scopes` (comma-separated)
- `notionDeviceId`, `from`, `nonce`, `workspaceId`, `pathname`
- **Electron:** `referer` set to `WEB_URL` when electron flag is true

After OAuth, the SPA parses the return URL (`state`, `code`, etc.) and continues in-app.

### 3.2 Token exchange / linking — `POST /v1/auth`

```http
POST https://calendar-api.notion.so/v1/auth?<standard-query>
Content-Type: application/json
```

JSON body (*confirmed* call site) includes at least:

- `provider` — string
- `code` — OAuth authorization code (or equivalent)
- Additional fields: `csrf`, `calendarCsrf`, `autolink`, `notionUserId`, `userId`, `spaceId`, `notionDesktopRedirectPageId`, `bypassNotionDev`, `isMobileNative`, `from`, `env`, etc. (Zod-validated; fields depend on provider)

On success, response objects include `user` and sometimes `account` (calendar account), per branching on `icloud` / `outlook` vs others (*confirmed* control flow).

### 3.3 Authenticated calls — bearer access token

```http
Authorization: Bearer <accessToken>
```

`accessToken` is held on `userStore`; the client checks `accessTokenExpiresAt` on the current user and, before requests, may call `refreshNotionSession` (*confirmed*).

### 3.4 Session refresh — `POST /v2/refreshNotionSession`

```http
POST https://calendar-api.notion.so/v2/refreshNotionSession
Content-Type: application/json

{"refreshToken":"..."}
```

Response (*confirmed* Zod schema):
```json
{"accessToken":"...","refreshToken":"...","accessTokenExpiresAt":"..."}
```

Rate limits / cooldown: max ~5 attempts / 30s window, 5-min cooldown if exceeded (*confirmed* in client).

### 3.5 Logout / disconnect Zoom — `DELETE /v1/auth`

Client calls `del("/v1/auth", {accountId})` for at least Zoom disconnect (*confirmed*). Exact query/body shape is *unknown*.

### 3.6 Cookie / CORS behavior

- The API client sets `fetch(..., { credentials: "include" })` in at least one Notion-related code path (*confirmed*).
- Expect **browser** sessions to rely on a combination of **Bearer token** + **first-party cookies** for `notion.so` / API subdomains.

### 3.7 Local credential storage (*confirmed* from filesystem analysis)

The Electron app stores the full authenticated user object (including tokens) in **Chromium LocalStorage** — a LevelDB database at:

```
~/Library/Application Support/Notion Calendar/Local Storage/leveldb/
```

| Key | Value | Notes |
|-----|-------|-------|
| `_https://calendar.notion.so\x00\x01user.auth.currentUser` | Full User JSON (~7.5KB) | Contains `accessToken`, `refreshToken`, `accessTokenExpiresAt`, all `accounts[]` |
| `_https://www.notion.so\x00\x01LRU:KeyValueStore2:current-user-id` | `{id, value, important}` | Notion user ID |
| `_https://identity.notion.so\x00\x01notion_user_id` | string | Notion user ID |

**No encryption, no Keychain** — tokens are stored as plain JSON in LevelDB. The database is locked while Electron is running; a CLI can either:
1. Copy the LevelDB files, remove the `LOCK` file, and read with any LevelDB library
2. Read while the app is closed (no lock contention)

**Token lifecycle:**
- `accessToken` — JWT signed with EdDSA (`kid: "prod-2024-11-07"`), issued by Notion, audience `Calendar`. Contains `encrypted_token_id`, `type: "session"`. Expires in ~5 hours (`exp` claim).
- `refreshToken` — same JWT format, `type: "refresh"`. Expires in ~180 days.
- On expiry, call `/v2/refreshNotionSession` with `refreshToken` to get new tokens.

**CLI auth strategy (`notion-cal auth --from-app`):**
1. Copy LevelDB from app data dir
2. Parse `user.auth.currentUser` key
3. Extract `accessToken` + `refreshToken` + `accessTokenExpiresAt`
4. Check if `accessToken` is expired; if so, call `/v2/refreshNotionSession`
5. Store in CLI's own credential store (e.g. macOS Keychain via `security` CLI)

### 3.8 Full authentication flow (*confirmed* from live traffic + static analysis)

```
┌─────────┐          ┌──────────────────┐          ┌────────────┐
│   CLI   │          │ calendar-api     │          │ notion.so  │
│         │          │ .notion.so       │          │            │
└────┬────┘          └────────┬─────────┘          └──────┬─────┘
     │  POST /v2/getNotionLoginUrl    │                    │
     │  {userId?, referralCode?,      │                    │
     │   forceLogin?}                 │                    │
     │──────────────────────────────>│                    │
     │  {url: "https://notion.so/    │                    │
     │   calendarAuth/?state=..."}   │                    │
     │<──────────────────────────────│                    │
     │                                │                    │
     │  Open browser → url + &state=  │                    │
     │  {client, nonce?, from?}       │                    │
     │───────────────────────────────────────────────────>│
     │                                │                    │
     │              (user enters email + OTP on notion.so) │
     │                                │                    │
     │  Redirect back with preAuthToken                    │
     │<───────────────────────────────────────────────────│
     │                                │                    │
     │  POST /v2/createNotionSession  │                    │
     │  {preAuthToken, context: {     │                    │
     │    from?, notionDeviceId?,     │                    │
     │    referralCode?, workspaceId?,│                    │
     │    collectionViewId?,          │                    │
     │    redirectUri?}}              │                    │
     │──────────────────────────────>│                    │
     │  {user: {id, accessToken,     │                    │
     │   refreshToken,               │                    │
     │   accessTokenExpiresAt, ...}, │                    │
     │   isNewUser: bool}            │                    │
     │<──────────────────────────────│                    │
     │                                │                    │
     │  (subsequent API calls)        │                    │
     │  Authorization: Bearer <token> │                    │
     │──────────────────────────────>│                    │
```

### 3.9 Server logout signal

If a JSON error returns **HTTP 401** and body `code === "invalidToken"` while a token was present, the client emits `userWasLoggedOutByServer` (*confirmed*).

---

## 4. Transport envelope (how `/v2/*` RPCs are called)

### 4.1 URL shape

All JSON API calls target:

`https://calendar-api.notion.so` + **path** (e.g. `/v2/incrementalSync`).

| Query key | Meaning |
|-----------|---------|
| `ver` | App / protocol version | *confirmed* |
| `client` | Client identifier | *confirmed* |
| `tz` | IANA time zone id (primary grid TZ or system default) | *confirmed* |
| `locale` | UI locale | *confirmed* |

### 4.2 HTTP method and body

The generic RPC wrapper does (*confirmed*):

```js
await req.post(url, paramsObject, undefined, options)
```

Each **`/v2/<OperationName>`** is invoked as **POST** with a JSON body encoding the operation's arguments object.

### 4.3 Health / maintenance probing — `GET /v1/status`

The API client performs `get("/v1/status")` when diagnosing connectivity; failures influence a **maintenance mode** UI (*confirmed*).

### 4.4 Remote config — `GET https://calendar.notion.so/config.json?ts=<unixSeconds>`

Bootstrap JSON controlling `minimumWebVersion`, `minimumElectronVersion`, `incrementalSyncInterval`, `maintenanceRetryInterval` (*confirmed*).

---

## 5. Request headers (routine `/v2` traffic)

| Header | Value / notes |
|--------|----------------|
| `Content-Type` | `application/json` |
| `X-Client-Platform` | platform identifier |
| `X-Client-Type` | same family as `client` query |
| `X-Client-OS` | OS identifier |
| `X-TimeZone` | primary TZ |
| `X-Notion-Authenticated` | `"true"` / `"false"` based on Notion auth flag on user |
| `X-Client-Feature-Flags` | optional; serialized experiment gate state |
| `x-realtime-client-id` | optional; from real-time store |
| `Authorization` | `Bearer <accessToken>` when token exists |

---

## 6. Shared object schemas

These schemas are referenced by multiple endpoints. Extracted from Zod validators in the client bundle.

### 6.1 Provider enum

```
"google" | "notion" | "icloud" | "outlook"
```

### 6.2 sendUpdates enum

```
"all" | "none" | "externalOnly"
```

### 6.3 recurringUpdate enum

```
"single" | "all" | "allFollowing"
```

### 6.4 Event object

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | required |
| `accountId` | string | required |
| `calendarId` | string | required |
| `kind` | string? | |
| `sequence` | number? | |
| `anyoneCanAddSelf` | boolean? | |
| `attachments` | `{fileUrl, iconLink, title, mimeType?, fileId?}[]`? | |
| `attendees` | Attendee[]? | see §6.5 |
| `colorId` | string? | nullable |
| `conferenceData` | object? | nullable |
| `created` | string? | ISO 8601 |
| `creator` | `{displayName?, email?, id?, self?}`? | |
| `description` | string? | nullable |
| `end` | `{date?, dateTime?, timeZone?}`? | |
| `endTimeUnspecified` | boolean? | |
| `etag` | string? | |
| `eventType` | enum? | `"default"` \| `"focusTime"` \| `"outOfOffice"` \| `"birthday"` \| `"availability"` \| `"fromGmail"` |
| `extendedProperties` | `{private?: object, shared?: object}`? | |
| `gadget` | object? | |
| `guestsCanInviteOthers` | boolean? | |
| `guestsCanModify` | boolean? | |
| `guestsCanSeeOtherGuests` | boolean? | |
| `hangoutLink` | string? | |
| `htmlLink` | string? | |
| `iCalUID` | string? | |
| `location` | string? | nullable |
| `locked` | boolean? | |
| `organizer` | `{displayName?, email?, id?, self?}`? | |
| `originalStartTime` | `{date?, dateTime?, timeZone?}`? | |
| `privateCopy` | boolean? | |
| `recurrence` | string[]? | RRULE strings |
| `recurringEventId` | string? | |
| `reminders` | `{overrides?: {method: "email"\|"popup", minutes: number}[], useDefault?: boolean}`? | |
| `source` | `{title, url}`? | |
| `start` | `{date?, dateTime?, timeZone?}`? | |
| `status` | enum? | `"confirmed"` \| `"tentative"` \| `"cancelled"` |
| `summary` | string? | event title |
| `transparency` | enum? | `"opaque"` \| `"transparent"` |
| `updated` | string? | ISO 8601 |
| `visibility` | enum? | `"default"` \| `"public"` \| `"private"` \| `"confidential"` |
| `attendeesOmitted` | boolean? | |
| `outOfOfficeProperties` | `{autoDeclineMode?, declineMessage?}`? | |
| `focusTimeProperties` | `{autoDeclineMode?, chatStatus?, declineMessage?}`? | |
| `workingLocationProperties` | `{customLocation?, homeOffice?, officeLocation?, type?}`? | |
| `responseStatus` | enum? | `"needsAction"` \| `"accepted"` \| `"declined"` \| `"tentative"` |
| `movingFromAccountId` | string? | |
| `movingFromCalendarId` | string? | |
| `movingFromProvider` | Provider? | |
| `movingFromEventId` | string? | |
| `movingFromEventType` | string? | |
| `movingFromHoldGroupId` | string? | |
| `notionIcon` | object? | |
| `notionUrl` | string? | |
| `notionTitleHasRichText` | boolean? | |
| `holdGroupId` | string? | |
| `provider` | Provider? | injected by backend |
| `notionPage` | `{properties?: any}`? | |
| `lastSyncedAt` | string? | ISO 8601; client-side sync timestamp (sent in requests) |
| `checkboxPropertyValueStates` | array? | empty array in captures; Notion task checkbox states |
| `date` | null? | explicitly sent as `null` in start/end when clearing all-day |

### 6.5 Attendee object

| Field | Type | Notes |
|-------|------|-------|
| `id` | string? | |
| `additionalGuests` | number? | |
| `comment` | string? | |
| `displayName` | string? | |
| `email` | string? | |
| `optional` | boolean? | nullable |
| `organizer` | boolean? | |
| `resource` | boolean? | |
| `responseStatus` | enum | `"needsAction"` \| `"accepted"` \| `"declined"` \| `"tentative"` |
| `self` | boolean? | |
| `groupMembers` | string[]? | |

### 6.6 autoDeclineMode enum

```
"declineNone" | "declineAllConflictingInvitations" | "declineOnlyNewConflictingInvitations"
```

### 6.7 User object (Zod schema `Vt`)

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | required |
| `accessToken` | string? | |
| `refreshToken` | string? | |
| `accessTokenExpiresAt` | string? | ISO 8601 |
| `createdAt` | string | ISO 8601 |
| `email` | string | |
| `username` | string | |
| `accounts` | Account[] | see §6.8 |
| `displayName` | string | |
| `givenName` | string? | |
| `familyName` | string? | |
| `locale` | string? | |
| `profilePhotoURL` | string? | |
| `status` | enum? | `"active"` \| `"canceled"` \| `"testing"` \| `"banned"` |
| `notionUserId` | string? | linked Notion user ID |

### 6.8 Account object (*confirmed* from live traffic)

| Field | Type | Notes |
|-------|------|-------|
| `id` | string (UUID) | required |
| `createdAt` | string | ISO 8601 |
| `primary` | boolean | whether this is the primary account |
| `email` | string | |
| `displayName` | string | |
| `givenName` | string? | absent on notion/icloud accounts |
| `familyName` | string? | absent on some accounts |
| `profilePhotoURL` | string? | absent on notion/icloud |
| `providerName` | Provider | `"google"`, `"notion"`, `"icloud"` |
| `providerUserId` | string | provider-specific user ID |
| `hostedDomain` | string? | Google Workspace domain; absent for consumer |
| `info` | object | provider-polymorphic; see below |
| `scopes` | string[] | OAuth scopes granted |
| `capabilities` | Capabilities | see §6.9 |

**Account.info** — empty `{}` for google/icloud; for `providerName="notion"`:

| Field | Type | Notes |
|-------|------|-------|
| `notionUserId` | string (UUID) | |
| `notionWorkspaceId` | string (UUID) | |
| `notionUserDisplayName` | string | |
| `notionWorkspaceSettings.isAiEnabled` | boolean | |
| `notionWorkspaceSettings.isTranscriptionEnabled` | boolean | |
| `notionWorkspaceSettings.meetingNotesSettings` | object | |

### 6.9 Capabilities object (*confirmed* from live traffic)

| Field | Type | Notes |
|-------|------|-------|
| `readCalendars` | boolean | |
| `readCalendarResources` | boolean | false for non-Google |
| `readEvents` | boolean | |
| `syncEvents` | boolean | false for notion accounts |
| `searchEvents` | boolean | false for notion/icloud |
| `readOrgEvents` | boolean | true only for Google Workspace |
| `writeEvents` | boolean | |
| `blockEvents` | boolean | false for notion |
| `supportHtmlDescriptions` | boolean | false for notion/icloud |
| `readContacts` | boolean | false for notion |
| `readDirectory` | boolean | false for notion/icloud |
| `takeMeetingNotes` | boolean | |

### 6.10 Calendar object (*confirmed* from live traffic)

Union shape; fields vary by provider. Google uses `kind: "calendar#calendarListEntry"`, iCloud uses `kind: "calendar#calendarList"`.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | calendar ID (email for Google primary, UUID for iCloud) |
| `kind` | string | `"calendar#calendarListEntry"` (Google) or `"calendar#calendarList"` (iCloud) |
| `etag` | string | version tag |
| `summary` | string | display name |
| `description` | string? | |
| `timeZone` | string? | Google only; IANA TZ |
| `dataOwner` | string? | Google only; present on shared calendars |
| `colorId` | string? | Google only; references getColors palette |
| `backgroundColor` | string | hex color (Google: 6-char `#RRGGBB`, iCloud: 8-char `#RRGGBBAA`) |
| `foregroundColor` | string? | Google only |
| `selected` | boolean? | Google only |
| `accessRole` | string | `"owner"` \| `"reader"` |
| `defaultReminders` | `{method: string, minutes: number}[]` | can be empty |
| `notificationSettings` | object? | Google primary calendars only |
| `notificationSettings.notifications` | `{type: string, method: string}[]` | types: `"eventCreation"`, `"eventChange"`, `"eventCancellation"`, `"eventResponse"` |
| `primary` | boolean? | Google only |
| `hidden` | boolean? | Google only |
| `conferenceProperties` | object? | Google only |
| `conferenceProperties.allowedConferenceSolutionTypes` | string[] | e.g. `["hangoutsMeet"]` |
| `selfAttendee.email` | string | |
| `selfAttendee.displayName` | string? | present on iCloud |
| `provider` | Provider | injected by backend |
| `accountId` | string (UUID) | injected by backend |

### 6.11 ConferenceData object (*confirmed* from live traffic)

| Field | Type | Notes |
|-------|------|-------|
| `entryPoints` | EntryPoint[] | |
| `conferenceSolution` | ConferenceSolution | |
| `conferenceId` | string | e.g. `"cxs-ivfi-emj"` |

**EntryPoint:**

| Field | Type | Notes |
|-------|------|-------|
| `entryPointType` | string | `"video"` \| `"phone"` \| `"more"` |
| `uri` | string | URL or tel: URI |
| `label` | string? | human-readable |
| `pin` | string? | phone/more only |
| `regionCode` | string? | phone only, e.g. `"US"` |

**ConferenceSolution:**

| Field | Type | Notes |
|-------|------|-------|
| `key.type` | string | `"hangoutsMeet"` |
| `name` | string | `"Google Meet"` |
| `iconUri` | string | |

### 6.12 HoldGroup object (*confirmed* from live traffic)

| Field | Type | Notes |
|-------|------|-------|
| `id` | string (UUID) | |
| `createdAt` | string | ISO 8601 |
| `shortId` | string | short identifier |
| `alias` | string | URL-safe slug for scheduling link |
| `type` | string | e.g. `"recurring"` |
| `status` | string | e.g. `"activeBookable"` |
| `userPrimaryTimeZone` | string | IANA TZ |
| `timeZone` | string | IANA TZ |
| `duration` | number | minutes |
| `timeRanges` | TimeRange[] | see below |
| `title` | string | |
| `description` | string? | |
| `conferencingProviderName` | string \| null | |
| `conferencingAccountId` | string \| null | |
| `googleAccountId` | string (UUID) | destination account (misleading name; can be iCloud) |
| `googleCalendarId` | string | destination calendar ID |
| `conflictFreeResources` | `{provider, accountId, calendarId}[]` | calendars checked for conflicts |
| `minLeadTime` | number | min minutes before booking |
| `maxLeadTime` | number | max minutes in advance |
| `hasBeenBooked` | boolean | |
| `schedulingLink` | string | public booking URL |
| `data` | object | extra data; typically `{}` |
| `senderDisplayName` | string? | public API only; hold owner's name |
| `senderEmail` | string? | public API only; hold owner's email |
| `customFields` | array? | public API only; custom booking form fields |

**TimeRange:**

| Field | Type | Notes |
|-------|------|-------|
| `id` | string (UUID) | |
| `startDate` | string | ISO 8601 with offset |
| `endDate` | string | ISO 8601 with offset |
| `recurrence` | string[] | RRULE strings |

### 6.13 NotionWorkspace object (*confirmed* from live traffic)

| Field | Type | Notes |
|-------|------|-------|
| `spaceID` | string (UUID) | Notion workspace ID |
| `name` | string | |
| `icon` | string \| null | workspace icon URL |
| `subscriptionTier` | string | `"free"`, `"plus"`, `"business"`, `"enterprise"` |
| `hasAiAddon` | boolean | |
| `hasUnlimitedAI` | boolean | |
| `isAiDisabled` | boolean | |

### 6.14 NotionUser object (*confirmed* from live traffic)

| Field | Type | Notes |
|-------|------|-------|
| `id` | string (UUID) | Notion user ID |
| `name` | string | |
| `email` | string | |
| `calendarUser` | User? | full User object (§6.7) when linked |

### 6.15 Contact object (*confirmed* from live traffic)

Full Google People API-derived shape:

| Field | Type | Notes |
|-------|------|-------|
| `resourceName` | string | `"people/..."` |
| `etag` | string | |
| `id` | string | same as resourceName |
| `email` | string | primary email (convenience) |
| `displayName` | string | convenience field |
| `alternateEmails` | string[] | all non-primary emails |
| `type` | string | `"directory"` |
| `accountId` | string (UUID) | injected by backend |
| `names` | `{displayName, familyName, givenName, displayNameLastFirst, unstructuredName, metadata}[]` | |
| `photos` | `{url: string, default?: boolean, metadata}[]` | |
| `emailAddresses` | `{value: string, type?: string, formattedType?: string, metadata}[]` | |
| `phoneNumbers` | `{value: string, canonicalForm: string, type: string, formattedType: string, metadata}[]`? | |
| `organizations` | `{department?: string, metadata}[]`? | |
| `memberships` | `{domainMembership: {inViewerDomain: boolean}, metadata}[]` | |
| `groupData` | `{isGroup: boolean}`? | from static analysis |

### 6.16 UserPreferences object (*confirmed* from live traffic)

| Field | Type | Notes |
|-------|------|-------|
| `etag` | string | optimistic concurrency |
| `locale` | string | |
| `preferredLocale` | string | |
| `primaryTimeZone` | string | IANA TZ |
| `additionalTimeZones` | string[] | |
| `deviceTypes.phone` | PhonePrefs | mobile-specific settings |
| `calendarListState` | CalendarListState | calendar visibility per-device |
| `recentParticipants` | `{email, displayName}[]` | |
| `dismissedNotices` | object | notice ID → ISO 8601 timestamp |
| `readNewsFeedItemIds` | string[] | |
| `autoShareMeetingNotes` | boolean | |
| `dismissedWelcomeChecklist` | boolean | |
| `completedWelcomeChecklistIds` | string[] | |
| `lastStartedMobileAppTimestamp` | number | Unix ms |
| `lastStartedDesktopAppTimestamp` | number | Unix ms |
| `autoAddConferencingPromptViewed` | boolean | |
| `defaultHoldConflictFreeResourcesMapping` | object | `"{accountId}_{calendarId}"` → CalendarRef[] |
| `format24HourTime` | boolean? | |
| `defaultNotionWorkspaceId` | string? | |
| `recentNotionPageLocations` | object? | |

**CalendarListState:**

| Field | Type | Notes |
|-------|------|-------|
| `accounts` | array | |
| `accounts[].id` | string (UUID) | account ID |
| `accounts[].calendars` | `{id: string, selected: boolean, active?: boolean}[]` | |
| `accounts[].collections` | array | always empty in captures |

### 6.17 NotionPage object (*confirmed* from live traffic)

| Field | Type | Notes |
|-------|------|-------|
| `id` | string (UUID) | Notion page/block ID |
| `name` | string | page title |
| `url` | string | full Notion URL |
| `object` | string | `"page"` |
| `path` | string[] | breadcrumb trail |
| `collectionIds` | string[] | database IDs (empty for pages) |

### 6.18 extendedProperties.shared conventions (*confirmed* from live traffic)

Events carry Notion Calendar metadata in `extendedProperties.shared`:

| Key | Value | Notes |
|-----|-------|-------|
| `cron.meetingNote` | string \| null | meeting note link (null = none) |
| `cron.holdGroup.id` | string (UUID) | hold group ID (on booked events) |
| `cron.holdGroup.shortId` | string | hold short ID |
| `cron.holdGroup.slotId` | string (UUID) | specific time slot ID |
| `cron.holdGroup.recipientName` | string | booker's name |
| `n.attchwsid.<blockId>` | string (UUID) | Notion workspace ID for attached page |

### 6.19 Notion page location (discriminated union on `type`)

| Variant | Fields |
|---------|--------|
| `{type: "private"}` | create in user's private space |
| `{type: "page", id: string}` | create under an existing page |
| `{type: "database", id: string, collectionViewId?: string}` | create in a database |

### 6.18 Hold location (discriminated union on `type`)

| Variant | Fields |
|---------|--------|
| `{type: "conferencing"}` | video conferencing |
| `{type: "inboundPhoneCall"}` | |
| `{type: "outboundPhoneCall", recipientPhoneNumber: string}` | |
| `{type: "inPerson"}` | |

---

## 7. `/v2` RPC catalog — full field schemas (85 operations)

Each entry: **POST** `https://calendar-api.notion.so<path>` with JSON body.

---

### 7.1 Events

#### `/v2/getEvents`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `queries` | array | required; one per calendar |
| `queries[].provider` | Provider | default `"google"` |
| `queries[].accountId` | string | |
| `queries[].calendarId` | string | |
| `queries[].timeMin` | number? | epoch ms |
| `queries[].timeMax` | number? | epoch ms |
| `queries[].singleEvents` | boolean? | |
| `queries[].showDeleted` | boolean? | |
| `queries[].query` | string? | search text |
| `queries[].userTimeZone` | string? | IANA TZ |
| `queries[].limit` | number? | |
| `queries[].participants` | boolean? | |
| `queries[].orderBy` | string? | |
| `queries[].includeUnscheduled` | boolean? | |
| `queries[].unfurlGroups` | boolean? | |
| `queries[].pageToken` | string? | pagination |
| `queries[].syncToken` | string? | incremental sync |
| `queries[].metadata` | `{caller?: string, websocketId?: string}`? | |

**Response:** array of per-calendar results:
| Field | Type | Notes |
|-------|------|-------|
| `[].accountId` | string | |
| `[].calendarId` | string | |
| `[].events` | Event[] | |
| `[].syncToken` | string? | for next sync |
| `[].pageToken` | string? | for next page |
| *or error:* | | |
| `[].provider` | string | |
| `[].errorMessage` | string | |

---

#### `/v2/getEvent`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `query.provider` | Provider | |
| `query.accountId` | string | |
| `query.calendarId` | string | |
| `query.eventId` | string | |
| `query.userTimeZone` | string? | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `event` | Event | |
| `accountId` | string | |
| `calendarId` | string | |

---

#### `/v2/createEvent`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `mutation.provider` | Provider | |
| `mutation.accountId` | string | |
| `mutation.calendarId` | string | |
| `mutation.eventData` | Event | `startTime`/`endTime` stripped |
| `mutation.sendUpdates` | sendUpdates? | |
| `mutation.order` | `"FIRST"`? | |

**Response:** Event object (with `accountId`, `calendarId` merged) or error variant.

---

#### `/v2/updateEvents`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `mutations` | array | batch |
| `mutations[].provider` | Provider | |
| `mutations[].accountId` | string | |
| `mutations[].eventId` | string | |
| `mutations[].calendarId` | string | |
| `mutations[].eventData` | Event | `startTime`/`endTime` stripped |
| `mutations[].userTimeZone` | string? | |
| `mutations[].sendUpdates` | sendUpdates? | |

**Response:** array of `{event: Event}` or error variants.

---

#### `/v2/deleteEvents`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `mutations` | array | batch |
| `mutations[].provider` | Provider | |
| `mutations[].accountId` | string | |
| `mutations[].calendarId` | string | |
| `mutations[].eventId` | string | |
| `mutations[].sendUpdates` | sendUpdates? | |

**Response:** array of success/error variants.

**Notes:** The Notion Calendar UI does **not** call `deleteEvents` for single-event deletion. Instead, it uses `updateEvents` with `status: "cancelled"` + `responseStatus: "declined"` for **both** Google and iCloud providers. (*confirmed* from live traffic — Google and iCloud tested). The `deleteEvents` endpoint may be reserved for batch purge or programmatic deletion scenarios.

---

#### `/v2/getEventInbox`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `calendarRequests` | array | |
| `calendarRequests[].provider` | Provider | |
| `calendarRequests[].accountId` | string | |
| `calendarRequests[].calendarId` | string | |
| `timeMin` | number? | epoch ms |
| `timeMax` | number? | epoch ms |

**Response:** opaque (returned directly).

---

#### `/v2/exportEvents`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `events` | array | |
| `events[].id` | string | |
| `events[].start` | `{date?, dateTime?, timeZone?}`? | |
| `events[].end` | `{date?, dateTime?, timeZone?}`? | |
| `events[].summary` | string? | |
| `events[].description` | string? | |
| `events[].location` | string? | |
| `events[].created` | string? | |
| `events[].recurrence` | string[]? | |
| `events[].attendees` | `{email, responseStatus, optional?}[]`? | |
| `events[].organizer` | `{email?}`? | |

**Response:** ICS file data.

---

### 7.2 Calendars

#### `/v2/getCalendars`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `queries` | array | |
| `queries[].provider` | Provider | |
| `queries[].accountId` | string | |
| `queries[].query` | string? | search text |
| `queries[].haveUsed` | boolean? | |
| `queries[].suggested` | boolean? | |

**Response:** array of `{calendars: Calendar[]}` or error variants.

---

#### `/v2/getCalendarLists`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `queries` | array | |
| `queries[].provider` | Provider | |
| `queries[].accountId` | string | |
| `queries[].metadata` | `{caller?: string, websocketId?: string}`? | |

**Response:** array of `{calendars: Calendar[]}` or error variants (including `invalid_grant` for disconnected accounts).

---

#### `/v2/insertCalendarList`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `mutation.provider` | Provider | |
| `mutation.accountId` | string | |
| `mutation.calendarId` | string | |
| `mutation.calendarData` | partial Calendar? | |

**Response:** `{calendar: Calendar}` or error (including `maximumNotionDatabaseCountReached`).

---

#### `/v2/deleteCalendarList`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `mutation.provider` | Provider | |
| `mutation.accountId` | string | |
| `mutation.calendarId` | string | |

**Response:** `{success: boolean}` or error with `{accountId, calendarId, error}`.

---

#### `/v2/updateCalendars`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `mutations` | array | batch |
| `mutations[].provider` | Provider | |
| `mutations[].accountId` | string | |
| `mutations[].calendarId` | string | |
| `mutations[].calendarData` | partial Calendar | |
| `mutations[].colorRgbFormat` | boolean? | |
| `mutations[].variant` | enum | `"calendarListUpdate"` \| `"calendarUpdate"` |

**Response:** array of `{accountId, calendarId, calendar?}` or error variants.

---

#### `/v2/getCalendarFreeBusy`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `timeMin` | number | epoch ms |
| `timeMax` | number | epoch ms |
| `calendars` | array | |
| `calendars[].provider` | string | e.g. `"google"` |
| `calendars[].accountId` | string | |
| `calendars[].calendarId` | string | typically resourceEmail |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `calendars` | array | |
| `calendars[].id` | string | calendarId |
| `calendars[].errors` | `{message: string}[]`? | e.g. `"notFound"` |
| `calendars[].schedule` | `{start, end}[]` | busy periods |

---

#### `/v2/getCalendarResources`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accounts` | string[] | Google account IDs only |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `buildingResources` | array | |
| `buildingResources[].accountId` | string | |
| `buildingResources[].buildingId` | string | |
| `buildingResources[].kind` | string? | |
| `buildingResources[].etags` | string? | |
| `buildingResources[].buildingName` | string? | |
| `buildingResources[].description` | string? | |
| `buildingResources[].coordinates` | object? | |
| `buildingResources[].floorNames` | string[]? | |
| `buildingResources[].address` | string? | |
| `calendarResources` | array | |
| `calendarResources[].accountId` | string | |
| `calendarResources[].generatedResourceName` | string? | |
| `calendarResources[].kind` | string? | |
| `calendarResources[].resourceCategory` | string? | |
| `calendarResources[].resourceEmail` | string | |
| `calendarResources[].resourceId` | string | |
| `calendarResources[].resourceName` | string? | |
| `calendarResources[].buildingId` | string? | |
| `calendarResources[].capacity` | number? | |
| `calendarResources[].etags` | string? | |
| `calendarResources[].featureInstances` | object? | |
| `calendarResources[].floorName` | string? | |
| `calendarResources[].floorSection` | string? | |
| `calendarResources[].resourceDescription` | string? | |
| `calendarResources[].resourceType` | string? | |
| `calendarResources[].userVisibleDescription` | string? | |

**Notes:** Google-only. Only called for accounts where `canReadCalendarResources` is true.

---

#### `/v2/getColors`

**Request:** `{}` (empty)

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `calendar` | Color[]? | |
| `event` | Color[]? | |

Each Color: `{id: number, kind: "calendar"|"event", background: string, foreground: string}` (hex, uppercase).

**Notes:** Google-only (guarded by `canUserReadGoogleCalendars`).

---

### 7.3 Sync

#### `/v2/incrementalSync`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `syncTokens` | array | |
| `syncTokens[].provider` | string? | |
| `syncTokens[].accountId` | string | |
| `syncTokens[].resourceId` | string | |
| `syncTokens[].token` | string | |
| `metadata` | `{caller?: string}`? | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `calendars` | Calendar[] | |
| `events` | Event[] | |
| `syncTokens` | SyncToken[] | |
| `errors` | Error[]? | |

**Notes:** Called periodically (default interval 60s). Only for providers supporting sync: `"google"`, `"icloud"`, `"outlook"`.

---

#### `/v2/incrementalAuth`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `providerName` | string | literal `"google"` |
| `requiredEmail` | string | account email |
| `code` | string | OAuth authorization code |
| `redirectURL` | string | OAuth redirect URL |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `user` | User | full user object |
| `account` | Account | with `.email` field |

**Notes:** Used to add incremental Google scopes.

---

### 7.4 Contacts & Groups

#### `/v2/getContacts`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `includeAlternateEmails` | boolean? | always `true` |

**Response:** Contact[] (array directly).

---

#### `/v2/getGroupMemberAttendees`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `eventId` | string | |
| `groupMembers` | string[] | email addresses |
| `accountId` | string? | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `attendees` | `Record<string, Attendee>` | email → attendee |

---

#### `/v2/getGroupMembers`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accountId` | string | |
| `emails` | string[] | |

**Response:** `Record<string, {isGroup: boolean}>` — email → group info.

---

### 7.5 Holds (scheduling links)

#### `/v2/getHolds`

**Request:** `{}` (empty)

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `holds` | HoldGroup[] | parsed via `parseHoldGroups` |

**Notes:** Guarded by `canUserReadHolds()` permission check.

---

#### `/v2/getHold`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `username` | string | |
| `alias` | string | holdShortId from URL |
| `timeMin` | number | epoch ms |
| `timeMax` | number | epoch ms |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `hold` | HoldGroup | with `timeRanges`, `duration` |

**Notes:** Used on public "Meet" scheduling page. Response is a **public subset** of the full HoldGroup — includes `senderDisplayName`, `senderEmail`, `customFields` but omits `id`, `createdAt`, `googleAccountId`, `googleCalendarId`, `conflictFreeResources`, `hasBeenBooked`, `schedulingLink`, `data`. Time ranges are **pre-expanded** (no `recurrence` field, conflict-free slots only).

---

#### `/v2/createHold`

**Request:** all fields from `holdGroup.toJSON()`:
| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `alias` | string | |
| `type` | string | |
| `status` | string | |
| `userPrimaryTimeZone` | string | |
| `timeZone` | string | |
| `duration` | number? | |
| `timeRanges` | array | |
| `pendingTimeRangeMoveStates` | object? | |
| `title` | string | |
| `description` | string | |
| `conferencingProviderName` | string | |
| `conferencingAccountId` | string | |
| `phone` | string? | |
| `location` | string? | |
| `googleAccountId` | string | |
| `googleCalendarId` | string | |
| `conflictFreeResources` | object? | |
| `minLeadTime` | number? | |
| `maxLeadTime` | number? | |
| `expirationDate` | number? | epoch ms |

**Response:** `{success: boolean}` or error.

---

#### `/v2/updateHold`

**Request:** same fields as `createHold`.

**Response:** `{success: boolean}`.

---

#### `/v2/deleteHold`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `holdId` | string | |

**Response:** `{success: boolean}`.

---

#### `/v2/getHoldEvent`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `eventId` | string | |
| `holdShortId` | string | |
| `holdSlotId` | string | |
| `timeMin` | number? | epoch ms |
| `timeMax` | number? | epoch ms |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `hold` | HoldGroup | same as getHold |
| `formerTimeSlot` | object | for rescheduling |

---

#### `/v2/createHoldEvent`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `holdShortId` | string | |
| `name` | string? | optional |
| `email` | string | |
| `startDate` | string | ISO 8601 |
| `endDate` | string | ISO 8601 |
| `location` | HoldLocation? | see §6.10 |
| `customFieldResponses` | array? | |

**Response:** `{success: boolean}`.

---

#### `/v2/updateHoldEvent`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `eventId` | string | |
| `holdShortId` | string | |
| `holdSlotId` | string | |
| `updateType` | enum | `"reschedule"` \| `"cancel"` |
| `updateProperties` | object | see below |

`updateProperties` for reschedule: `{newStartDate: string, newEndDate: string, updateReason?: string}`
`updateProperties` for cancel: `{updateReason?: string}`

**Response:** `{success: boolean}`.

---

#### `/v2/getHoldAliasAvailable`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `alias` | string | |

**Response:** `{available: boolean}`.

---

### 7.6 Synchronized calendars (event blocking)

#### `/v2/getSynchronizedCalendars`

**Request:** `{}` (empty)

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `synchronizedCalendars` | array | |

---

#### `/v2/createSynchronizedCalendar`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `sourceAccountId` | string | |
| `sourceCalendarId` | string | |
| `accountId` | string | |
| `calendarId` | string | |
| `syncEnabled` | boolean | |
| `visibility` | string? | optional |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `synchronizedCalendar` | object | the created record |

---

#### `/v2/deleteSynchronizedCalendar`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `synchronizedCalendarId` | string | |

**Response:** `{success: boolean}`.

---

#### `/v2/createSynchronizedEvent`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `event` | object | full event JSON via `toJSON()` |
| `sourceEvent.id` | string | |
| `sourceEvent.accountId` | string | |
| `sourceEvent.calendarId` | string | |
| `sourceEvent.recurringEventId` | string? | |
| `recurringType` | string? | e.g. `"all"` |
| `status` | string | |
| `visibility` | string | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `synchronizedCalendar` | object | |
| `synchronizedEvent` | object | |

---

#### `/v2/deleteSynchronizedEvent`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `sourceEventId` | string | |
| `sourceRecurringEventId` | string? | |
| `sourceCalendarId` | string | |
| `sourceAccountId` | string | |
| `calendarId` | string | |
| `accountId` | string | |
| `recurringUpdate` | string | e.g. `"all"` |

**Response:** `{success: boolean}`.

---

### 7.7 User & account management

#### `/v2/getUser`

**Request:** `{}` (empty)

**Response:** User object (see §6.7).

---

#### `/v2/updateUser`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `username` | string? | lowercased, nullish |
| `displayName` | string? | nullish |
| `profilePhotoURL` | string? | nullish |

**Response:** `{user: User}`.

---

#### `/v2/deleteUser`

**Request:** (no params)

**Response:** `{success: boolean}`.

**Notes:** Deletes the entire Notion Calendar account. Triggers `logOut()`.

---

#### `/v2/getUserPreferences`

**Request:** `{}` (empty)

**Response:** the full user preferences JSON blob, including:
| Field | Type | Notes |
|-------|------|-------|
| `etag` | string | optimistic concurrency |
| `preferredLocale` | string? | |
| `format24HourTime` | boolean? | |
| `lastStartedDesktopAppTimestamp` | number? | |
| `completedWelcomeChecklistIds` | string[]? | |
| `primaryTimeZone` | string? | IANA TZ |
| `defaultNotionWorkspaceId` | string? | |
| `recentNotionPageLocations` | object? | |

---

#### `/v2/updateUserPreferences`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `userPreferences` | object | changed preferences via `toDirtiedJSON()` |
| `userPreferenceUpdateSources` | string[] | e.g. `"user-interaction"`, `"side-effect"`, `"mobile-foreground"` |
| `pendingChanges` | object | debug diff details |
| `lastLiveSnapshotETag` | string | etag from last known snapshot |

**Response:** `{userPreferences: object}` — server's updated preferences.

**Notes:** Sends header `X-Force-Etag-Check: "true"`. On HTTP 412 (etag mismatch), error body contains `{userPreferences}` for reconciliation. Retries: etag-mismatch (50), network-error (∞), failed-to-update (50), local-changes-in-flight (50), bad-gateway (100).

---

#### `/v2/getUserSettings`

**Request:** `{}` (empty)

**Response (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `ai` | object? | |
| `ai.notionAccountId` | string? | |
| `ai.calendarPreferences` | string? | max 1000 chars |

---

#### `/v2/updateUserSettings`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `settings.ai` | object? | nullish |
| `settings.ai.notionAccountId` | string \| null? | null to delete |
| `settings.ai.calendarPreferences` | string \| null? | max 1000, null to delete |

**Response:** `{settings: object}` — full updated settings.

**Notes:** Retries up to 3 times with backoff.

---

#### `/v2/getUsernameAvailable`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `q` | string | username to check |

**Response:** `{available: boolean}`.

---

#### `/v2/updatePrimaryAccount`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accountId` | string | |

**Response:** `{user: User}`.

**Notes:** Only called when account is not already primary.

---

#### `/v2/getPrimaryAccountAvailable`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accounts` | string[] | calendar account IDs |

**Response:** opaque object (availability map keyed by account IDs).

---

#### `/v2/removeAccount`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accountId` | string | |

**Response:** success/error.

**Notes:** Used for calendar accounts, Notion workspaces, and custom conferencing accounts (different toast messages per type).

---

#### `/v2/identifyDevice`

**Request:** pass-through object (exact fields from caller, not visible in extracted context).

**Response:** opaque.

---

#### `/v2/createCalendarAccount`

**Request:** discriminated union on `provider`:

Google variant:
| Field | Type | Notes |
|-------|------|-------|
| `provider` | `"google"` | |
| `code` | string | OAuth authorization code |
| `redirectURL` | string? | |

iCloud variant:
| Field | Type | Notes |
|-------|------|-------|
| `provider` | `"icloud"` | |
| `username` | string? | |
| `email` | string? | |
| `appSpecificPassword` | string | |

Also a "required email" variant: `{providerName: "google", requiredEmail: string, code: string, redirectURL?: string}`.

**Response:** Account object (*likely*).

---

#### `/v2/updateCalendarAccountSettings`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accountId` | string | |
| `info` | object | key-value pairs to merge into account settings |

**Response:** success/error.

**Notes:** Optimistic update with rollback on failure.

---

### 7.8 Conferencing

#### `/v2/createConferencingAccount`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `displayName` | string | |
| `customValue` | string | URL template |

**Response:** success/error.

---

#### `/v2/updateConferencingAccount`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accountId` | string | |
| `displayName` | string | |
| `customValue` | string | URL template |

**Response:** success/error.

---

#### `/v2/createConferencing`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `providerName` | string | e.g. `"zoom"` |
| `startTime` | string | ISO 8601 |
| `endTime` | string | ISO 8601 |
| `topic` | string | meeting title |
| `timeZone` | string | IANA TZ |
| `isRecurring` | boolean | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `meeting.id` | string | coerced from number |
| `meeting.joinUrl` | string | |
| `meeting.password` | string | |

**Notes:** Currently only used for Zoom.

---

#### `/v2/updateConferencing`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `...zoomMeeting` | object | spread (includes `meetingId`) |
| `startTime` | string | ISO 8601 |
| `provider` | string | literal `"zoom"` |

**Response:** on failure: `{status: "failure", response: {code: number, message: string}}`. Error code `3001` = invalid meeting ID.

---

#### `/v2/deleteConferencing`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `meetingId` | string | |
| `scheduleForReminder` | boolean | hardcoded `false` |

**Response:** opaque data.

---

### 7.9 Notion integration

#### `/v2/getNotionPages`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `notionWorkspaceId` | string | |
| `pageIds` | string[] | array of Notion page IDs |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `pages` | NotionPage[] | each has at least `id` |

---

#### `/v2/getRecentNotionPages`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `notionWorkspaceId` | string | |
| `bypassCache` | boolean? | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `pages` | NotionPage[] | defaults to `[]` |

**Notes:** Client-side 30-second TTL cache.

---

#### `/v2/getNotionSearch`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `notionWorkspaceId` | string? | |
| `query` | string | search text |
| `limit` | integer? | 1–100 |
| `source` | string? | |
| `recordType` | enum? | `"block"` \| `"collection"` |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `results` | NotionSearchResult[] | defaults to `[]` |

---

#### `/v2/createNotionPage`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `notionWorkspaceId` | string? | |
| `title` | string | page title |
| `location` | NotionPageLocation | see §6.9 |

**Response:** opaque.

---

#### `/v2/upsertNotionMeetingNote`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `event` | object | calendar event data |
| `location` | NotionPageLocation? | see §6.9 |
| `share` | boolean | share meeting notes |
| `notionWorkspaceId` | string | |
| `from` | string | origin context |
| `platform` | enum | `"react-native"` \| `"electron"` \| `"browser"` |

**Response:** opaque.

---

#### `/v2/createNotionTaskDatabase`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `template` | `"custom"`? | only value in enum |
| `notionWorkspaceId` | string? | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `view.id` | string | used as calendar ID |

---

#### `/v2/canEditNotionBlocks`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `blockIds` | string[] | |
| `notionWorkspaceId` | string? | |

**Response:** opaque (likely `{[blockId]: boolean}`).

---

#### `/v2/getNotionUsersWithoutAccessToBlock`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `emailsToCheck` | string[] | |
| `blockId` | string | |
| `notionWorkspaceId` | string | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `usersWithoutAccess` | object[] | |
| `availableInviteRoles` | string[] | |

---

#### `/v2/grantNotionUsersAccessToBlock`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `userIds` | string[] | Notion user IDs |
| `blockId` | string | |
| `roleToGrant` | enum | `"editor"` \| `"read_and_write"` \| `"comment_only"` \| `"reader"` |
| `notionWorkspaceId` | string | |

**Response:** opaque.

---

#### `/v2/getNotionSessionUsers`

**Request:** `{}` (empty)

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `users` | NotionUser[] | each has `id`, `name`, `profilePhoto`, `calendarUser?` |

---

#### `/v2/getNotionWorkspaces`

**Request:** `{}` (empty)

**Response:** array (stored as `new Set(data)`) — workspace IDs or workspace objects.

**Notes:** Only called if `currentUser.isAuthedWithNotion`.

---

#### `/v2/getNotionLoginUrl`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `userId` | string? | |
| `referralCode` | string? | |
| `forceLogin` | enum? | `"true"` \| `"false"` |

**Response:** `{url: string}`.

**Notes:** Client appends `state` query param (with `client`, optional `nonce`, optional `from`).

---

#### `/v2/getNotionLogoutUrl`

**Request:** `{}` (empty)

**Response:** `{url: string}`.

**Notes:** The returned URL is then called with a separate `POST` including `Authorization` header and `{userId}` body.

---

#### `/v2/getNotionAddAccountUrl`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `redirectUrl` | string | |

**Response:** `{url: string}`.

---

#### `/v2/connectNotionWorkspace`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `workspaceId` | string | |

**Response:** opaque.

---

#### `/v2/updateNotionWorkspaceSettings`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `workspaceId` | string | |
| `defaultMeetingNotesLocation` | `{blockId: string, collectionId?: string}` \| null? | nullish |
| `autoAddMeetingNotes` | boolean? | nullish |
| `disableAutoShareMeetingNotes` | boolean? | nullish |

**Response:** triggers `userStore.synchronize()`.

---

#### `/v2/createNotionSession`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `preAuthToken` | string | |
| `context.from` | string? | nullish |
| `context.notionDeviceId` | string? | nullish |
| `context.referralCode` | string? | nullish |
| `context.workspaceId` | string? | nullish |
| `context.collectionViewId` | string? | nullish |
| `context.redirectUri` | string? | nullish |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `user` | User | full User object (§6.7) |
| `isNewUser` | boolean | |

**Notes:** Retried up to 3 times.

---

#### `/v2/refreshNotionSession`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `refreshToken` | string | |

**Response (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `accessToken` | string | |
| `refreshToken` | string | |
| `accessTokenExpiresAt` | string | |

**Notes:** Cooldown: max 5 attempts / 30s, 5-min cooldown if exceeded.

---

### 7.10 Notion AI transcripts

#### `/v2/getNotionInferenceTranscript`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accountId` | string | Notion account ID |
| `threadId` | string | conversation thread ID |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `title` | string? | |
| `steps` | TranscriptStep[] | Zod-validated |

---

#### `/v2/getNotionInferenceTranscriptsForUser`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accountId` | string | |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `threads` | `{id, title, createdTime, lastEditedTime}[]` | |
| `nextCursor` | string? | pagination |
| `hasMore` | boolean | |

---

#### `/v2/runNotionInferenceTranscript`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `transcript` | TranscriptStep[] | |
| `traceId` | string | UUID |
| `accountId` | string | |
| `generateTitle` | `true` | hardcoded |
| `threadId` | string | |
| `createThread` | boolean | true if first message |
| `threadParentPointer` | `{table: "space", id: string, spaceId: string}`? | only when `createThread` |
| `isPartialTranscript` | boolean | inverse of `createThread` |
| `saveAllThreadOperations` | `true` | hardcoded |
| `confirmToolStepIds` | string[]? | |
| `rejectToolStepIds` | string[]? | |

**Response:** NDJSON stream (`Accept: application/x-ndjson`). Each line is a Zod-validated TranscriptStep.

**Notes:** Uses raw `fetch()` with streaming, NOT the standard JSON RPC wrapper.

---

#### `/v2/stopNotionInferenceTranscript`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `accountId` | string | |
| `threadId` | string | |
| `inferenceId` | string | the `traceId` from `runNotionInferenceTranscript` |

**Response:** fire-and-forget (no `await`).

---

### 7.11 File upload

#### `/v2/getUploadFileURL`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `contentLength` | number | file size in bytes |
| `contentType` | string | MIME type |
| `name` | string | file name |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `signedPutURL` | string | pre-signed S3 URL |
| `url` | string | final file URL after upload |
| `putHeaders` | `{name: string, value: string}[]`? | extra headers for PUT |

**Notes:** Client then PUTs file content to `signedPutURL` with `Content-Type` and any `putHeaders`.

---

#### `/v2/deleteFile`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `fileURL` | string | URL-encoded via `encodeURIComponent()` |

**Response:** opaque.

---

#### `/v2/getTranscriptionRecordAncestorChainForEvent`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `eventUid` | string | unique event identifier |
| `knownVersions` | `{[spaceId]: {[blockId]: version}}`? | undefined if no blocks exist |

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `changed` | `{value: {id, space_id, ...}}[]` | blocks that changed |
| `removed` | string[] | block IDs that were removed |

**Notes:** Incremental sync for transcription blocks. Polls on 10-second interval while event is in progress. Only when AI MN Workflow V2 feature is enabled.

---

### 7.12 Referrals & feedback

#### `/v2/createReferral`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `recipientEmail` | string | |
| `referrer` | string | literal `"userReferral"` |

**Response:** opaque.

---

#### `/v2/getReferralSuggestions`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `emails` | string[] | up to 30, sorted by domain priority |

**Response:** array (checked via `Array.isArray`).

---

#### `/v2/sendFeedback`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `textBody` | string | feedback text |
| `attachments` | `{name: string, contentType: string, content: string}[]`? | only if non-empty |
| `uploads` | `{name: string, url: string}[]`? | only if non-empty |

**Response:** success/error.

**Notes:** Uses separate `APIRequest` instance with optional custom `User-Agent` header.

---

### 7.13 Telemetry & diagnostics

#### `/v2/logToSplunk`

**Request:**
| Field | Type | Notes |
|-------|------|-------|
| `logs` | object[] | array wrapping a single log entry |

**Response:** opaque.

**Notes:** Only called when `currentUser.isNotionInternal` is true.

---

#### `/v2/incrementMetrics`

**Request:** pass-through object (exact fields from caller).

**Response:** opaque.

**Notes:** Gated by `"client_metric_collection"` launch gate OR `currentUser.isNotionInternal`.

---

#### `/v2/getDecagonToken`

**Request:** (no params)

**Response:**
| Field | Type | Notes |
|-------|------|-------|
| `decagonSignature` | string | |

**Notes:** Used for Decagon AI support widget initialization.

---

#### `/v2/meetingNotificationHeartbeat`

**Request (Zod-validated):**
| Field | Type | Notes |
|-------|------|-------|
| `platform` | enum | `"mac"` \| `"windows"` |
| `isAlive` | boolean | default `true` |
| `botIds` | string[]? | optional, min length 1 each |
| `botId` | string? | legacy, use `botIds` |

**Response:** opaque.

---

## 8. `/v1` surface (non-RPC layout)

| Method | Path | Role |
|--------|------|------|
| `GET` | `/v1/auth` | OAuth / linking entry redirect with query string |
| `POST` | `/v1/auth` | Exchange / complete auth with JSON body |
| `GET` | `/v1/status` | Reachability / maintenance signaling |
| `DELETE` | `/v1/auth` | Account disconnect (Zoom path observed) |

---

## 9. Embedded client configuration (`API_URL` module)

Production constants (*confirmed*):

- `API_URL`: `https://calendar-api.notion.so`
- `WEB_URL`: `https://calendar.notion.so`
- `VERSION`: `1.132.0`
- Public third-party keys (Amplitude, Google API client key, Statsig client key, Sentry DSN) — **do not treat as secrets**; they are browser-exposed by design.

---

## 10. Error model

Errors include (*partially confirmed*):
- HTTP status code
- `code` — e.g. `"invalidToken"`, `"maximumNotionDatabaseCountReached"`
- `message` — human-readable
- `messageId` — localization key
- `intlMessage` — localized message
- `statusText` — HTTP status text

HTTP 401 with `code === "invalidToken"` triggers client logout.
HTTP 412 on `updateUserPreferences` indicates etag mismatch; body contains `{userPreferences}` for reconciliation.

---

## 11. Gaps and next steps for a CLI implementer

1. ~~**Calendar object schema**~~ — *resolved* via live traffic capture (§6.10).
2. ~~**HoldGroup object schema**~~ — *resolved* via live traffic capture (§6.12).
3. ~~**Account object schema**~~ — *resolved* via live traffic capture (§6.8).
4. **TranscriptStep schema** — the Zod schema `Gn` validates streamed NDJSON steps; exact step variants need enumeration.
5. **Rate limits** — only client-side refresh throttling is visible; server quotas are *unknown*.
6. **Version drift** — paths and headers can change any release; pin the app/bundle version you traced.
7. ~~**NotionPage schema**~~ — *resolved* via live traffic (§6.17). **NotionSearchResult** still not captured.
8. **iCloud-specific event fields** — live capture showed mostly Google events; iCloud event shape differences need a session with iCloud calendar activity.

---

## 12. Change log

| Date | Source | Notes |
|------|--------|-------|
| 2026-04-05 | `cron-web@1.132.0` JS | Initial spec from static RE; 82 `/v2` paths, auth + transport |
| 2026-04-05 | `cron-web@1.132.0` JS (entry + 28 lazy chunks) | Full field-level schemas for 85 `/v2` endpoints; 3 new endpoints discovered (`deleteFile`, `getUploadFileURL`, `getTranscriptionRecordAncestorChainForEvent`); Event, Attendee, User, Contact Zod schemas extracted |
| 2026-04-05 | Live traffic capture via mitmproxy (v1.132.0) — read path | Account, Calendar, HoldGroup, ConferenceData, NotionWorkspace, NotionUser, UserPreferences, Contact schemas confirmed and enriched from ground-truth responses; Capabilities object documented; provider-specific field differences noted |
| 2026-04-05 | Live traffic capture via mitmproxy (v1.132.0) — write path | createEvent, updateEvents (move, edit, decline/cancel), getHold (public), createHoldEvent, getRecentNotionPages, getNotionUsersWithoutAccessToBlock confirmed. NotionPage schema extracted. extendedProperties.shared conventions documented. Public vs private HoldGroup response differences noted. iCloud event CRUD fully confirmed. |
| 2026-04-05 | Filesystem analysis — credential storage | Tokens stored as plain JSON in Chromium LocalStorage LevelDB at `~/Library/Application Support/Notion Calendar/Local Storage/leveldb/` under key `user.auth.currentUser`. No encryption/Keychain. Full auth flow documented with sequence diagram. CLI credential hijack strategy defined. |
