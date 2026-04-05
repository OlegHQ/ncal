use serde::{Deserialize, Serialize};

use super::{calendar::Calendar, event::Event};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncToken {
    #[serde(default)]
    pub provider: Option<String>,
    pub account_id: String,
    pub resource_id: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncrementalSyncResponse {
    #[serde(default)]
    pub calendars: Vec<Calendar>,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub sync_tokens: Vec<SyncToken>,
    #[serde(default)]
    pub errors: Vec<serde_json::Value>,
}
