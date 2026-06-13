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
                ListEvents {
                    account: account.as_deref(),
                    calendar: calendar.as_deref(),
                    provider: provider.as_deref(),
                    from_time: from_time.as_deref(),
                    to_time: to_time.as_deref(),
                    query: query.as_deref(),
                    limit: *limit,
                    all: *all,
                    include_deleted: *include_deleted,
                    refresh: *refresh,
                },
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
                CreateEventArgs {
                    account: account.as_deref(),
                    calendar: calendar.as_deref(),
                    provider: provider.as_deref(),
                    summary,
                    start,
                    end,
                    description: description.as_deref(),
                    location: location.as_deref(),
                },
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
                UpdateEventArgs {
                    event_id,
                    account: account.as_deref(),
                    calendar: calendar.as_deref(),
                    provider: provider.as_deref(),
                    summary: summary.as_deref(),
                    start: start.as_deref(),
                    end: end.as_deref(),
                    description: description.as_deref(),
                    location: location.as_deref(),
                },
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
                DeleteEventArgs {
                    event_id,
                    account: account.as_deref(),
                    calendar: calendar.as_deref(),
                    provider: provider.as_deref(),
                    hard: *hard,
                },
            )
            .await
        }
    }
}

struct ListEvents<'a> {
    account: Option<&'a str>,
    calendar: Option<&'a str>,
    provider: Option<&'a str>,
    from_time: Option<&'a str>,
    to_time: Option<&'a str>,
    query: Option<&'a str>,
    limit: Option<u32>,
    all: bool,
    include_deleted: bool,
    refresh: bool,
}

async fn list(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    args: ListEvents<'_>,
) -> Result<(), CliError> {
    let time_min = match args.from_time {
        Some(s) => Some(parse_time_ms("from-time", s)?),
        None if args.all => None,
        None => Some(chrono::Utc::now().timestamp_millis()),
    };
    let time_max = args
        .to_time
        .map(|s| parse_time_ms("to-time", s))
        .transpose()?;

    // Try cache first (all-accounts path only, no filters).
    let use_cache = !args.refresh
        && args.account.is_none()
        && args.calendar.is_none()
        && args.provider.is_none()
        && config.defaults.account.is_none()
        && config.defaults.calendar.is_none()
        && args.query.is_none();

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
        let targets =
            if args.account.is_some() || args.calendar.is_some() || args.provider.is_some() {
                let ctx =
                    resolve_context(config, client, args.account, args.calendar, args.provider)
                        .await?;
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
                show_deleted: Some(args.include_deleted),
                query: args.query.map(String::from),
                user_time_zone: config.defaults.timezone.clone(),
                limit: args.limit,
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
            if !args.include_deleted && e.status == Some(ncal_api::types::EventStatus::Cancelled) {
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
    let limited: Vec<ncal_api::types::Event> = if let Some(n) = args.limit {
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

struct CreateEventArgs<'a> {
    account: Option<&'a str>,
    calendar: Option<&'a str>,
    provider: Option<&'a str>,
    summary: &'a str,
    start: &'a str,
    end: &'a str,
    description: Option<&'a str>,
    location: Option<&'a str>,
}

async fn create(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    args: CreateEventArgs<'_>,
) -> Result<(), CliError> {
    let (acct, cal, prov) =
        resolve_context(config, client, args.account, args.calendar, args.provider).await?;
    let tz = config.defaults.timezone.as_deref();
    let mut event_data = serde_json::Map::from_iter([
        ("summary".into(), serde_json::json!(args.summary)),
        ("start".into(), event_date_json(args.start, tz)),
        ("end".into(), event_date_json(args.end, tz)),
    ]);
    if let Some(d) = args.description {
        event_data.insert("description".into(), serde_json::json!(d));
    }
    if let Some(l) = args.location {
        event_data.insert("location".into(), serde_json::json!(l));
    }
    let req = CreateEventRequest {
        mutation: CreateEventMutation {
            provider: prov,
            account_id: acct,
            calendar_id: cal,
            event_data: serde_json::Value::Object(event_data),
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

struct UpdateEventArgs<'a> {
    event_id: &'a str,
    account: Option<&'a str>,
    calendar: Option<&'a str>,
    provider: Option<&'a str>,
    summary: Option<&'a str>,
    start: Option<&'a str>,
    end: Option<&'a str>,
    description: Option<&'a str>,
    location: Option<&'a str>,
}

async fn update(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    args: UpdateEventArgs<'_>,
) -> Result<(), CliError> {
    let (acct, cal, prov) =
        resolve_context(config, client, args.account, args.calendar, args.provider).await?;
    let tz = config.defaults.timezone.as_deref();
    let mut partial = serde_json::Map::new();
    if let Some(s) = args.summary {
        partial.insert("summary".into(), serde_json::json!(s));
    }
    if let Some(s) = args.start {
        partial.insert("start".into(), event_date_json(s, tz));
    }
    if let Some(s) = args.end {
        partial.insert("end".into(), event_date_json(s, tz));
    }
    if let Some(d) = args.description {
        partial.insert("description".into(), serde_json::json!(d));
    }
    if let Some(l) = args.location {
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
            event_id: args.event_id.to_string(),
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

struct DeleteEventArgs<'a> {
    event_id: &'a str,
    account: Option<&'a str>,
    calendar: Option<&'a str>,
    provider: Option<&'a str>,
    hard: bool,
}

async fn delete(
    cli: &Cli,
    config: &AppConfig,
    client: &ncal_api::client::NotionCalendarClient,
    args: DeleteEventArgs<'_>,
) -> Result<(), CliError> {
    let (acct, cal, prov) =
        resolve_context(config, client, args.account, args.calendar, args.provider).await?;
    if args.hard {
        let req = DeleteEventsRequest {
            mutations: vec![DeleteEventMutation {
                provider: prov,
                account_id: acct,
                calendar_id: cal,
                event_id: args.event_id.to_string(),
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
            event_id: args.event_id.to_string(),
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
