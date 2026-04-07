use chrono::Datelike;
use ncal_api::types::{Account, Calendar, Contact, Event, EventDateTime, HoldGroup};
use serde::Serialize;

use crate::Cli;
use crate::CliError;

// ── JSON output (machine-readable, --json) ──────────────────────────────────

pub fn print_json<T: Serialize>(cli: &Cli, value: &T) -> Result<(), CliError> {
    if cli.json {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

pub fn print_json_lines<T: Serialize>(cli: &Cli, items: &[T]) -> Result<(), CliError> {
    if cli.json {
        for item in items {
            println!("{}", serde_json::to_string(item)?);
        }
    } else {
        print_json(cli, &serde_json::json!({ "items": items }))?;
    }
    Ok(())
}

// ── Table output (human-readable, default) ──────────────────────────────────

pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("(none)");
        return;
    }
    let n = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(n) {
            widths[i] = widths[i].max(cell.len());
        }
    }
    let header: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:<w$}", h, w = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{header}");
    for row in rows {
        let line: String = row
            .iter()
            .enumerate()
            .take(n)
            .map(|(i, c)| format!("{:<w$}", c, w = widths[i]))
            .collect::<Vec<_>>()
            .join("  ");
        println!("{line}");
    }
}

// ── Key-value output (for describe/status) ──────────────────────────────────

pub fn print_kv(pairs: &[(&str, String)]) {
    let max_key = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (k, v) in pairs {
        println!("{:<w$}  {v}", format!("{k}:"), w = max_key + 1);
    }
}

// ── Hints (next-steps for humans and agents, stderr) ─────────────────────────

pub fn hint(items: &[&str]) {
    if items.is_empty() {
        return;
    }
    eprintln!();
    for (i, item) in items.iter().enumerate() {
        if i == 0 {
            eprintln!("hint: {item}");
        } else {
            eprintln!("      {item}");
        }
    }
}

// ── Type-specific display ───────────────────────────────────────────────────

pub fn print_accounts(cli: &Cli, accounts: &[Account]) -> Result<(), CliError> {
    if cli.json {
        return print_json(cli, &accounts);
    }
    let rows: Vec<Vec<String>> = accounts
        .iter()
        .map(|a| {
            vec![
                a.provider_name.map(|p| p.to_string()).unwrap_or_default(),
                a.email.clone().unwrap_or_default(),
                a.display_name.clone().unwrap_or_default(),
                if a.primary == Some(true) { "*".into() } else { String::new() },
                a.id.clone(),
            ]
        })
        .collect();
    print_table(&["PROVIDER", "EMAIL", "NAME", "PRI", "ID"], &rows);
    Ok(())
}

pub fn print_calendars(cli: &Cli, calendars: &[Calendar]) -> Result<(), CliError> {
    if cli.json {
        return print_json(cli, &calendars);
    }
    let rows: Vec<Vec<String>> = calendars
        .iter()
        .map(|c| {
            vec![
                c.summary.clone().unwrap_or_else(|| c.id.clone()),
                c.provider.map(|p| p.to_string()).unwrap_or_default(),
                c.access_role.clone().unwrap_or_default(),
                if c.primary == Some(true) { "*".into() } else { String::new() },
                c.id.clone(),
            ]
        })
        .collect();
    print_table(&["CALENDAR", "PROVIDER", "ROLE", "PRI", "ID"], &rows);
    Ok(())
}

pub fn print_events(cli: &Cli, events: &[Event]) -> Result<(), CliError> {
    if cli.json {
        return print_json(cli, &events);
    }
    let mut sorted: Vec<&Event> = events.iter().collect();
    sorted.sort_by_key(|e| event_sort_key(e));
    let rows: Vec<Vec<String>> = sorted
        .iter()
        .map(|e| {
            vec![
                fmt_datetime(&e.start),
                fmt_datetime(&e.end),
                e.summary.clone().unwrap_or_else(|| "(no title)".into()),
                e.status.map(|s| s.to_string()).unwrap_or_default(),
                e.id.clone(),
            ]
        })
        .collect();
    print_table(&["START", "END", "SUMMARY", "STATUS", "ID"], &rows);
    Ok(())
}

pub fn print_event_detail(cli: &Cli, event: &Event) -> Result<(), CliError> {
    if cli.json {
        return print_json(cli, event);
    }
    let mut pairs = vec![
        ("ID", event.id.clone()),
        ("Summary", event.summary.clone().unwrap_or_else(|| "(no title)".into())),
        ("Start", fmt_datetime(&event.start)),
        ("End", fmt_datetime(&event.end)),
        ("Status", event.status.map(|s| s.to_string()).unwrap_or_default()),
    ];
    if let Some(loc) = &event.location {
        pairs.push(("Location", loc.clone()));
    }
    if let Some(desc) = &event.description {
        pairs.push(("Description", desc.clone()));
    }
    if let Some(link) = &event.html_link {
        pairs.push(("Link", link.clone()));
    }
    if let Some(r) = &event.response_status {
        pairs.push(("RSVP", r.to_string()));
    }
    let refs: Vec<(&str, String)> = pairs.into_iter().collect();
    print_kv(&refs);
    Ok(())
}

