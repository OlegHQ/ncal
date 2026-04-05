use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Minimal contact shape for `getContacts`; extend as needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    #[serde(default)]
    pub resource_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, JsonValue>,
}
