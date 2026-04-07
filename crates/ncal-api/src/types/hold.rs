use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// A scheduling hold group — defines availability for booking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldGroup {
    pub id: String,
    #[serde(default)]
    pub short_id: Option<String>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default, rename = "type")]
    pub hold_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub user_primary_time_zone: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub time_ranges: Option<Vec<TimeRange>>,
    #[serde(default)]
    pub google_account_id: Option<String>,
    #[serde(default)]
    pub google_calendar_id: Option<String>,
    #[serde(default)]
    pub conferencing_provider_name: Option<String>,
    #[serde(default)]
    pub conferencing_account_id: Option<String>,
    #[serde(default)]
    pub conflict_free_resources: Option<Vec<ConflictFreeResource>>,
    #[serde(default)]
    pub min_lead_time: Option<u64>,
    #[serde(default)]
    pub max_lead_time: Option<u64>,
    #[serde(default)]
    pub has_been_booked: Option<bool>,
    #[serde(default)]
    pub scheduling_link: Option<String>,
    #[serde(default)]
    pub data: Option<JsonValue>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub recurrence: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictFreeResource {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub calendar_id: Option<String>,
}
