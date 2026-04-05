use serde::{Deserialize, Serialize};

use super::common::Provider;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    #[serde(default)]
    pub read_calendars: Option<bool>,
    #[serde(default)]
    pub read_calendar_resources: Option<bool>,
    #[serde(default)]
    pub read_events: Option<bool>,
    #[serde(default)]
    pub sync_events: Option<bool>,
    #[serde(default)]
    pub search_events: Option<bool>,
    #[serde(default)]
    pub read_org_events: Option<bool>,
    #[serde(default)]
    pub write_events: Option<bool>,
    #[serde(default)]
    pub block_events: Option<bool>,
    #[serde(default)]
    pub support_html_descriptions: Option<bool>,
    #[serde(default)]
    pub read_contacts: Option<bool>,
    #[serde(default)]
    pub read_directory: Option<bool>,
    #[serde(default)]
    pub take_meeting_notes: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub primary: Option<bool>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
    #[serde(default)]
    pub family_name: Option<String>,
    #[serde(default)]
    pub profile_photo_url: Option<String>,
    #[serde(default)]
    pub provider_name: Option<Provider>,
    #[serde(default)]
    pub provider_user_id: Option<String>,
    #[serde(default)]
    pub hosted_domain: Option<String>,
    #[serde(default)]
    pub info: Option<JsonValue>,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
    #[serde(default)]
    pub capabilities: Option<Capabilities>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub access_token_expires_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub accounts: Option<Vec<Account>>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
    #[serde(default)]
    pub family_name: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub profile_photo_url: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub notion_user_id: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, JsonValue>,
}
