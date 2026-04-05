# Implementation Plan: notion-calendar-cli

**Target:** Rust CLI binary for agent-driven automation of Notion Calendar.
**API surface:** 85 `/v2` JSON RPC endpoints on `calendar-api.notion.so` (see `research/notion-calendar-api-contract-spec.md`).
**Design guide:** `.agents/skills/agent-cli-ux/SKILL.md`

---

## 1. Crate layout

```
notion-calendar-cli/
├── Cargo.toml              # workspace root
├── crates/
│   ├── ncal-api/           # API client library (no CLI deps)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── client.rs       # HTTP client, transport envelope, auth header injection
│   │   │   ├── auth.rs         # Token management, refresh logic, credential extraction
│   │   │   ├── types/          # Shared API types (serde models)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── event.rs        # Event, Attendee, DateTimeOrDate, ConferenceData
│   │   │   │   ├── calendar.rs     # Calendar, Color, CalendarResource
│   │   │   │   ├── user.rs         # User, Account, Capabilities, UserPreferences
│   │   │   │   ├── hold.rs         # HoldGroup, TimeRange, HoldLocation
│   │   │   │   ├── contact.rs      # Contact, ContactName, ContactEmail
│   │   │   │   ├── notion.rs       # NotionPage, NotionWorkspace, NotionUser
│   │   │   │   ├── sync.rs         # SyncToken, IncrementalSyncResponse
│   │   │   │   └── common.rs       # Provider enum, sendUpdates, recurringUpdate, error model
│   │   │   ├── endpoints/      # One module per RPC group
│   │   │   │   ├── mod.rs
│   │   │   │   ├── events.rs       # getEvents, getEvent, createEvent, updateEvents, deleteEvents, exportEvents, getEventInbox
│   │   │   │   ├── calendars.rs    # getCalendarLists, getCalendars, insertCalendarList, deleteCalendarList, updateCalendars, getCalendarFreeBusy, getCalendarResources, getColors
│   │   │   │   ├── sync.rs         # incrementalSync, incrementalAuth
│   │   │   │   ├── holds.rs        # getHolds, getHold, createHold, updateHold, deleteHold, getHoldEvent, createHoldEvent, updateHoldEvent, getHoldAliasAvailable
│   │   │   │   ├── user.rs         # getUser, updateUser, deleteUser, getUserPreferences, updateUserPreferences, getUserSettings, updateUserSettings, getUsernameAvailable, updatePrimaryAccount, getPrimaryAccountAvailable, removeAccount, identifyDevice
│   │   │   │   ├── contacts.rs     # getContacts, getGroupMembers, getGroupMemberAttendees
│   │   │   │   ├── notion.rs       # getNotionPages, getRecentNotionPages, getNotionSearch, createNotionPage, upsertNotionMeetingNote, createNotionTaskDatabase, canEditNotionBlocks, getNotionUsersWithoutAccessToBlock, grantNotionUsersAccessToBlock, getNotionSessionUsers, getNotionWorkspaces, connectNotionWorkspace, updateNotionWorkspaceSettings
│   │   │   │   ├── auth.rs         # getNotionLoginUrl, getNotionLogoutUrl, getNotionAddAccountUrl, createNotionSession, refreshNotionSession, createCalendarAccount
│   │   │   │   ├── conferencing.rs # createConferencingAccount, updateConferencingAccount, createConferencing, updateConferencing, deleteConferencing
│   │   │   │   ├── sync_cal.rs     # getSynchronizedCalendars, createSynchronizedCalendar, deleteSynchronizedCalendar, createSynchronizedEvent, deleteSynchronizedEvent
│   │   │   │   ├── social.rs       # createReferral, getReferralSuggestions, sendFeedback
│   │   │   │   ├── files.rs        # getUploadFileURL, deleteFile, getTranscriptionRecordAncestorChainForEvent
│   │   │   │   └── ai.rs           # getNotionInferenceTranscript, getNotionInferenceTranscriptsForUser, runNotionInferenceTranscript, stopNotionInferenceTranscript
│   │   │   └── error.rs        # API error types, HTTP status mapping
│   │   └── Cargo.toml
│   └── ncal-cli/           # CLI binary
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands/       # One module per CLI command group
│       │   │   ├── mod.rs
│       │   │   ├── auth.rs
│       │   │   ├── events.rs
│       │   │   ├── calendars.rs
│       │   │   ├── holds.rs
│       │   │   ├── sync.rs
│       │   │   ├── user.rs
│       │   │   ├── contacts.rs
│       │   │   └── notion.rs
│       │   ├── output.rs       # --json / --pretty formatting
│       │   └── config.rs       # Config file, env var resolution
│       └── Cargo.toml
├── research/                   # RE docs (existing)
├── scripts/                    # RE scripts (existing)
└── IMPLEMENTATION_PLAN.md      # This file
```

### Why two crates

- `ncal-api` is a standalone library — importable by other Rust tools, testable without CLI overhead, clean API surface. Zero CLI dependencies. Async-first.
- `ncal-cli` is the thin CLI shell — clap parsing, output formatting, exit codes. Depends on `ncal-api`.

### Dependency direction (strict)

```
ncal-cli → ncal-api → {reqwest, serde, chrono, ...}
         ↘ {clap, keyring, open, ...}
```

