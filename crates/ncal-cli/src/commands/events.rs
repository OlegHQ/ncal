use clap::Subcommand;

use ncal_api::endpoints::{
    create_event, delete_events, get_event, get_events, update_events, CreateEventMutation,
    CreateEventRequest, DeleteEventMutation, DeleteEventsRequest, EventQuery, GetEventQuery,
    GetEventRequest, GetEventsRequest, GetEventsResult, UpdateEventMutation, UpdateEventsRequest,
};

use super::context::{
    authenticated_client, build_all_account_queries, parse_time_ms, resolve_context,
};
use super::do_sync;
use crate::cache;
use crate::config::AppConfig;
use crate::output::{
    event_sort_key, hint, print_event_detail, print_events, print_json, print_json_lines,
};
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
        /// Force a fresh sync (bypass cache).
        #[arg(long)]
        refresh: bool,
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
    let client = authenticated_client(config)?;
    match cmd {
        EventsCmd::List {
            account,
            calendar,
            provider,
            from_time,
            to_time,
            query,
            limit,
            all,
            include_deleted,
            refresh,
        } => {
            list(
                cli,
                config,
                &client,
                account.as_deref(),
                calendar.as_deref(),
                provider.as_deref(),
                from_time.as_deref(),
                to_time.as_deref(),
                query.as_deref(),
                *limit,
                *all,
                *include_deleted,
                *refresh,
            )
            .await
        }
        EventsCmd::Get {
            event_id,
            account,
            calendar,
            provider,
        } => {
            get(
                cli,
                config,
                &client,
                event_id,
                account.as_deref(),
                calendar.as_deref(),
                provider.as_deref(),
            )
            .await
        }
        EventsCmd::Create {
            account,
            calendar,
            provider,
            summary,
            start,
            end,
            description,
            location,
        } => {
            create(
                cli,
                config,
                &client,
                account.as_deref(),
                calendar.as_deref(),
                provider.as_deref(),
                summary,
                start,
                end,
                description.as_deref(),
                location.as_deref(),
            )
            .await
        }
        EventsCmd::Update {
            event_id,
            account,
            calendar,
            provider,
            summary,
            start,
            end,
            description,
            location,
        } => {
            update(
                cli,
                config,
                &client,
                event_id,
                account.as_deref(),
                calendar.as_deref(),
                provider.as_deref(),
                summary.as_deref(),
                start.as_deref(),
                end.as_deref(),
                description.as_deref(),
                location.as_deref(),
            )
            .await
        }
        EventsCmd::Delete {
            event_id,
            account,
            calendar,
            provider,
            hard,
        } => {
            delete(
                cli,
                config,
                &client,
                event_id,
                account.as_deref(),
                calendar.as_deref(),
                provider.as_deref(),
                *hard,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn list(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    account: Option<&str>,
    calendar: Option<&str>,
    provider: Option<&str>,
    from_time: Option<&str>,
    to_time: Option<&str>,
    query: Option<&str>,
    limit: Option<u32>,
    all: bool,
    include_deleted: bool,
    refresh: bool,
) -> Result<(), CliError> {
    let time_min = match from_time {
        Some(s) => Some(parse_time_ms("from-time", s)?),
        None if all => None,
        None => Some(chrono::Utc::now().timestamp_millis()),
    };
    let time_max = to_time.map(|s| parse_time_ms("to-time", s)).transpose()?;

    // Try cache first (all-accounts path only, no filters).
    let use_cache = !refresh
        && account.is_none()
        && calendar.is_none()
        && provider.is_none()
        && config.defaults.account.is_none()
        && config.defaults.calendar.is_none()
        && query.is_none();

    let events = if use_cache {
        if let Some(cached) = cache::read_fresh_cache(config) {
            eprintln!("(using cache)");
            cached.events
        } else {
            eprintln!("(syncing…)");
            do_sync(config, client).await?.events
        }
    } else {
        // Filtered path: use getEvents API with specific targets.
        let targets = if account.is_some() || calendar.is_some() || provider.is_some() {
            let ctx = resolve_context(config, client, account, calendar, provider).await?;
            vec![ctx]
        } else if config.defaults.account.is_some() || config.defaults.calendar.is_some() {
            let ctx = resolve_context(config, client, None, None, None).await?;
            vec![ctx]
        } else {
            build_all_account_queries(client).await?
        };

        let queries = targets
            .into_iter()
            .map(|(acct, cal, prov)| EventQuery {
                provider: prov,
                account_id: acct,
                calendar_id: cal,
                time_min,
                time_max,
                single_events: Some(true),
                show_deleted: Some(include_deleted),
                query: query.map(String::from),
                user_time_zone: config.defaults.timezone.clone(),
                limit,
            })
            .collect();

        let req = GetEventsRequest { queries };
        let results = get_events(client, &req).await.map_err(CliError::Api)?;
        if cli.json {
            return print_json_lines(cli, &results);
        }
        let mut evts = Vec::new();
        for r in &results {
            match r {
                GetEventsResult::Ok(ok) => evts.extend_from_slice(&ok.events),
                GetEventsResult::Err(err) => eprintln!("warning: {}", err.error_message),
            }
        }
        evts
    };

    // Client-side filtering for cached events.
    let filtered: Vec<&ncal_api::types::Event> = events
        .iter()
        .filter(|e| {
            if !include_deleted && e.status == Some(ncal_api::types::EventStatus::Cancelled) {
                return false;
            }
            let key = event_sort_key(e);
            if let Some(min) = time_min {
                if key < min {
                    return false;
                }
            }
            if let Some(max) = time_max {
                if key > max {
                    return false;
                }
            }
            true
        })
        .collect();

    // Apply limit.
    let limited: Vec<ncal_api::types::Event> = if let Some(n) = limit {
        let mut sorted = filtered;
        sorted.sort_by_key(|e| event_sort_key(e));
        sorted.into_iter().take(n as usize).cloned().collect()
    } else {
        filtered.into_iter().cloned().collect()
    };

    if cli.json {
        return print_json(cli, &limited);
    }
    print_events(cli, &limited)?;
    if limited.is_empty() {
        hint(&[
            "ncal events list --from-time <RFC3339>  — try a different time range",
            "ncal events list --refresh              — force a fresh sync",
            "ncal calendars list                     — check available calendars",
        ]);
    }
    Ok(())
}

async fn get(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    event_id: &str,
    account: Option<&str>,
    calendar: Option<&str>,
    provider: Option<&str>,
) -> Result<(), CliError> {
    let (acct, cal, prov) = resolve_context(config, client, account, calendar, provider).await?;
    let req = GetEventRequest {
        query: GetEventQuery {
            provider: prov,
            account_id: acct,
            calendar_id: cal,
            event_id: event_id.to_string(),
            user_time_zone: config.defaults.timezone.clone(),
        },
    };
    let resp = get_event(client, &req).await.map_err(CliError::Api)?;
    print_event_detail(cli, &resp.event)?;
    if !cli.json {
        hint(&[
            &format!("ncal events update {event_id} --summary <TEXT>  — update this event"),
            &format!("ncal events delete {event_id}                  — cancel this event"),
        ]);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    account: Option<&str>,
    calendar: Option<&str>,
    provider: Option<&str>,
    summary: &str,
    start: &str,
    end: &str,
    description: Option<&str>,
    location: Option<&str>,
) -> Result<(), CliError> {
    let (acct, cal, prov) = resolve_context(config, client, account, calendar, provider).await?;
    let tz = config.defaults.timezone.as_deref();
    let mut event_data = serde_json::json!({
        "summary": summary,
        "start": event_date_json(start, tz),
        "end": event_date_json(end, tz),
    });
    let obj = event_data.as_object_mut().unwrap();
    if let Some(d) = description {
        obj.insert("description".into(), serde_json::json!(d));
    }
    if let Some(l) = location {
        obj.insert("location".into(), serde_json::json!(l));
    }
    let req = CreateEventRequest {
        mutation: CreateEventMutation {
            provider: prov,
            account_id: acct,
            calendar_id: cal,
            event_data,
            send_updates: None,
        },
    };
    let event = create_event(client, &req).await.map_err(CliError::Api)?;
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

#[allow(clippy::too_many_arguments)]
async fn update(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    event_id: &str,
    account: Option<&str>,
    calendar: Option<&str>,
    provider: Option<&str>,
    summary: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    description: Option<&str>,
    location: Option<&str>,
) -> Result<(), CliError> {
    let (acct, cal, prov) = resolve_context(config, client, account, calendar, provider).await?;
    let tz = config.defaults.timezone.as_deref();
    let mut partial = serde_json::Map::new();
    if let Some(s) = summary {
        partial.insert("summary".into(), serde_json::json!(s));
    }
    if let Some(s) = start {
        partial.insert("start".into(), event_date_json(s, tz));
    }
    if let Some(s) = end {
        partial.insert("end".into(), event_date_json(s, tz));
    }
    if let Some(d) = description {
        partial.insert("description".into(), serde_json::json!(d));
    }
    if let Some(l) = location {
        partial.insert("location".into(), serde_json::json!(l));
    }
    if partial.is_empty() {
        return Err(CliError::Usage(
            "provide at least one of --summary --start --end --description --location".into(),
        ));
    }
    let req = UpdateEventsRequest {
        mutations: vec![UpdateEventMutation {
            provider: prov,
            account_id: acct,
            event_id: event_id.to_string(),
            calendar_id: cal,
            event_data: serde_json::Value::Object(partial),
            user_time_zone: config.defaults.timezone.clone(),
            send_updates: None,
        }],
    };
    let resp = update_events(client, &req).await.map_err(CliError::Api)?;
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
    eprintln!("\nUpdated.");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn delete(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    event_id: &str,
    account: Option<&str>,
    calendar: Option<&str>,
    provider: Option<&str>,
    hard: bool,
) -> Result<(), CliError> {
    let (acct, cal, prov) = resolve_context(config, client, account, calendar, provider).await?;
    if hard {
        let req = DeleteEventsRequest {
            mutations: vec![DeleteEventMutation {
                provider: prov,
                account_id: acct,
                calendar_id: cal,
                event_id: event_id.to_string(),
                send_updates: None,
            }],
        };
        let resp = delete_events(client, &req).await.map_err(CliError::Api)?;
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
            event_id: event_id.to_string(),
            calendar_id: cal,
            event_data: serde_json::json!({ "status": "cancelled", "responseStatus": "declined" }),
            user_time_zone: config.defaults.timezone.clone(),
            send_updates: None,
        }],
    };
    let resp = update_events(client, &req).await.map_err(CliError::Api)?;
    if cli.json {
        return print_json(cli, &resp);
    }
    eprintln!("Cancelled.");
    hint(&["ncal events list  — verify the cancellation"]);
    Ok(())
}

fn event_date_json(datetime_rfc3339: &str, tz: Option<&str>) -> serde_json::Value {
    match tz {
        Some(z) => serde_json::json!({ "dateTime": datetime_rfc3339, "timeZone": z }),
        None => serde_json::json!({ "dateTime": datetime_rfc3339 }),
    }
}
