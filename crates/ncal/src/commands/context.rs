use ncal_api::client::NotionCalendarClient;
use ncal_api::endpoints::{
    get_calendar_lists, get_user, CalendarListQuery, CalendarListResult, GetCalendarListsRequest,
};
use ncal_api::types::{Account, Calendar, Provider};

use super::auth::{build_client, resolve_credentials};
use crate::config::AppConfig;
use crate::CliError;

/// Build an authenticated client (resolve credentials + construct client) in one step.
pub fn authenticated_client(config: &AppConfig) -> Result<NotionCalendarClient, CliError> {
    build_client(config, resolve_credentials(config)?)
}

/// Parse a provider string (case-insensitive) into a `Provider`.
pub(super) fn parse_provider(s: &str) -> Result<Provider, CliError> {
    s.parse().map_err(CliError::Usage)
}

/// Parse an RFC3339 or epoch-millis string into epoch millis.
pub fn parse_time_ms(label: &str, s: &str) -> Result<i64, CliError> {
    if let Ok(ms) = s.parse::<i64>() {
        return Ok(ms);
    }
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.timestamp_millis())
        .map_err(|_| {
            CliError::Usage(format!(
                "--{label}: expected epoch millis or RFC3339, got {s:?}"
            ))
        })
}

/// Extract successful calendars from batch results, logging errors to stderr.
pub fn flatten_calendar_results(results: Vec<CalendarListResult>) -> Vec<Calendar> {
    results
        .into_iter()
        .flat_map(|r| match r {
            CalendarListResult::Ok(ok) => ok.calendars,
            CalendarListResult::Err(err) => {
                eprintln!("warning: calendar list error: {}", err.error_message);
                vec![]
            }
        })
        .collect()
}

/// Resolve account + calendar + provider from optional CLI hints and config defaults.
pub async fn resolve_context(
    config: &AppConfig,
    client: &NotionCalendarClient,
    account: Option<&str>,
    calendar: Option<&str>,
    provider: Option<&str>,
) -> Result<(String, String, Provider), CliError> {
    let account_hint = account.or(config.defaults.account.as_deref());
    let (account_id, resolved_provider) = resolve_account(client, account_hint).await?;
    let prov = provider
        .map(parse_provider)
        .transpose()?
        .unwrap_or(resolved_provider);
    let calendar_id = match calendar.or(config.defaults.calendar.as_deref()) {
        Some(c) => c.to_string(),
        None => auto_resolve_calendar(client, &account_id, prov).await?,
    };
    Ok((account_id, calendar_id, prov))
}

async fn resolve_account(
    client: &NotionCalendarClient,
    hint: Option<&str>,
) -> Result<(String, Provider), CliError> {
    let user = get_user(client).await.map_err(CliError::Api)?;
    let accounts = user.accounts.unwrap_or_default();

    if let Some(q) = hint {
        let q_lower = q.to_ascii_lowercase();
        if let Some(found) = accounts.iter().find(|a| {
            a.id == q
                || a.email
                    .as_deref()
                    .is_some_and(|e| e.to_ascii_lowercase() == q_lower)
        }) {
            return Ok((
                found.id.clone(),
                found.provider_name.unwrap_or(Provider::Google),
            ));
        }
        return Err(CliError::Usage(format!(
            "no account matching {:?}; available accounts:\n{}",
            q,
            format_account_list(&accounts),
        )));
    }

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
        many => Err(CliError::Usage(format!(
            "multiple accounts found; specify --account <EMAIL>:\n{}\nor set defaults.account in your config file",
            format_account_list(many),
        ))),
    }
}

/// Build one (account, calendar, provider) tuple per calendar across all accounts.
/// Fetches all calendars in a single batch RPC. Used by `events list` when no
/// `--account` is specified — queries every calendar the user has.
pub async fn build_all_account_queries(
    client: &NotionCalendarClient,
) -> Result<Vec<(String, String, Provider)>, CliError> {
    let user = get_user(client).await.map_err(CliError::Api)?;
    let accounts = user.accounts.unwrap_or_default();
    if accounts.is_empty() {
        return Err(CliError::Usage(
            "no accounts connected; add one in the Notion Calendar app".into(),
        ));
    }
    let queries: Vec<CalendarListQuery> = accounts
        .iter()
        .filter_map(|a| {
            Some(CalendarListQuery {
                provider: a.provider_name?,
                account_id: a.id.clone(),
            })
        })
        .collect();
    let results = get_calendar_lists(client, &GetCalendarListsRequest { queries })
        .await
        .map_err(CliError::Api)?;
    let calendars = flatten_calendar_results(results);
    if calendars.is_empty() {
        return Err(CliError::Usage("no calendars found for any account".into()));
    }
    Ok(calendars
        .into_iter()
        .filter_map(|c| Some((c.account_id?, c.id, c.provider?)))
        .collect())
}

fn format_account_list(accounts: &[Account]) -> String {
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
    let results = get_calendar_lists(client, &req)
        .await
        .map_err(CliError::Api)?;
    let calendars = flatten_calendar_results(results);

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
                    if c.primary == Some(true) {
                        " (primary)"
                    } else {
                        ""
                    },
                ));
            }
            msg.push_str("\nor set defaults.calendar in your config file");
            Err(CliError::Usage(msg))
        }
    }
}