`ncal-api` must **never** depend on `ncal-cli`. The API crate exposes `NotionCalendarClient` + typed endpoint methods + all serde types. The CLI crate handles:
- Argument parsing and validation
- Credential resolution (which sources to try, in what order)
- Output formatting (JSON vs pretty)
- Exit code mapping from `ApiError`

### Module visibility rules

- `ncal-api/src/types/` — all `pub` (these are the public API surface for consumers)
- `ncal-api/src/endpoints/` — all `pub` (typed wrappers around `rpc()`)
- `ncal-api/src/client.rs` — `NotionCalendarClient` is `pub`, internal helpers are `pub(crate)`
- `ncal-api/src/auth.rs` — `CredentialSource` trait and `Credentials` struct are `pub`, implementations are `pub`
- `ncal-cli/src/commands/` — `pub(crate)` (only visible within the CLI binary)

---

## 2. Dependencies

### ncal-api

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client (with `rustls-tls`, `json`, `gzip` features) |
| `serde`, `serde_json` | JSON serialization for all API types |
| `chrono` | Date/time parsing (ISO 8601, epoch ms) |
| `thiserror` | Error type derivation |
| `tracing` | Structured logging |
| `rusty-leveldb` | Read Chromium LocalStorage LevelDB for `auth --from-app` |
| `tokio` | Async runtime (reqwest requirement) |
| `url` | URL construction with query params |
| `base64` | JWT inspection (check expiry without full decode) |

### ncal-cli

| Crate | Purpose |
|-------|---------|
| `clap` (derive) | CLI argument parsing |
| `serde_json` | JSON output formatting |
| `tokio` | Async runtime |
| `tracing-subscriber` | Log output control |
| `directories` | Platform-appropriate config/data dirs |
| `keyring` | OS keychain integration for credential storage |
| `open` | Open browser for OAuth flow |

---

## 3. Type system design

### 3.1 Core enums (closed sets — enum + match is correct)

```rust
// Provider: 4 variants, fixed by Notion's backend. Enum with exhaustive match.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Provider { Google, Notion, Icloud, Outlook }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SendUpdates { All, None, ExternalOnly }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecurringUpdate { Single, All, AllFollowing }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventStatus { Confirmed, Tentative, Cancelled }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseStatus { NeedsAction, Accepted, Declined, Tentative }
```

### 3.2 Serde strategy

- All API types derive `Serialize + Deserialize` with `#[serde(rename_all = "camelCase")]`.
- **Separate request and response types.** The API has different optionality:
  - Responses: use `#[serde(default)]` liberally (provider-specific fields may be absent).
  - Requests: use `#[serde(skip_serializing_if = "Option::is_none")]` (send only set fields).
- Use `#[serde(untagged)]` for union response types (success vs error per-calendar results).
- Discriminated unions (e.g. NotionPageLocation) use `#[serde(tag = "type")]`.
- Unknown enum variants: use `#[serde(other)]` fallback to avoid breaking on new server values.

### 3.3 Request vs response type separation

The API uses the same `Event` shape in both directions, but requests are partial updates while responses are full objects. Use a shared `Event` struct for responses and typed request structs for mutations:

```rust
/// Response type — all fields optional because providers return different subsets.
/// Deserialize-only (no skip_serializing_if noise).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub account_id: String,
    pub calendar_id: String,
    #[serde(default)] pub kind: Option<String>,
    #[serde(default)] pub summary: Option<String>,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub start: Option<DateTimeOrDate>,
    #[serde(default)] pub end: Option<DateTimeOrDate>,
    #[serde(default)] pub status: Option<EventStatus>,
    #[serde(default)] pub attendees: Option<Vec<Attendee>>,
    #[serde(default)] pub provider: Option<Provider>,
    // ... all 50+ fields from spec §6.4
    /// Catch-all for unknown fields (forward-compat with new API versions)
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Request type for createEvent — builder pattern for ergonomic construction.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventMutation {
    pub provider: Provider,
    pub account_id: String,
    pub calendar_id: String,
    pub event_data: EventData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_updates: Option<SendUpdates>,
}

/// Request type for updateEvents — only changed fields are serialized.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventMutation {
    pub account_id: String,
    pub provider: Provider,
    pub event_id: String,
    pub calendar_id: String,
    pub event_data: serde_json::Value, // partial update — only set fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_updates: Option<SendUpdates>,
}
```

### 3.4 Builder pattern for complex query params

Endpoints like `getEvents` have 15+ optional fields. Use consuming builders:

```rust
/// Builder for getEvents queries (Creational: Builder pattern)
pub struct EventQueryBuilder {
    provider: Provider,
    account_id: String,
    calendar_id: String,
    time_min: Option<i64>,
    time_max: Option<i64>,
    query: Option<String>,
    limit: Option<u32>,
    single_events: Option<bool>,
    show_deleted: Option<bool>,
    // ...
}

impl EventQueryBuilder {
    pub fn new(provider: Provider, account_id: &str, calendar_id: &str) -> Self { /* ... */ }
    pub fn time_range(mut self, min: i64, max: i64) -> Self { /* ... */ }
    pub fn query(mut self, q: &str) -> Self { /* ... */ }
    pub fn limit(mut self, n: u32) -> Self { /* ... */ }
    pub fn build(self) -> EventQuery { /* ... */ }
}
```

