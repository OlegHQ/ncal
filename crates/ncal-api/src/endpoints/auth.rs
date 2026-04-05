use serde::{Deserialize, Serialize};

use crate::client::NotionCalendarClient;
use crate::error::ApiError;
use crate::types::User;

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetNotionLoginUrlRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_login: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginUrlResponse {
    pub url: String,
}

pub async fn get_notion_login_url(
    client: &NotionCalendarClient,
    req: &GetNotionLoginUrlRequest,
) -> Result<LoginUrlResponse, ApiError> {
    client.rpc_anonymous("getNotionLoginUrl", req).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotionSessionRequest {
    pub pre_auth_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CreateNotionSessionContext>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotionSessionContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notion_device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_view_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotionSessionResponse {
    pub user: User,
    #[serde(default)]
    pub is_new_user: Option<bool>,
}

pub async fn create_notion_session(
    client: &NotionCalendarClient,
    req: &CreateNotionSessionRequest,
) -> Result<CreateNotionSessionResponse, ApiError> {
    client.rpc_anonymous("createNotionSession", req).await
}
