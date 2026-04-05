use clap::Subcommand;

use ncal_api::client::NotionCalendarClient;
use ncal_api::endpoints::{
    create_event, delete_events, get_calendar_lists, get_event, get_events, get_user,
    update_events, CalendarListQuery, CalendarListResult, CreateEventMutation, CreateEventRequest,
    DeleteEventMutation, DeleteEventsRequest, GetEventQuery, GetEventRequest, GetEventsRequest,
    GetEventsResult, EventQuery, GetCalendarListsRequest, UpdateEventMutation,
    UpdateEventsRequest,
};
use ncal_api::types::Provider;

use super::auth::{build_client, resolve_credentials};
use crate::config::AppConfig;
use crate::output::{hint, parse_provider, print_event_detail, print_events, print_json, print_json_lines};
use crate::Cli;
use crate::CliError;

#[derive(Subcommand, Debug)]
pub enum EventsCmd {
    /// List events (defaults to upcoming; auto-resolves account and calendar).
    List {
        /// Account email or ID.
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        calendar: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        /// Start time — RFC3339 or epoch millis (default: now).
        #[arg(long)]
        from_time: Option<String>,
        /// End time — RFC3339 or epoch millis.
        #[arg(long)]
        to_time: Option<String>,
        /// Search query string.
        #[arg(long)]
        query: Option<String>,
        /// Max number of events (default: 25).
        #[arg(long, default_value = "25")]
        limit: Option<u32>,
        /// Show all events (not just upcoming).
        #[arg(long)]
        all: bool,
        /// Include deleted/cancelled events.
        #[arg(long)]
        include_deleted: bool,
    },
    /// Get a single event by ID.
    Get {
        /// Event ID.
        event_id: String,
        /// Account email or ID.
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        calendar: Option<String>,
        #[arg(long)]
        provider: Option<String>,
    },
    /// Create a new calendar event.
    Create {
        /// Account email or ID.
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        calendar: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        /// Event title.
        #[arg(long)]
        summary: String,
        /// Start time (RFC3339).
        #[arg(long)]
        start: String,
        /// End time (RFC3339).
        #[arg(long)]
        end: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        location: Option<String>,
    },
    /// Update fields on an existing event.
    Update {
        /// Event ID.
        event_id: String,
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        calendar: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        location: Option<String>,
    },
    /// Delete or cancel an event (soft-cancel by default).
    Delete {
        /// Event ID.
        event_id: String,
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        calendar: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        /// Hard-delete via `deleteEvents` instead of soft-cancel.
        #[arg(long)]
        hard: bool,
    },
}