### 3.5 Forward compatibility with `#[serde(flatten)]`

The API adds new fields regularly. Every response struct should capture unknown fields:

```rust
#[serde(flatten)]
pub extra: serde_json::Map<String, serde_json::Value>,
```

This prevents deserialization failures when the server adds new fields, and lets the CLI pass through data it doesn't understand (important for `--json` output).

---

## 4. HTTP client (`client.rs`)

### 4.1 Architecture: Token management with interior mutability

The client needs `&self` for all API calls (shared across async tasks) but must mutate tokens on refresh. Use `Arc<RwLock<TokenState>>` — the Decorator pattern wraps the raw HTTP transport with auth concerns.

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mutable token state, refreshed transparently.
struct TokenState {
    access_token: String,
    refresh_token: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    /// Cooldown: track recent refresh attempts
    last_refresh: Option<std::time::Instant>,
    refresh_count_window: u32,
}

/// Immutable client config, set once at construction.
struct ClientConfig {
    base_url: Url,          // https://calendar-api.notion.so
    version: String,        // "1.132.0+..."
    client_type: String,    // "cli"
    timezone: String,
    locale: String,
}

/// The main API client. Clone-friendly (Arc internals).
/// Pattern: Facade — simplifies the HTTP + auth + retry subsystem.
#[derive(Clone)]
pub struct NotionCalendarClient {
    http: reqwest::Client,
    config: Arc<ClientConfig>,
    tokens: Arc<RwLock<TokenState>>,
}
```

### 4.2 Generic RPC transport

```rust
impl NotionCalendarClient {
    /// Generic POST /v2/{operation} with JSON body.
    /// Handles auto-refresh, 401 retry, and error envelope parsing.
    pub async fn rpc<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        operation: &str,
        params: &Req,
    ) -> Result<Resp, ApiError> {
        // 1. Check token expiry, refresh if needed
        self.ensure_valid_token().await?;

        // 2. Read current token (short-lived read lock)
        let token = self.tokens.read().await.access_token.clone();

        // 3. Make request
        let resp = self.do_request(operation, params, &token).await?;

        // 4. On 401 invalidToken: refresh once and retry
        if resp.status() == 401 {
            self.refresh_token().await?;
            let new_token = self.tokens.read().await.access_token.clone();
            let resp = self.do_request(operation, params, &new_token).await?;
            return self.parse_response(resp).await;
        }

        self.parse_response(resp).await
    }

    /// Raw HTTP call without auth logic — separated for testability.
    async fn do_request<Req: Serialize>(
        &self,
        operation: &str,
        params: &Req,
        token: &str,
    ) -> Result<reqwest::Response, ApiError> {
        let url = self.config.base_url.join(&format!("/v2/{operation}"))?;
        Ok(self.http
            .post(url)
            .query(&[
                ("ver", &self.config.version),
                ("client", &self.config.client_type),
                ("tz", &self.config.timezone),
                ("locale", &self.config.locale),
            ])
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .header("X-Client-Platform", "cli")
            .header("X-Client-Type", "cli")
            .header("X-Client-OS", std::env::consts::OS)
            .header("X-TimeZone", &self.config.timezone)
            .header("X-Notion-Authenticated", "true")
            .json(params)
            .send()
            .await?)
    }
}
```

### 4.3 Auto-refresh with cooldown

```rust
impl NotionCalendarClient {
    /// Refresh if token expires within 60 seconds.
    async fn ensure_valid_token(&self) -> Result<(), ApiError> {
        let tokens = self.tokens.read().await;
        if tokens.expires_at > chrono::Utc::now() + chrono::Duration::seconds(60) {
            return Ok(()); // Still valid
        }
        drop(tokens); // Release read lock before write
        self.refresh_token().await
    }

    /// Call /v2/refreshNotionSession. Cooldown: max 5 attempts per 30s.
    async fn refresh_token(&self) -> Result<(), ApiError> {
        let mut tokens = self.tokens.write().await;
        // Double-check after acquiring write lock (another task may have refreshed)
        if tokens.expires_at > chrono::Utc::now() + chrono::Duration::seconds(60) {
            return Ok(());
        }
        // Cooldown check
        if let Some(last) = tokens.last_refresh {
            if last.elapsed() < std::time::Duration::from_secs(30) && tokens.refresh_count_window >= 5 {
                return Err(ApiError::RefreshCooldown);
            }
        }
        // Call refresh endpoint (bypass rpc() to avoid recursion)
        let resp = self.do_request(
            "refreshNotionSession",
            &serde_json::json!({"refreshToken": tokens.refresh_token}),
            &tokens.access_token,
        ).await?;
        let body: RefreshResponse = self.parse_response(resp).await?;
        tokens.access_token = body.access_token;
        tokens.refresh_token = body.refresh_token;
        tokens.expires_at = body.access_token_expires_at.parse()?;
        tokens.last_refresh = Some(std::time::Instant::now());
        tokens.refresh_count_window += 1;
        // Persist updated tokens via callback (see §5.3)
        Ok(())
    }
}
```

### 4.4 Streaming RPC for NDJSON (AI transcripts)

```rust
impl NotionCalendarClient {
    /// POST with streaming NDJSON response (for runNotionInferenceTranscript).
    pub async fn rpc_stream<Req: Serialize>(
        &self,
        operation: &str,
        params: &Req,
    ) -> Result<impl futures::Stream<Item = Result<serde_json::Value, ApiError>>, ApiError> {
        self.ensure_valid_token().await?;
        let token = self.tokens.read().await.access_token.clone();
        let resp = self.do_request(operation, params, &token).await?;
        // Return byte stream → split on newlines → parse each line as JSON
        Ok(ndjson_stream(resp.bytes_stream()))
    }
}
```

---

## 5. Auth module (`auth.rs`)

### 5.1 Credential source trait (Strategy pattern — open for extension)

There are 4+ ways to obtain credentials, and more may come (e.g. service accounts, CI tokens). Each source is independent, substantial, and has its own data — trait is the right abstraction.

```rust
/// Shared credential shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub user_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: String, // ISO 8601
}

