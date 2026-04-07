use serde::{Deserialize, Serialize};

use crate::client::NotionCalendarClient;
use crate::error::ApiError;
use crate::types::{Event, Provider, ProviderError, SendUpdates};

#[derive(Debug, Serialize)]
pub struct GetEventsRequest {
    pub queries: Vec<EventQuery>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventQuery {
    pub provider: Provider,
    pub account_id: String,
    pub calendar_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_max: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_events: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_deleted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetEventsOk {
    pub account_id: String,
    pub calendar_id: String,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub sync_token: Option<String>,
    #[serde(default)]
    pub page_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GetEventsResult {
    Ok(GetEventsOk),
    Err(ProviderError),
}

pub async fn get_events(
    client: &NotionCalendarClient,
    req: &GetEventsRequest,
) -> Result<Vec<GetEventsResult>, ApiError> {
    client.rpc("getEvents", req).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetEventQuery {
    pub provider: Provider,
    pub account_id: String,
    pub calendar_id: String,
    pub event_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_time_zone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetEventRequest {
    pub query: GetEventQuery,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetEventResponse {
    pub event: Event,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub calendar_id: Option<String>,
}

pub async fn get_event(
    client: &NotionCalendarClient,
    req: &GetEventRequest,
) -> Result<GetEventResponse, ApiError> {
    client.rpc("getEvent", req).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventMutation {
    pub provider: Provider,
    pub account_id: String,
    pub calendar_id: String,
    pub event_data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_updates: Option<SendUpdates>,
}

#[derive(Debug, Serialize)]
pub struct CreateEventRequest {
    pub mutation: CreateEventMutation,
}

pub async fn create_event(
    client: &NotionCalendarClient,
    req: &CreateEventRequest,
) -> Result<Event, ApiError> {
    client.rpc("createEvent", req).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventMutation {
    pub provider: Provider,
    pub account_id: String,
    pub event_id: String,
    pub calendar_id: String,
    pub event_data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_updates: Option<SendUpdates>,
}

#[derive(Debug, Serialize)]
pub struct UpdateEventsRequest {
    pub mutations: Vec<UpdateEventMutation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventEntry {
    #[serde(default)]
    pub event: Option<Event>,
    #[serde(default)]
    pub error_message: Option<String>,
}

pub async fn update_events(
    client: &NotionCalendarClient,
    req: &UpdateEventsRequest,
) -> Result<Vec<UpdateEventEntry>, ApiError> {
    client.rpc("updateEvents", req).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEventMutation {
    pub provider: Provider,
    pub account_id: String,
    pub calendar_id: String,
    pub event_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_updates: Option<SendUpdates>,
}

#[derive(Debug, Serialize)]
pub struct DeleteEventsRequest {
    pub mutations: Vec<DeleteEventMutation>,
}

pub async fn delete_events(
    client: &NotionCalendarClient,
    req: &DeleteEventsRequest,
) -> Result<serde_json::Value, ApiError> {
    client.rpc("deleteEvents", req).await
}