pub async fn run(cli: &Cli, config: &AppConfig, cmd: &EventsCmd) -> Result<(), CliError> {
    let client = build_client(config, resolve_credentials(config)?)?;
    match cmd {
        EventsCmd::List { account, calendar, provider, from_time, to_time, query, limit, all, include_deleted } => {
            let (acct, cal, prov) = resolve_context(config, &client, account.as_deref(), calendar.as_deref(), provider.as_deref()).await?;
            // Default to "from now" unless --all or --from-time is set
            let time_min = match from_time.as_deref() {
                Some(s) => Some(parse_time_ms("from-time", s)?),
                None if *all => None,
                None => Some(chrono::Utc::now().timestamp_millis()),
            };
            let time_max = to_time.as_deref().map(|s| parse_time_ms("to-time", s)).transpose()?;
            let req = GetEventsRequest {
                queries: vec![EventQuery {
                    provider: prov,
                    account_id: acct,
                    calendar_id: cal,
                    time_min,
                    time_max,
                    single_events: Some(true),
                    show_deleted: Some(*include_deleted),
                    query: query.clone(),
                    user_time_zone: config.defaults.timezone.clone(),
                    limit: *limit,
                }],
            };
            let results = get_events(&client, &req).await.map_err(CliError::Api)?;
            if cli.json {
                return print_json_lines(cli, &results);
            }
            let mut events = Vec::new();
            for r in &results {
                match r {
                    GetEventsResult::Ok(ok) => events.extend_from_slice(&ok.events),
                    GetEventsResult::Err(err) => eprintln!("warning: {}", err.error_message),
                }
            }
            print_events(cli, &events)?;
            if events.is_empty() && !cli.json {
                hint(&[
                    "ncal events list --from-time <RFC3339>  — try a different time range",
                    "ncal calendars list                     — check available calendars",
                ]);
            }
            Ok(())
        }
        EventsCmd::Get { event_id, account, calendar, provider } => {
            let (acct, cal, prov) = resolve_context(config, &client, account.as_deref(), calendar.as_deref(), provider.as_deref()).await?;
            let req = GetEventRequest {
                query: GetEventQuery {
                    provider: prov,
                    account_id: acct,
                    calendar_id: cal,
                    event_id: event_id.clone(),
                    user_time_zone: config.defaults.timezone.clone(),
                },
            };
            let resp = get_event(&client, &req).await.map_err(CliError::Api)?;
            print_event_detail(cli, &resp.event)?;
            if !cli.json {
                hint(&[
                    &format!("ncal events update {event_id} --summary <TEXT>  — update this event"),
                    &format!("ncal events delete {event_id}                  — cancel this event"),
                ]);
            }
            Ok(())
        }
        EventsCmd::Create { account, calendar, provider, summary, start, end, description, location } => {
            let (acct, cal, prov) = resolve_context(config, &client, account.as_deref(), calendar.as_deref(), provider.as_deref()).await?;
            let tz = config.defaults.timezone.as_deref();
            let mut event_data = serde_json::json!({
                "summary": summary,
                "start": event_date_json(start, tz),
                "end": event_date_json(end, tz),
            });
            let obj = event_data.as_object_mut().unwrap();
            if let Some(d) = description { obj.insert("description".into(), serde_json::json!(d)); }
            if let Some(l) = location { obj.insert("location".into(), serde_json::json!(l)); }
            let req = CreateEventRequest {
                mutation: CreateEventMutation {
                    provider: prov,
                    account_id: acct,
                    calendar_id: cal,
                    event_data,
                    send_updates: None,
                },
            };
            let event = create_event(&client, &req).await.map_err(CliError::Api)?;
            print_event_detail(cli, &event)?;
            if !cli.json {
                eprintln!("\nCreated.");
                hint(&[
                    &format!("ncal events get {}  — view this event", event.id),
                    "ncal events list                  — list events",
                ]);
            }
            Ok(())
        }
        EventsCmd::Update { event_id, account, calendar, provider, summary, start, end, description, location } => {
            let (acct, cal, prov) = resolve_context(config, &client, account.as_deref(), calendar.as_deref(), provider.as_deref()).await?;
            let tz = config.defaults.timezone.as_deref();
            let mut partial = serde_json::Map::new();
            if let Some(s) = summary { partial.insert("summary".into(), serde_json::json!(s)); }
            if let Some(s) = start { partial.insert("start".into(), event_date_json(s, tz)); }
            if let Some(s) = end { partial.insert("end".into(), event_date_json(s, tz)); }
            if let Some(d) = description { partial.insert("description".into(), serde_json::json!(d)); }
            if let Some(l) = location { partial.insert("location".into(), serde_json::json!(l)); }
            if partial.is_empty() {
                return Err(CliError::Usage("provide at least one of --summary --start --end --description --location".into()));
            }
            let req = UpdateEventsRequest {
                mutations: vec![UpdateEventMutation {
                    provider: prov,
                    account_id: acct,
                    event_id: event_id.clone(),
                    calendar_id: cal,
                    event_data: serde_json::Value::Object(partial),
                    user_time_zone: config.defaults.timezone.clone(),
                    send_updates: None,
                }],
            };
            let resp = update_events(&client, &req).await.map_err(CliError::Api)?;
            if cli.json {
                return print_json(cli, &resp);
            }
            for entry in &resp {
                if let Some(e) = &entry.event {
                    print_event_detail(cli, e)?;
                }
                if let Some(err) = &entry.error_message {
                    eprintln!("error: {err}");
                }
            }
            if !cli.json {
                eprintln!("\nUpdated.");
            }
            Ok(())
        }
        EventsCmd::Delete { event_id, account, calendar, provider, hard } => {
            let (acct, cal, prov) = resolve_context(config, &client, account.as_deref(), calendar.as_deref(), provider.as_deref()).await?;
            if *hard {
                let req = DeleteEventsRequest {
                    mutations: vec![DeleteEventMutation {
                        provider: prov,
                        account_id: acct,
                        calendar_id: cal,
                        event_id: event_id.clone(),
                        send_updates: None,
                    }],
                };
                let resp = delete_events(&client, &req).await.map_err(CliError::Api)?;
                if cli.json {
                    return print_json(cli, &resp);
                }
                eprintln!("Deleted (hard).");
                return Ok(());
            }
            let req = UpdateEventsRequest {
                mutations: vec![UpdateEventMutation {
                    provider: prov,
                    account_id: acct,
                    event_id: event_id.clone(),
                    calendar_id: cal,
                    event_data: serde_json::json!({ "status": "cancelled", "responseStatus": "declined" }),
                    user_time_zone: config.defaults.timezone.clone(),
                    send_updates: None,
                }],
            };
            let resp = update_events(&client, &req).await.map_err(CliError::Api)?;
            if cli.json {
                return print_json(cli, &resp);
            }
            eprintln!("Cancelled.");
            hint(&["ncal events list  — verify the cancellation"]);
            Ok(())
        }
    }
}

// ── Auto-resolution ─────────────────────────────────────────────────────────