/// Strategy pattern: each credential source is a separate impl.
/// Adding a new source (e.g. service account) = new struct + impl. No existing code touched.
pub trait CredentialSource {
    fn name(&self) -> &str;
    fn obtain(&self) -> Result<Credentials, AuthError>;
}
```

### 5.2 Credential source implementations

```rust
/// Read tokens from the running Notion Calendar desktop app's LevelDB.
pub struct DesktopAppSource {
    app_data_dir: PathBuf, // ~/Library/Application Support/Notion Calendar/
}

impl CredentialSource for DesktopAppSource {
    fn name(&self) -> &str { "desktop-app" }
    fn obtain(&self) -> Result<Credentials, AuthError> {
        let db_path = self.app_data_dir.join("Local Storage/leveldb");
        // Copy to temp dir (app holds LevelDB lock while running)
        let tmp = tempfile::tempdir()?;
        fs_extra::dir::copy(&db_path, tmp.path(), &Default::default())?;
        std::fs::remove_file(tmp.path().join("leveldb/LOCK")).ok();

        let mut db = rusty_leveldb::DB::open(
            tmp.path().join("leveldb"),
            rusty_leveldb::Options::default(),
        )?;
        let key = b"_https://calendar.notion.so\x00\x01user.auth.currentUser";
        let value = db.get(key)?.ok_or(AuthError::NoCredentials)?;
        serde_json::from_slice(&value).map_err(Into::into)
    }
}

/// Read tokens from environment variables.
pub struct EnvVarSource;

impl CredentialSource for EnvVarSource {
    fn name(&self) -> &str { "env" }
    fn obtain(&self) -> Result<Credentials, AuthError> {
        Ok(Credentials {
            user_id: std::env::var("NCAL_USER_ID").map_err(|_| AuthError::NoCredentials)?,
            access_token: std::env::var("NCAL_ACCESS_TOKEN").map_err(|_| AuthError::NoCredentials)?,
            refresh_token: std::env::var("NCAL_REFRESH_TOKEN").unwrap_or_default(),
            access_token_expires_at: std::env::var("NCAL_TOKEN_EXPIRES_AT").unwrap_or_default(),
        })
    }
}

/// Read tokens from OS keychain (previously stored by this CLI).
pub struct KeychainSource {
    service: String, // "notion-calendar-cli"
}

impl CredentialSource for KeychainSource {
    fn name(&self) -> &str { "keychain" }
    fn obtain(&self) -> Result<Credentials, AuthError> {
        let entry = keyring::Entry::new(&self.service, "default")?;
        let json = entry.get_password().map_err(|_| AuthError::NoCredentials)?;
        serde_json::from_str(&json).map_err(Into::into)
    }
}

/// Browser-based OAuth flow (interactive).
pub struct BrowserOAuthSource { /* config for redirect URI, etc. */ }
// impl CredentialSource for BrowserOAuthSource { ... }
```

### 5.3 Credential resolution chain

The CLI tries sources in priority order — first match wins:

```rust
/// Try each source in order. First success wins.
pub fn resolve_credentials(sources: &[Box<dyn CredentialSource>]) -> Result<Credentials, AuthError> {
    for source in sources {
        match source.obtain() {
            Ok(creds) => {
                tracing::info!(source = source.name(), "credentials loaded");
                return Ok(creds);
            }
            Err(AuthError::NoCredentials) => continue,
            Err(e) => return Err(e), // Hard error (e.g. corrupted LevelDB)
        }
    }
    Err(AuthError::NoCredentials)
}

// Default resolution order:
// 1. EnvVarSource (CI/scripts override everything)
// 2. KeychainSource (previously stored by `ncal auth login` or `ncal auth from-app`)
// 3. DesktopAppSource (fallback: read from running app)
```

### 5.4 Credential persistence

After obtaining credentials (from any source or after refresh), persist to keychain:

```rust
pub fn store_credentials(creds: &Credentials) -> Result<(), AuthError> {
    let entry = keyring::Entry::new("notion-calendar-cli", "default")?;
    entry.set_password(&serde_json::to_string(creds)?)?;
    Ok(())
}
```

### 5.5 Token refresh callback

The `NotionCalendarClient` needs to persist refreshed tokens. Use a callback:

```rust
pub type OnTokenRefresh = Arc<dyn Fn(&Credentials) + Send + Sync>;

