use serde::{Deserialize, Serialize};

// Closed sets from the API contract — enum + exhaustive match is appropriate.

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    Google,
    Notion,
    Icloud,
    Outlook,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Google => "google",
            Self::Notion => "notion",
            Self::Icloud => "icloud",
            Self::Outlook => "outlook",
        })
    }
}

impl std::str::FromStr for Provider {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "google" => Ok(Self::Google),
            "notion" => Ok(Self::Notion),
            "icloud" => Ok(Self::Icloud),
            "outlook" => Ok(Self::Outlook),
            _ => Err(format!(
                "unknown provider {s:?} (expected google|notion|icloud|outlook)"
            )),
        }
    }
}

/// Per-provider error returned by batch endpoints (getEvents, getCalendarLists, etc.).
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderError {
    #[serde(default)]
    pub provider: Option<String>,
    pub error_message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SendUpdates {
    All,
    #[serde(rename = "none")]
    None,
    ExternalOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecurringUpdate {
    Single,
    All,
    AllFollowing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Confirmed => "confirmed",
            Self::Tentative => "tentative",
            Self::Cancelled => "cancelled",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseStatus {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
}

impl std::fmt::Display for ResponseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NeedsAction => "needs-action",
            Self::Accepted => "accepted",
            Self::Declined => "declined",
            Self::Tentative => "tentative",
        })
    }
}
