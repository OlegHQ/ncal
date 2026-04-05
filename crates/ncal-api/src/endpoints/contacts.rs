use serde::Serialize;

use crate::client::NotionCalendarClient;
use crate::error::ApiError;
use crate::types::Contact;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContactsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_alternate_emails: Option<bool>,
}

pub async fn get_contacts(client: &NotionCalendarClient) -> Result<Vec<Contact>, ApiError> {
    let req = GetContactsRequest {
        include_alternate_emails: Some(true),
    };
    client.rpc("getContacts", &req).await
}