// Passed to client at construction:
let on_refresh: OnTokenRefresh = Arc::new(|creds| {
    store_credentials(creds).ok(); // Best-effort persist
});
```

---

## 6. CLI command map

All commands follow the agent-CLI-UX skill principles: `--json` for machine output, exit codes per spec, secrets from env/keychain.

### 6.1 Auth (P0)

```
ncal auth login              # Browser-based OAuth flow
ncal auth from-app           # Extract tokens from desktop Notion Calendar app
ncal auth status             # Show current auth state (token expiry, user email)
ncal auth refresh            # Force token refresh
ncal auth logout             # Clear stored credentials
```

### 6.2 Events (P0)

```
ncal events list             # List events across calendars
  --account <id>             # Filter by account
  --calendar <id>            # Filter by calendar
  --from <datetime>          # Start time (ISO 8601 or relative: "today", "-1d")
  --to <datetime>            # End time
  --query <text>             # Search text
  --limit <n>                # Max results
  --include-deleted          # Include cancelled events
  --json                     # JSON output

ncal events get <event-id>   # Get single event
  --account <id>             # Required
  --calendar <id>            # Required

ncal events create           # Create event
  --account <id>             # Required
  --calendar <id>            # Required
  --summary <title>          # Event title
  --start <datetime>         # Start time (ISO 8601)
  --end <datetime>           # End time
  --description <text>       # Description
  --location <text>          # Location
  --attendees <emails>       # Comma-separated
  --send-updates <all|none|externalOnly>
  --json                     # Output created event as JSON

ncal events update <event-id>  # Update event
  --account <id>
  --calendar <id>
  --summary <title>          # Fields to update (only specified fields sent)
  --start <datetime>
  --end <datetime>
  --description <text>
  --location <text>
  --attendees <emails>
  --send-updates <all|none|externalOnly>

ncal events delete <event-id>  # Delete/cancel event
  --account <id>
  --calendar <id>
  --send-updates <all|none|externalOnly>
  --hard                     # Use deleteEvents RPC (default: updateEvents with status=cancelled)

ncal events export           # Export events as ICS
  --account <id>
  --calendar <id>
  --from <datetime>
  --to <datetime>
```

### 6.3 Calendars (P0)

```
ncal calendars list          # List all calendars across accounts
  --account <id>             # Filter by account
  --json

ncal calendars get <id>      # Get calendar details
  --account <id>

ncal calendars add <id>      # Subscribe to a calendar (insertCalendarList)
  --account <id>

ncal calendars remove <id>   # Unsubscribe from a calendar (deleteCalendarList)
  --account <id>

ncal calendars update <id>   # Update calendar properties
  --account <id>
  --color <hex>
  --summary <name>

ncal calendars free-busy     # Check free/busy
  --calendars <account:calendar,...>
  --from <datetime>
  --to <datetime>

ncal calendars colors        # List color palette
```

### 6.4 Sync (P0)

```
ncal sync                    # Run incremental sync
  --tokens-file <path>       # File to read/write sync tokens (default: ~/.config/ncal/sync-tokens.json)
  --watch                    # Keep running, sync every 60s
  --interval <seconds>       # Custom sync interval
  --json                     # Stream changed events/calendars as JSON lines
```

### 6.5 User & Accounts (P0/P1)

```
ncal whoami                  # Show current user info (getUser)
  --json

ncal accounts list           # List connected accounts with capabilities
  --json

ncal accounts set-primary <id>  # Set primary account (updatePrimaryAccount)

ncal accounts remove <id>      # Disconnect account (removeAccount)

ncal preferences get         # Show user preferences
  --json

ncal preferences set         # Update preferences
  --timezone <tz>
  --locale <locale>
  --format-24h <true|false>
```

### 6.6 Holds / Scheduling (P1)

```
ncal holds list              # List scheduling links (getHolds)
  --json

ncal holds get <alias>       # Get hold details (getHold, public view)
  --from <datetime>
  --to <datetime>

ncal holds create            # Create scheduling link
  --alias <slug>
  --title <title>
  --duration <minutes>
  --timezone <tz>
  --account <id>
  --calendar <id>
  --description <text>

ncal holds update <id>       # Update hold
  --title <title>
  --duration <minutes>
  --alias <slug>

ncal holds delete <id>       # Delete hold

ncal holds book <short-id>   # Book a slot (createHoldEvent)
  --email <email>
  --name <name>
  --start <datetime>
  --end <datetime>

ncal holds check-alias <alias>  # Check alias availability
```

### 6.7 Contacts (P2)

```
ncal contacts list           # List contacts
  --json

ncal contacts groups         # Check if emails are groups
  --emails <email1,email2>
  --account <id>
```

### 6.8 Notion Integration (P2)

```
ncal notion pages recent     # Recent Notion pages
  --workspace <id>
  --limit <n>

ncal notion pages search     # Search Notion pages
  --workspace <id>
  --query <text>
  --limit <n>
  --type <block|collection>

ncal notion pages create     # Create Notion page
  --workspace <id>
  --title <title>
  --location-type <private|page|database>
  --location-id <id>

ncal notion meeting-note     # Create/update meeting note
  --workspace <id>
  --event-id <id>
  --account <id>
  --calendar <id>
  --share

ncal notion workspaces list  # List connected workspaces

