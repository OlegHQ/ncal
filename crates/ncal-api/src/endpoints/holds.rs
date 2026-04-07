use serde::{Deserialize, Serialize};

use crate::client::NotionCalendarClient;
use crate::error::ApiError;
use crate::types::HoldGroup;

// ── getHolds ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHoldsResponse {
    #[serde(default)]
    pub holds: Vec<HoldGroup>,
}

pub async fn get_holds(client: &NotionCalendarClient) -> Result<GetHoldsResponse, ApiError> {
    client.rpc("getHolds", &serde_json::json!({})).await
}

// ── getHold (public scheduling page) ───────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHoldRequest {
    pub username: String,
    pub alias: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_max: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHoldResponse {
    pub hold: HoldGroup,
}

pub async fn get_hold(
    client: &NotionCalendarClient,
    req: &GetHoldRequest,
) -> Result<GetHoldResponse, ApiError> {
    client.rpc("getHold", req).await
}

// ── createHold ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHoldRequest {
    pub id: String,
    pub alias: String,
    #[serde(rename = "type")]
    pub hold_type: String,
    pub status: String,
    pub user_primary_time_zone: String,
    pub time_zone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_ranges: Option<Vec<crate::types::TimeRange>>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conferencing_provider_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conferencing_account_id: Option<String>,
    pub google_account_id: String,
    pub google_calendar_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_free_resources: Option<Vec<crate::types::ConflictFreeResource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_lead_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_lead_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateHoldResponse {
    pub success: bool,
}

pub async fn create_hold(
    client: &NotionCalendarClient,
    req: &CreateHoldRequest,
) -> Result<CreateHoldResponse, ApiError> {
    client.rpc("createHold", req).await
}

// ── updateHold ─────────────────────────────────────────────────────────────

pub async fn update_hold(
    client: &NotionCalendarClient,
    req: &CreateHoldRequest,
) -> Result<CreateHoldResponse, ApiError> {
    client.rpc("updateHold", req).await
}

// ── deleteHold ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteHoldRequest {
    pub hold_id: String,
}

pub async fn delete_hold(
    client: &NotionCalendarClient,
    req: &DeleteHoldRequest,
) -> Result<CreateHoldResponse, ApiError> {
    client.rpc("deleteHold", req).await
}

// ── getHoldAliasAvailable ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CheckAliasRequest {
    pub alias: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CheckAliasResponse {
    pub available: bool,
}

pub async fn get_hold_alias_available(
    client: &NotionCalendarClient,
    req: &CheckAliasRequest,
) -> Result<CheckAliasResponse, ApiError> {
    client.rpc("getHoldAliasAvailable", req).await
}

// ── createHoldEvent (book a slot) ──────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHoldEventRequest {
    pub hold_short_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub email: String,
    pub start_date: String,
    pub end_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_field_responses: Option<Vec<serde_json::Value>>,
}

pub async fn create_hold_event(
    client: &NotionCalendarClient,
    req: &CreateHoldEventRequest,
) -> Result<CreateHoldResponse, ApiError> {
    client.rpc("createHoldEvent", req).await
}

// ── updateHoldEvent (reschedule / cancel) ──────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateHoldEventRequest {
    pub event_id: String,
    pub hold_short_id: String,
    pub hold_slot_id: String,
    pub update_type: String,
    pub update_properties: serde_json::Value,
}

pub async fn update_hold_event(
    client: &NotionCalendarClient,
    req: &UpdateHoldEventRequest,
) -> Result<CreateHoldResponse, ApiError> {
    client.rpc("updateHoldEvent", req).await
}