async fn resolve_context(
    config: &AppConfig,
    client: &NotionCalendarClient,
    account: Option<&str>,
    calendar: Option<&str>,
    provider: Option<&str>,
) -> Result<(String, String, Provider), CliError> {
    let account_hint = account.or(config.defaults.account.as_deref());
    let (account_id, resolved_provider) = resolve_account(client, account_hint).await?;
    let prov = provider.map(parse_provider).transpose()?.unwrap_or(resolved_provider);

    let calendar_id = match calendar.or(config.defaults.calendar.as_deref()) {
        Some(c) => c.to_string(),
        None => auto_resolve_calendar(client, &account_id, prov).await?,
    };

    Ok((account_id, calendar_id, prov))
}

/// Resolve an account hint (email, UUID, or None) to (account_id, provider).
async fn resolve_account(
    client: &NotionCalendarClient,
    hint: Option<&str>,
) -> Result<(String, Provider), CliError> {
    let user = get_user(client).await.map_err(CliError::Api)?;
    let accounts = user.accounts.unwrap_or_default();

    if let Some(q) = hint {
        // Match by ID or email (case-insensitive)
        let q_lower = q.to_ascii_lowercase();
        if let Some(found) = accounts.iter().find(|a| {
            a.id == q || a.email.as_deref().map(|e| e.to_ascii_lowercase()) == Some(q_lower.clone())
        }) {
            let prov = found.provider_name.unwrap_or(Provider::Google);
            return Ok((found.id.clone(), prov));
        }
        return Err(CliError::Usage(format!(
            "no account matching {:?}; available accounts:\n{}",
            q,
            format_account_list(&accounts),
        )));
    }

    // No hint — auto-resolve
    match accounts.as_slice() {
        [] => Err(CliError::Usage(
            "no accounts connected; add one in the Notion Calendar app".into(),
        )),
        [one] => {
            eprintln!(
                "auto: using account {} ({})",
                one.email.as_deref().unwrap_or("?"),
                one.provider_name.map(|p| p.to_string()).unwrap_or_default()
            );
            Ok((one.id.clone(), one.provider_name.unwrap_or(Provider::Google)))
        }
        many => {
            Err(CliError::Usage(format!(
                "multiple accounts found; specify --account <EMAIL>:\n{}\nor set defaults.account in your config file",
                format_account_list(many),
            )))
        }
    }
}

fn format_account_list(accounts: &[ncal_api::types::Account]) -> String {
    accounts
        .iter()
        .map(|a| {
            format!(
                "  --account {}  # {} ({})",
                a.email.as_deref().unwrap_or(&a.id),
                a.display_name.as_deref().unwrap_or(""),
                a.provider_name.map(|p| p.to_string()).unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn auto_resolve_calendar(
    client: &NotionCalendarClient,
    account_id: &str,
    provider: Provider,
) -> Result<String, CliError> {
    let req = GetCalendarListsRequest {
        queries: vec![CalendarListQuery {
            provider,
            account_id: account_id.to_string(),
        }],
    };
    let results = get_calendar_lists(client, &req).await.map_err(CliError::Api)?;
    let calendars: Vec<_> = results
        .into_iter()
        .flat_map(|r| match r {
            CalendarListResult::Ok(ok) => ok.calendars,
            CalendarListResult::Err(err) => {
                eprintln!("warning: calendar list error: {}", err.error_message);
                vec![]
            }
        })
        .collect();

    // Prefer the primary calendar
    if let Some(primary) = calendars.iter().find(|c| c.primary == Some(true)) {
        eprintln!(
            "auto: using calendar {:?} (primary)",
            primary.summary.as_deref().unwrap_or(&primary.id)
        );
        return Ok(primary.id.clone());
    }

    match calendars.as_slice() {
        [] => Err(CliError::Usage(
            "no calendars found for this account".into(),
        )),
        [one] => {
            eprintln!(
                "auto: using calendar {:?}",
                one.summary.as_deref().unwrap_or(&one.id)
            );
            Ok(one.id.clone())
        }
        many => {
            let mut msg = String::from("multiple calendars found; specify --calendar <ID>:\n");
            for c in many {
                msg.push_str(&format!(
                    "  --calendar {}  # {}{}\n",
                    c.id,
                    c.summary.as_deref().unwrap_or(""),
                    if c.primary == Some(true) { " (primary)" } else { "" },
                ));
            }
            msg.push_str("\nor set defaults.calendar in your config file");
            Err(CliError::Usage(msg))
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn parse_time_ms(label: &str, s: &str) -> Result<i64, CliError> {
    if let Ok(ms) = s.parse::<i64>() {
        return Ok(ms);
    }
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.timestamp_millis())
        .map_err(|_| CliError::Usage(format!("--{label}: expected epoch millis or RFC3339, got {s:?}")))
}

fn event_date_json(datetime_rfc3339: &str, tz: Option<&str>) -> serde_json::Value {
    match tz {
        Some(z) => serde_json::json!({ "dateTime": datetime_rfc3339, "timeZone": z }),
        None => serde_json::json!({ "dateTime": datetime_rfc3339 }),
    }
}
