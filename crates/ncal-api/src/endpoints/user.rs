use serde_json::json;

use crate::client::NotionCalendarClient;
use crate::error::ApiError;
use crate::types::User;

pub async fn get_user(client: &NotionCalendarClient) -> Result<User, ApiError> {
    client.rpc("getUser", &json!({})).await
}

pub async fn get_user_preferences(
    client: &NotionCalendarClient,
) -> Result<serde_json::Value, ApiError> {
    client.rpc("getUserPreferences", &json!({})).await
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePrimaryAccountResponse {
    pub user: User,
}

pub async fn update_primary_account(
    client: &NotionCalendarClient,
    account_id: &str,
) -> Result<UpdatePrimaryAccountResponse, ApiError> {
    client
        .rpc(
            "updatePrimaryAccount",
            &json!({ "accountId": account_id }),
        )
        .await
}

pub async fn remove_account(client: &NotionCalendarClient, account_id: &str) -> Result<serde_json::Value, ApiError> {
    client.rpc("removeAccount", &json!({ "accountId": account_id })).await
}
