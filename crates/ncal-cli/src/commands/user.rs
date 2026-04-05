use clap::Subcommand;

use ncal_api::endpoints::{get_user, get_user_preferences, remove_account, update_primary_account};

use super::auth::{build_client, resolve_credentials};
use crate::config::AppConfig;
use crate::output::{hint, print_accounts, print_json, print_user};
use crate::Cli;
use crate::CliError;

#[derive(Subcommand, Debug)]
pub enum AccountsCmd {
    /// List connected calendar accounts.
    List,
    /// Set the primary calendar account.
    SetPrimary {
        /// Account ID.
        id: String,
    },
    /// Disconnect a calendar account.
    Remove {
        /// Account ID.
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum PreferencesCmd {
    /// Show current user preferences.
    Get,
}

pub async fn whoami(cli: &Cli, config: &AppConfig) -> Result<(), CliError> {
    let client = build_client(config, resolve_credentials(config)?)?;
    let user = get_user(&client).await.map_err(CliError::Api)?;
    print_user(cli, &user)?;
    if !cli.json {
        hint(&[
            "ncal accounts list   — list connected calendar accounts",
            "ncal calendars list  — list available calendars",
            "ncal events list     — list upcoming events",
        ]);
    }
    Ok(())
}

pub async fn accounts(cli: &Cli, config: &AppConfig, cmd: &AccountsCmd) -> Result<(), CliError> {
    let client = build_client(config, resolve_credentials(config)?)?;
    match cmd {
        AccountsCmd::List => {
            let accounts = get_user(&client)
                .await
                .map_err(CliError::Api)?
                .accounts
                .ok_or_else(|| CliError::Usage("no accounts on user object".into()))?;
            print_accounts(cli, &accounts)?;
            if !cli.json {
                hint(&[
                    "ncal calendars list --account <ID>  — list calendars for an account",
                    "ncal events list                    — list upcoming events",
                ]);
            }
            Ok(())
        }
        AccountsCmd::SetPrimary { id } => {
            let resp = update_primary_account(&client, id).await.map_err(CliError::Api)?;
            if cli.json {
                return print_json(cli, &resp);
            }
            eprintln!("Primary account set to {id}.");
            Ok(())
        }
        AccountsCmd::Remove { id } => {
            let resp = remove_account(&client, id).await.map_err(CliError::Api)?;
            if cli.json {
                return print_json(cli, &resp);
            }
            eprintln!("Account {id} removed.");
            Ok(())
        }
    }
}

pub async fn preferences(cli: &Cli, config: &AppConfig, cmd: &PreferencesCmd) -> Result<(), CliError> {
    let client = build_client(config, resolve_credentials(config)?)?;
    match cmd {
        PreferencesCmd::Get => print_json(cli, &get_user_preferences(&client).await.map_err(CliError::Api)?),
    }
}
