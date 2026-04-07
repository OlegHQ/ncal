#![allow(clippy::result_large_err)]

mod cache;
mod commands;
mod config;
mod output;

use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::commands::run_command;
use crate::config::AppConfig;

/// Notion Calendar — agent-oriented CLI (unpublished API surface).
#[derive(Parser, Debug)]
#[command(name = "ncal", version, about)]
pub struct Cli {
    /// Machine-readable JSON on stdout (human text otherwise).
    #[arg(long, global = true)]
    pub json: bool,

    /// Config file (default: platform config dir / ncal / config.toml). Overrides `NCAL_CONFIG`.
    #[arg(long, global = true, env = "NCAL_CONFIG")]
    pub config: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub command: commands::Command,
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Auth(#[from] ncal_api::error::AuthError),

    #[error(transparent)]
    Api(#[from] ncal_api::error::ApiError),

    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn exit_code_for(err: &CliError) -> u8 {
    match err {
        CliError::Usage(_) => 2,
        CliError::Auth(_) | CliError::Api(ncal_api::error::ApiError::InvalidToken) => 3,
        CliError::Api(ncal_api::error::ApiError::RefreshCooldown) => 3,
        CliError::Api(ncal_api::error::ApiError::Network(_)) => 4,
        CliError::Api(ncal_api::error::ApiError::Server { .. }) => 4,
        CliError::Api(_) => 4,
        CliError::Io(_) => 4,
        CliError::Json(_) => 4,
    }
}

fn error_hint(e: &CliError) -> Option<&'static str> {
    match e {
        CliError::Auth(ncal_api::error::AuthError::NoCredentials) => {
            Some("run `ncal auth from-app` to import from the desktop app, or `ncal auth login` to sign in")
        }
        CliError::Auth(ncal_api::error::AuthError::LevelDb { .. }) => {
            Some("is Notion Calendar installed? check that the desktop app has been opened at least once")
        }
        CliError::Auth(ncal_api::error::AuthError::Json { .. }) => {
            Some("the desktop app's stored credentials may be corrupt or from an incompatible version; try logging out and back in to the desktop app")
        }
        CliError::Auth(ncal_api::error::AuthError::Keychain { .. }) => {
            Some("check macOS Keychain Access for the 'notion-calendar-cli' entry, or re-run `ncal auth from-app`")
        }
        CliError::Api(ncal_api::error::ApiError::InvalidToken) => {
            Some("session expired; run `ncal auth refresh` or `ncal auth from-app`")
        }
        CliError::Api(ncal_api::error::ApiError::RefreshCooldown) => {
            Some("too many refresh attempts; wait 30 seconds and retry")
        }
        CliError::Api(ncal_api::error::ApiError::Network(_)) => {
            Some("check your internet connection and that calendar-api.notion.so is reachable")
        }
        _ => None,
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let config = match AppConfig::load(cli.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("config error: {e}");
            return ExitCode::from(2u8);
        }
    };

    match run_command(&cli, &config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let code = exit_code_for(&e);
            if cli.json {
                let _ = serde_json::to_writer(std::io::stderr(), &serde_json::json!({
                    "error": format!("{e}"),
                    "code": code,
                    "hint": error_hint(&e),
                }));
                eprintln!();
            } else {
                eprintln!("error: {e}");
                if let Some(hint) = error_hint(&e) {
                    eprintln!("hint: {hint}");
                }
            }
            ExitCode::from(code)
        }
    }
}
