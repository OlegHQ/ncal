use clap::Subcommand;

use ncal_api::endpoints::{
    get_calendar_lists, get_colors, CalendarListQuery, CalendarListResult, GetCalendarListsRequest,
};
use ncal_api::types::User;

use super::auth::{build_client, resolve_credentials};
use crate::config::AppConfig;
use crate::output::{hint, parse_provider, print_calendars, print_json};
use crate::Cli;
use crate::CliError;

#[derive(Subcommand, Debug)]
pub enum CalCmd {
    /// List calendars for all connected accounts (or filter with --account).
    List {
        /// Filter to a specific account ID.
        #[arg(long)]
        account: Option<String>,
        /// Override provider (default: from account).
        #[arg(long)]
        provider: Option<String>,
    },
    /// Show the named color palette.
    Colors,
}

pub async fn run(cli: &Cli, config: &AppConfig, cmd: &CalCmd) -> Result<(), CliError> {
    match cmd {
        CalCmd::List { account, provider } => {
            let client = build_client(config, resolve_credentials(config)?)?;
            let user: User = ncal_api::endpoints::get_user(&client).await.map_err(CliError::Api)?;
            let accounts = user.accounts.as_ref().ok_or_else(|| {
                CliError::Usage("no accounts connected; add one in the Notion Calendar app".into())
            })?;

            let mut queries = Vec::new();
            for acct in accounts {
                if let Some(f) = account.as_deref() {
                    if acct.id != f { continue; }
                }
                let pn = acct.provider_name.ok_or_else(|| {
                    CliError::Usage(format!("account {} has no provider", acct.id))
                })?;
                let prov = provider.as_deref().map(parse_provider).transpose()?.unwrap_or(pn);
                queries.push(CalendarListQuery { provider: prov, account_id: acct.id.clone() });
            }

            if queries.is_empty() {
                return Err(CliError::Usage(
                    "no calendar accounts matched (check --account or sign in)".into(),
                ));
            }

            let results = get_calendar_lists(&client, &GetCalendarListsRequest { queries })
                .await
                .map_err(CliError::Api)?;

            if cli.json {
                return print_json(cli, &results);
            }

            let mut calendars = Vec::new();
            for r in results {
                match r {
                    CalendarListResult::Ok(ok) => calendars.extend(ok.calendars),
                    CalendarListResult::Err(err) => eprintln!("warning: {}", err.error_message),
                }
            }
            print_calendars(cli, &calendars)?;
            if !cli.json {
                hint(&[
                    "ncal events list --calendar <ID>  — list events in a calendar",
                    "ncal calendars colors             — show the color palette",
                ]);
            }
            Ok(())
        }
        CalCmd::Colors => {
            let client = build_client(config, resolve_credentials(config)?)?;
            print_json(cli, &get_colors(&client).await.map_err(CliError::Api)?)
        }
    }
}
