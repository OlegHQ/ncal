use serde::Serialize;

use crate::client::NotionCalendarClient;
use crate::error::ApiError;
use crate::types::IncrementalSyncResponse;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTokenInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub account_id: String,
    pub resource_id: String,
    pub token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncrementalSyncRequest {
    pub sync_tokens: Vec<SyncTokenInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

pub async fn incremental_sync(
    client: &NotionCalendarClient,
    req: &IncrementalSyncRequest,
) -> Result<IncrementalSyncResponse, ApiError> {
    client.rpc("incrementalSync", req).await
}
