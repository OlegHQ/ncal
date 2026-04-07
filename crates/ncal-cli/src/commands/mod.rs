mod auth;
mod calendars;
mod contacts;
pub(crate) mod context;
mod events;
mod holds;
mod user;

use std::path::Path;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use ncal_api::endpoints::{incremental_sync, IncrementalSyncRequest, SyncTokenInput};
use ncal_api::types::{IncrementalSyncResponse, SyncToken};

use crate::cache::{self, CacheFile};
use crate::config::AppConfig;
use crate::output::{hint, print_json};
use crate::Cli;
use crate::CliError;

#[derive(Debug, Serialize, Deserialize, Default)]
struct SyncTokensFile {
    #[serde(default)]
    tokens: Vec<SyncToken>,
}

fn token_to_input(t: SyncToken) -> SyncTokenInput {
    SyncTokenInput {
        provider: t.provider,
        account_id: t.account_id,
        resource_id: t.resource_id,
        token: t.token,
    }
}

fn read_tokens(path: &Path) -> Result<Vec<SyncTokenInput>, CliError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path).map_err(CliError::Io)?;
    let file: SyncTokensFile = serde_json::from_str(&raw)
        .map_err(|e| CliError::Usage(format!("{}: {e}", path.display())))?;
    Ok(file.tokens.into_iter().map(token_to_input).collect())
}

fn write_tokens(path: &Path, tokens: &[SyncToken]) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(CliError::Io)?;
    }
    let file = SyncTokensFile { tokens: tokens.to_vec() };
    std::fs::write(path, serde_json::to_string_pretty(&file)?).map_err(CliError::Io)
}

/// Run an incremental sync: reads tokens from cache or tokens file, calls the API,
/// writes updated tokens and cache. Returns the sync response.
pub(crate) async fn do_sync(
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
) -> Result<IncrementalSyncResponse, CliError> {
    // Prefer sync tokens from the cache, fall back to the tokens file.
    let tokens = cache::read_cache(config)
        .map(|c| c.sync_tokens.into_iter().map(token_to_input).collect())
        .unwrap_or_else(|| read_tokens(&config.tokens_file()).unwrap_or_default());

    let req = IncrementalSyncRequest { sync_tokens: tokens, metadata: None };
    let resp = incremental_sync(client, &req).await.map_err(CliError::Api)?;

    // Write tokens file (backward compat) and cache.
    write_tokens(&config.tokens_file(), &resp.sync_tokens)?;
    let cache_data = CacheFile {
        updated_at: chrono::Utc::now().timestamp_millis(),
        sync_tokens: resp.sync_tokens.clone(),
        calendars: resp.calendars.clone(),
        events: resp.events.clone(),
    };
    cache::write_cache(config, &cache_data)?;
    Ok(resp)
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Authenticate with Notion Calendar.
    #[command(subcommand)]
    Auth(auth::AuthCmd),

    /// Create, list, update, and delete calendar events.
    #[command(subcommand)]
    Events(events::EventsCmd),

    /// List calendars and color palettes.
    #[command(subcommand)]
    Calendars(calendars::CalCmd),

    /// Manage scheduling holds (availability blocks and scheduling links).
    #[command(subcommand)]
    Holds(holds::HoldsCmd),

    /// Fetch incremental changes since last sync.
    Sync {
        /// Path to sync tokens file (default: from config).
        #[arg(long)]
        tokens_file: Option<std::path::PathBuf>,
    },

    /// Show the current authenticated user.
    Whoami,

    /// Manage connected calendar accounts.
    #[command(subcommand)]
    Accounts(user::AccountsCmd),

    /// View user preferences.
    #[command(subcommand)]
    Preferences(user::PreferencesCmd),

    /// List contacts from connected accounts.
    Contacts,
}

pub async fn run_command(cli: &Cli, config: &AppConfig) -> Result<(), CliError> {
    match &cli.command {
        Command::Auth(c) => auth::run(cli, config, c).await,
        Command::Events(c) => events::run(cli, config, c).await,
        Command::Calendars(c) => calendars::run(cli, config, c).await,
        Command::Holds(c) => holds::run(cli, config, c).await,
        Command::Sync { tokens_file } => {
            let client = context::authenticated_client(config)?;
            if let Some(path) = tokens_file.as_deref() {
                // Explicit tokens file: use legacy path (no cache).
                let tokens = read_tokens(path)?;
                let req = IncrementalSyncRequest { sync_tokens: tokens, metadata: None };
                let resp = incremental_sync(&client, &req).await.map_err(CliError::Api)?;
                write_tokens(path, &resp.sync_tokens)?;
                eprintln!("Wrote {} sync token(s) to {}.", resp.sync_tokens.len(), path.display());
                return print_json(cli, &resp);
            }
            let resp = do_sync(config, &client).await?;
            eprintln!(
                "Synced: {} calendar(s), {} event(s), {} token(s).",
                resp.calendars.len(), resp.events.len(), resp.sync_tokens.len(),
            );
            print_json(cli, &resp)
        }
        Command::Whoami => user::whoami(cli, config).await,
        Command::Accounts(c) => user::accounts(cli, config, c).await,
        Command::Preferences(c) => user::preferences(cli, config, c).await,
        Command::Contacts => {
            contacts::run(cli, config).await?;
            if !cli.json {
                hint(&["ncal events list  — list upcoming events"]);
            }
            Ok(())
        }
    }
}
