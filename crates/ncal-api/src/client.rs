use std::sync::Arc;

use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;
use url::Url;

use crate::auth::Credentials;
use crate::error::{ApiError, ApiErrorBody};

pub type OnTokenRefresh = Arc<dyn Fn(&Credentials) + Send + Sync>;

/// Mutable token state, refreshed transparently.
struct TokenState {
    user_id: String,
    access_token: String,
    refresh_token: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    last_refresh: Option<std::time::Instant>,
    refresh_failures_in_window: u32,
}

/// Immutable client config, set once at construction.
#[derive(Clone)]
pub struct ClientConfig {
    pub base_url: Url,
    pub version: String,
    pub client_type: String,
    pub timezone: String,
    pub locale: String,
}

impl ClientConfig {
    pub fn production_defaults() -> Result<Self, ApiError> {
        Ok(Self {
            base_url: Url::parse("https://calendar-api.notion.so")?,
            version: std::env::var("NCAL_CLIENT_VERSION").unwrap_or_else(|_| "0.1.0+cli".into()),
            client_type: "cli".into(),
            timezone: std::env::var("TZ").unwrap_or_else(|_| "UTC".into()),
            locale: std::env::var("LANG")
                .ok()
                .and_then(|s| s.split('.').next().map(String::from))
                .unwrap_or_else(|| "en-US".into()),
        })
    }
}

/// Main API client. Clone-friendly (`Arc` internals). Facade over HTTP + auth + retry.
#[derive(Clone)]
pub struct NotionCalendarClient {
    http: reqwest::Client,
    config: Arc<ClientConfig>,
    tokens: Arc<RwLock<TokenState>>,
    on_refresh: Option<OnTokenRefresh>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshSessionResponse {
    access_token: String,
    refresh_token: String,
    access_token_expires_at: String,
}

impl NotionCalendarClient {
    /// Client that can only call [`Self::rpc_anonymous`] (e.g. OAuth bootstrap). Do not use [`Self::rpc`].
    pub fn for_anonymous_requests(http: reqwest::Client, config: ClientConfig) -> Self {
        let tokens = TokenState {
            user_id: String::new(),
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at: chrono::DateTime::UNIX_EPOCH,
            last_refresh: None,
            refresh_failures_in_window: 0,
        };
        Self {
            http,
            config: Arc::new(config),
            tokens: Arc::new(RwLock::new(tokens)),
            on_refresh: None,
        }
    }

    pub fn builder(http: reqwest::Client) -> NotionCalendarClientBuilder {
        NotionCalendarClientBuilder {
            http,
            config: None,
            creds: None,
            on_refresh: None,
        }
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Force a refresh round-trip (for `ncal auth refresh`).
    pub async fn refresh_session_now(&self) -> Result<(), ApiError> {
        self.refresh_token().await
    }

    /// Generic POST `/v2/{operation}` with JSON body — auto-refresh, 401 retry once.
    pub async fn rpc<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        operation: &str,
        params: &Req,
    ) -> Result<Resp, ApiError> {
        self.ensure_valid_token().await?;
        let token = self.tokens.read().await.access_token.clone();
        let resp = self.do_request(operation, params, Some(&token)).await?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            self.refresh_token().await?;
            let token = self.tokens.read().await.access_token.clone();
            let resp = self.do_request(operation, params, Some(&token)).await?;
            return self.parse_response(resp).await;
        }
        self.parse_response(resp).await
    }

