use clap::Subcommand;

use ncal_api::endpoints::{
    create_hold, delete_hold, get_hold_alias_available, get_holds, CheckAliasRequest,
    CreateHoldRequest, DeleteHoldRequest,
};
use ncal_api::types::TimeRange;

use super::context::authenticated_client;
use crate::config::AppConfig;
use crate::output::{hint, print_hold_detail, print_holds, print_json};
use crate::Cli;
use crate::CliError;

#[derive(Subcommand, Debug)]
pub enum HoldsCmd {
    /// List all scheduling holds.
    List,
    /// Show details of a hold by ID.
    Get {
        /// Hold ID.
        hold_id: String,
    },
    /// Create a new scheduling hold (availability block).
    Create {
        /// Hold title.
        #[arg(long)]
        title: String,
        /// URL-safe alias for the scheduling link.
        #[arg(long)]
        alias: String,
        /// Duration in minutes.
        #[arg(long)]
        duration: u32,
        /// IANA timezone (default: from config or UTC).
        #[arg(long)]
        timezone: Option<String>,
        /// Destination account ID (googleAccountId field).
        #[arg(long)]
        account: String,
        /// Destination calendar ID (googleCalendarId field).
        #[arg(long)]
        calendar: String,
        /// Hold type (default: recurring).
        #[arg(long, default_value = "recurring")]
        hold_type: String,
        /// Description.
        #[arg(long)]
        description: Option<String>,
        /// Minimum lead time in minutes.
        #[arg(long)]
        min_lead_time: Option<u64>,
        /// Maximum lead time in minutes.
        #[arg(long)]
        max_lead_time: Option<u64>,
    },
    /// Delete a scheduling hold.
    Delete {
        /// Hold ID.
        hold_id: String,
    },
    /// Check if a scheduling link alias is available.
    CheckAlias {
        /// The alias to check.
        alias: String,
    },
}

pub async fn run(cli: &Cli, config: &AppConfig, cmd: &HoldsCmd) -> Result<(), CliError> {
    let client = authenticated_client(config)?;
    match cmd {
        HoldsCmd::List => list(cli, &client).await,
        HoldsCmd::Get { hold_id } => get(cli, &client, hold_id).await,
        HoldsCmd::Create {
            title,
            alias,
            duration,
            timezone,
            account,
            calendar,
            hold_type,
            description,
            min_lead_time,
            max_lead_time,
        } => {
            create(
                cli,
                config,
                &client,
                CreateHoldArgs {
                    title,
                    alias,
                    duration: *duration,
                    timezone: timezone.as_deref(),
                    account,
                    calendar,
                    hold_type,
                    description: description.as_deref(),
                    min_lead_time: *min_lead_time,
                    max_lead_time: *max_lead_time,
                },
            )
            .await
        }
        HoldsCmd::Delete { hold_id } => delete(cli, &client, hold_id).await,
        HoldsCmd::CheckAlias { alias } => check_alias(cli, &client, alias).await,
    }
}

async fn list(cli: &Cli, client: &ncal_api::client::NotionCalendarClient) -> Result<(), CliError> {
    let resp = get_holds(client).await.map_err(CliError::Api)?;
    if cli.json {
        return print_json(cli, &resp.holds);
    }
    print_holds(cli, &resp.holds)?;
    hint(&[
        "ncal holds get <ID>              — view hold details",
        "ncal holds create --title <T> …  — create a new hold",
    ]);
    Ok(())
}

async fn get(
    cli: &Cli,
    client: &ncal_api::client::NotionCalendarClient,
    hold_id: &str,
) -> Result<(), CliError> {
    let resp = get_holds(client).await.map_err(CliError::Api)?;
    let hold = resp.holds.iter().find(|h| h.id == hold_id).ok_or_else(|| {
        CliError::Usage(format!(
            "no hold with ID {hold_id:?}; use `ncal holds list` to see all"
        ))
    })?;
    print_hold_detail(cli, hold)?;
    if !cli.json {
        hint(&[
            &format!("ncal holds delete {hold_id}  — delete this hold"),
            "ncal holds list              — list all holds",
        ]);
    }
    Ok(())
}

struct CreateHoldArgs<'a> {
    title: &'a str,
    alias: &'a str,
    duration: u32,
    timezone: Option<&'a str>,
    account: &'a str,
    calendar: &'a str,
    hold_type: &'a str,
    description: Option<&'a str>,
    min_lead_time: Option<u64>,
    max_lead_time: Option<u64>,
}

async fn create(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    args: CreateHoldArgs<'_>,
) -> Result<(), CliError> {
    let tz = args
        .timezone
        .map(String::from)
        .or_else(|| config.defaults.timezone.clone())
        .unwrap_or_else(|| "UTC".to_string());
    let id = uuid::Uuid::new_v4().to_string();
    let req = CreateHoldRequest {
        id: id.clone(),
        alias: args.alias.to_string(),
        hold_type: args.hold_type.to_string(),
        status: "activeBookable".to_string(),
        user_primary_time_zone: tz.clone(),
        time_zone: tz,
        duration: Some(args.duration),
        time_ranges: Some(Vec::<TimeRange>::new()),
        title: args.title.to_string(),
        description: args.description.map(String::from),
        conferencing_provider_name: None,
        conferencing_account_id: None,
        google_account_id: args.account.to_string(),
        google_calendar_id: args.calendar.to_string(),
        conflict_free_resources: None,
        min_lead_time: args.min_lead_time,
        max_lead_time: args.max_lead_time,
        expiration_date: None,
    };
    let resp = create_hold(client, &req).await.map_err(CliError::Api)?;
    if cli.json {
        return print_json(
            cli,
            &serde_json::json!({ "id": id, "success": resp.success }),
        );
    }
    eprintln!("Created hold {id}.");
    hint(&[
        &format!("ncal holds get {id}  — view the hold"),
        "ncal holds list           — list all holds",
    ]);
    Ok(())
}

async fn delete(
    cli: &Cli,
    client: &ncal_api::client::NotionCalendarClient,
    hold_id: &str,
) -> Result<(), CliError> {
    let req = DeleteHoldRequest {
        hold_id: hold_id.to_string(),
    };
    let resp = delete_hold(client, &req).await.map_err(CliError::Api)?;
    if cli.json {
        return print_json(cli, &resp);
    }
    eprintln!("Deleted hold {hold_id}.");
    Ok(())
}

async fn check_alias(
    cli: &Cli,
    client: &ncal_api::client::NotionCalendarClient,
    alias: &str,
) -> Result<(), CliError> {
    let req = CheckAliasRequest {
        alias: alias.to_string(),
    };
    let resp = get_hold_alias_available(client, &req)
        .await
        .map_err(CliError::Api)?;
    if cli.json {
        return print_json(cli, &resp);
    }
    if resp.available {
        eprintln!("Alias {alias:?} is available.");
    } else {
        eprintln!("Alias {alias:?} is taken.");
    }
    Ok(())
}
