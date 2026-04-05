use serde::{Deserialize, Serialize};

use super::common::{EventStatus, Provider, ResponseStatus};
use serde_json::Value as JsonValue;

/// Event datetime shape returned by the API (`date` / `dateTime` / `timeZone`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventDateTime {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub date_time: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

/// Full event payload (response-oriented): many fields optional across providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub calendar_id: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub start: Option<EventDateTime>,
    #[serde(default)]
    pub end: Option<EventDateTime>,
    #[serde(default)]
    pub status: Option<EventStatus>,
    #[serde(default)]
    pub response_status: Option<ResponseStatus>,
    #[serde(default)]
    pub html_link: Option<String>,
    #[serde(default)]
    pub hangout_link: Option<String>,
    #[serde(default)]
    pub recurrence: Option<Vec<String>>,
    #[serde(default)]
    pub recurring_event_id: Option<String>,
    #[serde(default)]
    pub provider: Option<Provider>,
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, JsonValue>,
}