pub fn print_contacts(cli: &Cli, contacts: &[Contact]) -> Result<(), CliError> {
    if cli.json {
        return print_json(cli, &contacts);
    }
    let rows: Vec<Vec<String>> = contacts
        .iter()
        .map(|c| {
            vec![
                c.display_name.clone().unwrap_or_default(),
                c.email.clone().unwrap_or_default(),
            ]
        })
        .collect();
    print_table(&["NAME", "EMAIL"], &rows);
    Ok(())
}

pub fn print_holds(cli: &Cli, holds: &[HoldGroup]) -> Result<(), CliError> {
    if cli.json {
        return print_json(cli, &holds);
    }
    let rows: Vec<Vec<String>> = holds
        .iter()
        .map(|h| {
            vec![
                h.title.clone().unwrap_or_default(),
                h.alias.clone().unwrap_or_default(),
                h.duration.map(|d| format!("{d}m")).unwrap_or_default(),
                h.status.clone().unwrap_or_default(),
                h.scheduling_link.clone().unwrap_or_default(),
                h.id.clone(),
            ]
        })
        .collect();
    print_table(&["TITLE", "ALIAS", "DUR", "STATUS", "LINK", "ID"], &rows);
    Ok(())
}

pub fn print_hold_detail(cli: &Cli, hold: &HoldGroup) -> Result<(), CliError> {
    if cli.json {
        return print_json(cli, hold);
    }
    let mut pairs = vec![
        ("ID", hold.id.clone()),
        ("Title", hold.title.clone().unwrap_or_default()),
        ("Alias", hold.alias.clone().unwrap_or_default()),
        ("Type", hold.hold_type.clone().unwrap_or_default()),
        ("Status", hold.status.clone().unwrap_or_default()),
        ("Duration", hold.duration.map(|d| format!("{d} min")).unwrap_or_default()),
        ("Timezone", hold.time_zone.clone().unwrap_or_default()),
    ];
    if let Some(link) = &hold.scheduling_link {
        pairs.push(("Link", link.clone()));
    }
    if let Some(desc) = &hold.description {
        pairs.push(("Description", desc.clone()));
    }
    if let Some(ranges) = &hold.time_ranges {
        for (i, r) in ranges.iter().enumerate() {
            let label = if i == 0 { "Time ranges" } else { "" };
            let start = r.start_date.as_deref().unwrap_or("?");
            let end = r.end_date.as_deref().unwrap_or("?");
            pairs.push((label, format!("{start} — {end}")));
        }
    }
    if let Some(acct) = &hold.google_account_id {
        pairs.push(("Account", acct.clone()));
    }
    if let Some(cal) = &hold.google_calendar_id {
        pairs.push(("Calendar", cal.clone()));
    }
    let refs: Vec<(&str, String)> = pairs.into_iter().collect();
    print_kv(&refs);
    Ok(())
}

pub fn print_user(cli: &Cli, user: &ncal_api::types::User) -> Result<(), CliError> {
    if cli.json {
        return print_json(cli, user);
    }
    let name = user.display_name.as_deref().unwrap_or("(unknown)");
    let email = user.email.as_deref().unwrap_or("(unknown)");
    let n_accounts = user.accounts.as_ref().map(|a| a.len()).unwrap_or(0);
    print_kv(&[
        ("Name", name.into()),
        ("Email", email.into()),
        ("User ID", user.id.clone()),
        ("Accounts", n_accounts.to_string()),
    ]);
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

pub fn fmt_datetime(edt: &Option<EventDateTime>) -> String {
    match edt {
        None => "-".into(),
        Some(dt) => {
            if let Some(d) = &dt.date_time {
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(d) {
                    let now = chrono::Utc::now();
                    let fmt = if parsed.year() != now.year() {
                        "%b %d %Y, %H:%M"
                    } else {
                        "%b %d, %H:%M"
                    };
                    return parsed.format(fmt).to_string();
                }
                d.clone()
            } else if let Some(d) = &dt.date {
                if let Ok(parsed) = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d") {
                    let now = chrono::Utc::now().date_naive();
                    let fmt = if parsed.year() != now.year() {
                        "%b %d %Y"
                    } else {
                        "%b %d"
                    };
                    return format!("{} (all day)", parsed.format(fmt));
                }
                format!("{d} (all day)")
            } else {
                "-".into()
            }
        }
    }
}

/// Extract a sortable epoch-millis key from an event's start time.
pub fn event_sort_key(event: &Event) -> i64 {
    match &event.start {
        Some(dt) => {
            if let Some(d) = &dt.date_time {
                chrono::DateTime::parse_from_rfc3339(d)
                    .map(|p| p.timestamp_millis())
                    .unwrap_or(i64::MAX)
            } else if let Some(d) = &dt.date {
                chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                    .map(|p| p.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis())
                    .unwrap_or(i64::MAX)
            } else {
                i64::MAX
            }
        }
        None => i64::MAX,
    }
}

