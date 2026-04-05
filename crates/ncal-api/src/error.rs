use serde::Deserialize;

/// Server error body, deserialized from API JSON responses.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorBody {
    pub code: Option<String>,
    pub message: Option<String>,
    pub message_id: Option<String>,
    pub intl_message: Option<String>,
    pub status_text: Option<String>,
}

/// API client errors — library consumers match on these.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("API error (HTTP {status}): {body:?}")]
    Server { status: u16, body: ApiErrorBody },

    #[error("logged out by server (invalidToken)")]
    InvalidToken,

    #[error("token refresh failed: cooldown active")]
    RefreshCooldown,

    #[error("etag mismatch (HTTP 412)")]
    EtagMismatch { server_state: serde_json::Value },

    #[error("unexpected response shape: {0}")]
    UnexpectedResponse(String),

    #[error("client configuration: {0}")]
    Config(String),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
}

/// Auth-specific errors (separate from API errors).
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no credentials found (checked env vars, keychain, and desktop app)")]
    NoCredentials,

    #[error("cannot read desktop app database at {path}: {reason}")]
    LevelDb { path: String, reason: String },

    #[error("keychain error ({operation}): {reason}")]
    Keychain { operation: &'static str, reason: String },

    #[error("invalid credential payload from {origin}: {reason}")]
    InvalidCredentials { origin: &'static str, reason: String },

    #[error("API error during auth: {0}")]
    Api(#[from] ApiError),

    #[error("I/O error reading {path}: {source}")]
    Io { path: String, source: std::io::Error },

    #[error("cannot parse {context} as JSON: {source}")]
    Json { context: String, source: serde_json::Error },
}