ncal notion users            # List Notion users with calendar access
```

### 6.9 Conferencing (P2)

```
ncal conferencing create     # Create meeting (Zoom)
  --provider zoom
  --topic <title>
  --start <datetime>
  --end <datetime>
  --timezone <tz>
  --recurring

ncal conferencing update <meeting-id>
  --start <datetime>

ncal conferencing delete <meeting-id>

ncal conferencing accounts list
ncal conferencing accounts add --name <name> --url <template>
ncal conferencing accounts update <id> --name <name> --url <template>
```

### 6.10 Synchronized Calendars / Event Blocking (P2)

```
ncal blocking list           # List synchronized calendars
ncal blocking create         # Create event blocking rule
  --source-account <id>
  --source-calendar <id>
  --target-account <id>
  --target-calendar <id>
ncal blocking delete <id>    # Remove blocking rule
```

---

## 7. Error handling

### 7.1 Layered error types (thiserror in library, mapped to exit codes in CLI)

**Library errors** (`ncal-api`) use `thiserror` — structured, matchable, no formatting opinion:

```rust
// ncal-api/src/error.rs

/// Server error body, deserialized from API JSON responses.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorBody {
    pub code: Option<String>,           // e.g. "invalidToken"
    pub message: Option<String>,
    pub message_id: Option<String>,
    pub intl_message: Option<String>,
    pub status_text: Option<String>,
}

/// API client errors — library consumers match on these.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("API error (HTTP {status}): {body:?}")]
    Server { status: u16, body: ApiErrorBody },

    #[error("logged out by server (invalidToken)")]
    InvalidToken,

    #[error("token refresh failed: cooldown active")]
    RefreshCooldown,

    #[error("etag mismatch (HTTP 412)")]
    EtagMismatch { server_state: serde_json::Value },

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
}

/// Auth-specific errors (separate from API errors).
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no credentials found")]
    NoCredentials,

    #[error("credential source '{source}' failed: {reason}")]
    SourceFailed { source: String, reason: String },

    #[error("LevelDB read error: {0}")]
    LevelDb(String),

    #[error("keychain error: {0}")]
    Keychain(String),

    #[error("API error during auth: {0}")]
    Api(#[from] ApiError),
}
```

**CLI error mapping** (`ncal-cli`) converts library errors to exit codes:

```rust
// ncal-cli/src/main.rs

fn exit_code_for(err: &CliError) -> i32 {
    match err {
        CliError::Usage(_) => 2,
        CliError::Auth(_) => 3,
        CliError::Api(ApiError::InvalidToken) => 3,
        CliError::Api(ApiError::Network(_)) => 4,
        CliError::Api(ApiError::Server { .. }) => 4,
        _ => 1,
    }
}
```

### 7.2 Exit codes (per agent-cli-ux skill)

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Usage / validation error (bad flags, missing required args) |
| 3 | Auth / config missing (no credentials, expired and can't refresh) |
| 4 | Upstream API error (server returned error, network failure) |

---

## 8. Output formatting

```rust
pub enum OutputMode {
    Json,       // --json: machine-readable, one JSON object per result
    Pretty,     // default: human-readable table/text
}

// Progress/status → stderr
// Data → stdout
// Errors → stderr with exit code
```

For `--json` on list commands, emit one JSON object per line (JSON Lines) for streaming compatibility.

---

## 9. Config file

Location: `~/.config/ncal/config.toml` (or `$NCAL_CONFIG`)

```toml
[auth]
# Credentials stored in OS keychain under "notion-calendar-cli"
# Override with NCAL_ACCESS_TOKEN / NCAL_REFRESH_TOKEN env vars

[defaults]
timezone = "Europe/Zagreb"
locale = "en-US"
account = "609ede4d-..."      # default account for commands
calendar = "opustovit@cx2.vc" # default calendar for commands
output = "pretty"             # "json" or "pretty"

