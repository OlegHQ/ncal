use serde::{Deserialize, Serialize};

use crate::client::NotionCalendarClient;
use crate::error::ApiError;
use crate::types::{Calendar, Provider};

#[derive(Debug, Serialize, Clone)]
pub struct GetCalendarListsRequest {
    pub queries: Vec<CalendarListQuery>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListQuery {
    pub provider: Provider,
    pub account_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListOk {
    #[serde(default)]
    pub calendars: Vec<Calendar>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListErr {
    #[serde(default)]
    pub provider: Option<String>,
    pub error_message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CalendarListResult {
    Ok(CalendarListOk),
    Err(CalendarListErr),
}

pub async fn get_calendar_lists(
    client: &NotionCalendarClient,
    req: &GetCalendarListsRequest,
) -> Result<Vec<CalendarListResult>, ApiError> {
    client.rpc("getCalendarLists", req).await
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorEntry {
    pub id: i64,
    pub kind: String,
    pub background: String,
    pub foreground: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorsResponse {
    #[serde(default)]
    pub calendar: Option<Vec<ColorEntry>>,
    #[serde(default)]
    pub event: Option<Vec<ColorEntry>>,
}

pub async fn get_colors(client: &NotionCalendarClient) -> Result<ColorsResponse, ApiError> {
    client.rpc("getColors", &serde_json::json!({})).await
}