    /// POST without `Authorization` (for `getNotionLoginUrl`, `createNotionSession`, etc.).
    pub async fn rpc_anonymous<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        operation: &str,
        params: &Req,
    ) -> Result<Resp, ApiError> {
        let resp = self.do_request::<Req>(operation, params, None).await?;
        self.parse_response(resp).await
    }

    async fn ensure_valid_token(&self) -> Result<(), ApiError> {
        let tokens = self.tokens.read().await;
        if tokens.expires_at > chrono::Utc::now() + chrono::Duration::seconds(60) {
            return Ok(());
        }
        drop(tokens);
        self.refresh_token().await
    }

    async fn refresh_token(&self) -> Result<(), ApiError> {
        let mut tokens = self.tokens.write().await;
        if tokens.expires_at > chrono::Utc::now() + chrono::Duration::seconds(60) {
            return Ok(());
        }

        if let Some(last) = tokens.last_refresh {
            if last.elapsed() < std::time::Duration::from_secs(30)
                && tokens.refresh_failures_in_window >= 5
            {
                return Err(ApiError::RefreshCooldown);
            }
            if last.elapsed() >= std::time::Duration::from_secs(30) {
                tokens.refresh_failures_in_window = 0;
            }
        }

        let refresh_token = tokens.refresh_token.clone();
        let access_for_header = tokens.access_token.clone();
        drop(tokens);

        let resp = self
            .do_request(
                "refreshNotionSession",
                &serde_json::json!({ "refreshToken": refresh_token }),
                Some(&access_for_header),
            )
            .await?;

        let body: RefreshSessionResponse = self.parse_response(resp).await?;
        let expires_at = parse_expires(&body.access_token_expires_at);
        let access_token_expires_at = body.access_token_expires_at.clone();

        let creds = {
            let mut tokens = self.tokens.write().await;
            tokens.access_token = body.access_token;
            tokens.refresh_token = body.refresh_token;
            tokens.expires_at = expires_at;
            tokens.last_refresh = Some(std::time::Instant::now());
            tokens.refresh_failures_in_window = tokens.refresh_failures_in_window.saturating_add(1);
            Credentials {
                user_id: tokens.user_id.clone(),
                access_token: tokens.access_token.clone(),
                refresh_token: tokens.refresh_token.clone(),
                access_token_expires_at,
            }
        };

        if let Some(cb) = &self.on_refresh {
            cb(&creds);
        }

        Ok(())
    }

    async fn do_request<Req: Serialize>(
        &self,
        operation: &str,
        params: &Req,
        bearer: Option<&str>,
    ) -> Result<reqwest::Response, ApiError> {
        let path = format!("/v2/{operation}");
        let url = self.config.base_url.join(&path)?;
        let mut req = self
            .http
            .post(url)
            .query(&[
                ("ver", self.config.version.as_str()),
                ("client", self.config.client_type.as_str()),
                ("tz", self.config.timezone.as_str()),
                ("locale", self.config.locale.as_str()),
            ])
            .header("Content-Type", "application/json")
            .header("X-Client-Platform", "cli")
            .header("X-Client-Type", "cli")
            .header("X-Client-OS", std::env::consts::OS)
            .header("X-TimeZone", &self.config.timezone)
            .header(
                "X-Notion-Authenticated",
                if bearer.is_some() { "true" } else { "false" },
            )
            .json(params);

        if let Some(token) = bearer {
            req = req.header("Authorization", format!("Bearer {token}"));
        }

        Ok(req.send().await?)
    }

    async fn parse_response<Resp: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<Resp, ApiError> {
        let status = resp.status();
        let bytes = resp.bytes().await?;

        if status == StatusCode::UNAUTHORIZED {
            if let Ok(body) = serde_json::from_slice::<ApiErrorBody>(&bytes) {
                if body.code.as_deref() == Some("invalidToken") {
                    return Err(ApiError::InvalidToken);
                }
            }
            return Err(ApiError::InvalidToken);
        }

        if status == StatusCode::PRECONDITION_FAILED {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                return Err(ApiError::EtagMismatch { server_state: v });
            }
        }

        if !status.is_success() {
            let body = serde_json::from_slice::<ApiErrorBody>(&bytes).unwrap_or(ApiErrorBody {
                code: None,
                message: Some(String::from_utf8_lossy(&bytes).into_owned()),
                message_id: None,
                intl_message: None,
                status_text: None,
            });
            return Err(ApiError::Server {
                status: status.as_u16(),
                body,
            });
        }

        Ok(serde_json::from_slice(&bytes)?)
    }
}

fn parse_expires(iso: &str) -> chrono::DateTime<chrono::Utc> {
    if iso.is_empty() {
        return chrono::DateTime::UNIX_EPOCH;
    }
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or(chrono::DateTime::UNIX_EPOCH)
}

pub struct NotionCalendarClientBuilder {
    http: reqwest::Client,
    config: Option<ClientConfig>,
    creds: Option<Credentials>,
    on_refresh: Option<OnTokenRefresh>,
}

impl NotionCalendarClientBuilder {
    pub fn config(mut self, config: ClientConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn credentials(mut self, creds: Credentials) -> Self {
        self.creds = Some(creds);
        self
    }

    pub fn on_token_refresh(mut self, cb: OnTokenRefresh) -> Self {
        self.on_refresh = Some(cb);
        self
    }

    pub fn build(self) -> Result<NotionCalendarClient, ApiError> {
        let config =
            Arc::new(self.config.unwrap_or_else(|| {
                ClientConfig::production_defaults().expect("valid default URL")
            }));
        let creds = self.creds.ok_or_else(|| {
            ApiError::Config("NotionCalendarClientBuilder: missing credentials".into())
        })?;
        let expires_at = parse_expires(&creds.access_token_expires_at);
        let tokens = TokenState {
            user_id: creds.user_id.clone(),
            access_token: creds.access_token.clone(),
            refresh_token: creds.refresh_token.clone(),
            expires_at,
            last_refresh: None,
            refresh_failures_in_window: 0,
        };
        Ok(NotionCalendarClient {
            http: self.http,
            config,
            tokens: Arc::new(RwLock::new(tokens)),
            on_refresh: self.on_refresh,
        })
    }
}