[sync]
tokens_file = "~/.config/ncal/sync-tokens.json"
interval = 60
```

---

## 10. Implementation milestones

### M0: Scaffold (est. 1-2 days)

- [ ] `cargo init` workspace with `ncal-api` and `ncal-cli` crates
- [ ] Add dependencies to `Cargo.toml` files
- [ ] Implement `Provider`, `SendUpdates`, `EventStatus`, `ResponseStatus` enums
- [ ] Implement `NotionCalendarClient` with `rpc()` method
- [ ] Implement `ApiError` type
- [ ] Basic `main.rs` with clap skeleton

### M1: Auth (est. 2-3 days)

- [ ] `auth from-app` — LevelDB reader for Chromium LocalStorage
- [ ] `auth status` — display current token state
- [ ] `auth refresh` — call refreshNotionSession
- [ ] `auth login` — browser OAuth flow with local redirect capture
- [ ] `auth logout` — clear keychain entry
- [ ] Credential storage in OS keychain via `keyring`
- [ ] Auto-refresh middleware in `rpc()` client

### M2: Read operations (est. 3-4 days)

- [ ] `Event`, `Calendar`, `User`, `Account`, `Contact` serde types (complete from spec §6)
- [ ] `whoami` / `getUser`
- [ ] `accounts list` — from user.accounts
- [ ] `calendars list` / `getCalendarLists`
- [ ] `calendars colors` / `getColors`
- [ ] `events list` / `getEvents` with time range, filtering
- [ ] `events get` / `getEvent`
- [ ] `contacts list` / `getContacts`
- [ ] `sync` / `incrementalSync` with token file persistence
- [ ] `--json` output mode for all commands

### M3: Write operations (est. 3-4 days)

- [ ] `events create` / `createEvent`
- [ ] `events update` / `updateEvents` (partial update — only send changed fields)
- [ ] `events delete` — `updateEvents` with `status: cancelled` (default) or `deleteEvents` with `--hard`
- [ ] `calendars add` / `insertCalendarList`
- [ ] `calendars remove` / `deleteCalendarList`
- [ ] `calendars update` / `updateCalendars`
- [ ] `preferences set` / `updateUserPreferences`

### M4: Holds / Scheduling (est. 2-3 days)

- [ ] `HoldGroup`, `TimeRange`, `HoldLocation` serde types
- [ ] `holds list` / `getHolds`
- [ ] `holds get` / `getHold` (public endpoint)
- [ ] `holds create` / `createHold`
- [ ] `holds update` / `updateHold`
- [ ] `holds delete` / `deleteHold`
- [ ] `holds book` / `createHoldEvent`
- [ ] `holds check-alias` / `getHoldAliasAvailable`

### M5: Notion integration (est. 2-3 days)

- [ ] `NotionPage`, `NotionWorkspace`, `NotionUser` serde types
- [ ] `notion pages recent` / `getRecentNotionPages`
- [ ] `notion pages search` / `getNotionSearch`
- [ ] `notion pages create` / `createNotionPage`
- [ ] `notion meeting-note` / `upsertNotionMeetingNote`
- [ ] `notion workspaces list` / `getNotionWorkspaces`
- [ ] `notion users` / `getNotionSessionUsers`

### M6: Conferencing & Blocking (est. 1-2 days)

- [ ] `conferencing create/update/delete` (Zoom)
- [ ] `conferencing accounts` CRUD
- [ ] `blocking list/create/delete` (synchronized calendars)

### M7: Polish & Release (est. 2-3 days)

- [ ] `sync --watch` mode with signal handling
- [ ] `events export` → ICS output
- [ ] `calendars free-busy`
- [ ] Shell completions (clap_complete)
- [ ] `--dry-run` for mutating commands
- [ ] Man page / `--help` polish
- [ ] CI: `cargo clippy`, `cargo test`, `cargo fmt`
- [ ] GitHub release workflow with cross-compilation (macOS aarch64 + x86_64, Linux)

---

## 11. Testing strategy

### Unit tests
- Serde round-trip tests for every API type using captured JSON from `research/artifacts/captures/json/`.
- Mock HTTP responses for each endpoint.

### Integration tests
- Against a real Notion Calendar account (opt-in via `NCAL_INTEGRATION_TEST=1`).
- Test auth flow, event CRUD, sync cycle.
- Snapshot test JSON output for stability.

### Property tests
- Time range handling (timezone edge cases, all-day vs timed events).
- Provider-specific serialization differences (Google vs iCloud response shapes).

---

## 12. API coverage matrix

All 85 endpoints mapped to CLI commands:

| # | Endpoint | CLI Command | Priority |
|---|----------|-------------|----------|
| 1 | getEvents | `events list` | P0 |
| 2 | getEvent | `events get` | P0 |
| 3 | createEvent | `events create` | P0 |
| 4 | updateEvents | `events update` | P0 |
| 5 | deleteEvents | `events delete --hard` | P0 |
| 6 | getCalendarLists | `calendars list` | P0 |
| 7 | getCalendars | `calendars search` | P1 |
| 8 | insertCalendarList | `calendars add` | P1 |
| 9 | deleteCalendarList | `calendars remove` | P1 |
| 10 | updateCalendars | `calendars update` | P1 |
| 11 | getCalendarFreeBusy | `calendars free-busy` | P1 |
| 12 | getCalendarResources | `calendars resources` | P2 |
| 13 | getColors | `calendars colors` | P1 |
| 14 | getEventInbox | `events inbox` | P2 |
| 15 | exportEvents | `events export` | P1 |
| 16 | incrementalSync | `sync` | P0 |
| 17 | incrementalAuth | (internal, used during Google scope upgrade) | P3 |
| 18 | getContacts | `contacts list` | P2 |
| 19 | getGroupMembers | `contacts groups` | P2 |
| 20 | getGroupMemberAttendees | (internal, used in event detail expansion) | P3 |
| 21 | getHolds | `holds list` | P1 |
| 22 | getHold | `holds get` | P1 |
| 23 | createHold | `holds create` | P1 |
| 24 | updateHold | `holds update` | P1 |
| 25 | deleteHold | `holds delete` | P1 |
| 26 | getHoldEvent | (internal, used in hold rescheduling UI) | P2 |
| 27 | createHoldEvent | `holds book` | P1 |
| 28 | updateHoldEvent | `holds reschedule` / `holds cancel` | P2 |
| 29 | getHoldAliasAvailable | `holds check-alias` | P1 |
| 30 | getUser | `whoami` | P0 |
| 31 | updateUser | `user update` | P2 |
| 32 | deleteUser | (dangerous, omit from CLI or require --confirm) | P3 |
| 33 | getUserPreferences | `preferences get` | P1 |
| 34 | updateUserPreferences | `preferences set` | P1 |
| 35 | getUserSettings | `settings get` | P2 |
| 36 | updateUserSettings | `settings set` | P2 |
| 37 | getUsernameAvailable | `user check-username` | P2 |
| 38 | updatePrimaryAccount | `accounts set-primary` | P1 |
| 39 | getPrimaryAccountAvailable | (internal) | P3 |
| 40 | removeAccount | `accounts remove` | P1 |
| 41 | identifyDevice | (telemetry, skip) | skip |
| 42 | createCalendarAccount | `accounts add` | P2 |
| 43 | updateCalendarAccountSettings | `accounts settings` | P2 |
| 44 | getNotionPages | `notion pages get` | P2 |
| 45 | getRecentNotionPages | `notion pages recent` | P2 |
| 46 | getNotionSearch | `notion pages search` | P2 |
| 47 | createNotionPage | `notion pages create` | P2 |
| 48 | upsertNotionMeetingNote | `notion meeting-note` | P2 |
| 49 | createNotionTaskDatabase | `notion task-db create` | P2 |
| 50 | canEditNotionBlocks | (internal check) | P3 |
| 51 | getNotionUsersWithoutAccessToBlock | (internal check) | P3 |
| 52 | grantNotionUsersAccessToBlock | `notion grant-access` | P2 |
| 53 | getNotionSessionUsers | `notion users` | P2 |
| 54 | getNotionWorkspaces | `notion workspaces list` | P2 |
| 55 | getNotionLoginUrl | (auth internal) | P0 |
| 56 | getNotionLogoutUrl | (auth internal) | P0 |
| 57 | getNotionAddAccountUrl | (auth internal) | P2 |
| 58 | connectNotionWorkspace | `notion workspaces connect` | P2 |
| 59 | updateNotionWorkspaceSettings | `notion workspaces settings` | P2 |
| 60 | createNotionSession | (auth internal) | P0 |
| 61 | refreshNotionSession | (auth internal, auto-refresh) | P0 |
| 62 | createConferencingAccount | `conferencing accounts add` | P2 |
| 63 | updateConferencingAccount | `conferencing accounts update` | P2 |
| 64 | createConferencing | `conferencing create` | P2 |
| 65 | updateConferencing | `conferencing update` | P2 |
| 66 | deleteConferencing | `conferencing delete` | P2 |
| 67 | getSynchronizedCalendars | `blocking list` | P2 |
| 68 | createSynchronizedCalendar | `blocking create` | P2 |
| 69 | deleteSynchronizedCalendar | `blocking delete` | P2 |
| 70 | createSynchronizedEvent | `blocking block-event` | P2 |
| 71 | deleteSynchronizedEvent | `blocking unblock-event` | P2 |
| 72 | createReferral | (social, skip) | skip |
| 73 | getReferralSuggestions | (social, skip) | skip |
| 74 | sendFeedback | (social, skip) | skip |
| 75 | logToSplunk | (telemetry, skip) | skip |
| 76 | incrementMetrics | (telemetry, skip) | skip |
| 77 | getDecagonToken | (support widget, skip) | skip |
| 78 | meetingNotificationHeartbeat | (desktop-only, skip) | skip |
| 79 | stopNotionInferenceTranscript | `notion ai stop` | P3 |
| 80 | getNotionInferenceTranscript | `notion ai transcript` | P3 |
| 81 | getNotionInferenceTranscriptsForUser | `notion ai history` | P3 |
| 82 | runNotionInferenceTranscript | `notion ai run` | P3 |
| 83 | getUploadFileURL | `files upload` | P2 |
| 84 | deleteFile | `files delete` | P2 |
| 85 | getTranscriptionRecordAncestorChainForEvent | (AI internal) | P3 |

**Coverage:** 68 endpoints mapped to CLI commands, 9 internal-only, 8 skipped (telemetry/social).

---

## 13. Behavioral notes for implementers

1. **Event deletion uses updateEvents, not deleteEvents.** The Notion Calendar UI sets `status: "cancelled"` + `responseStatus: "declined"` via `updateEvents` for both Google and iCloud. The `deleteEvents` endpoint should be available via `--hard` flag but is not the default path. (spec §7.1, confirmed from live traffic)

2. **Provider-specific response shapes.** Google returns `htmlLink`, `conferenceData`, `colorId`; iCloud omits these. Google omits empty fields; iCloud returns them as `""` or `[]`. Use `#[serde(default)]` everywhere.

3. **Etag-based optimistic concurrency on preferences.** `updateUserPreferences` sends `lastLiveSnapshotETag` and handles HTTP 412 with retry logic. The CLI should implement a simpler read-modify-write with one retry.

4. **NDJSON streaming for AI transcripts.** `runNotionInferenceTranscript` returns `Accept: application/x-ndjson` — needs streaming response handling, not standard JSON parse.

5. **extendedProperties.shared conventions.** Events carry Notion metadata in `cron.meetingNote`, `cron.holdGroup.*`, `n.attchwsid.*` keys. Preserve these on updates.

6. **Sync tokens must be persisted.** `incrementalSync` returns new tokens that must replace the old ones. Store in a JSON file. If tokens are lost, a full re-fetch is needed via `getEvents` + `getCalendarLists`.
