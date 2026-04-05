use clap::Subcommand;
use std::sync::Arc;

use ncal_api::auth::{
    store_credentials, CredentialSource, Credentials, DesktopAppSource, EnvVarSource,
    KeychainSource,
};
use ncal_api::client::{ClientConfig, NotionCalendarClient, OnTokenRefresh};
use ncal_api::endpoints::{create_notion_session, get_notion_login_url, CreateNotionSessionRequest};

use crate::config::AppConfig;
use crate::output::{hint, print_json, print_kv};
use crate::Cli;
use crate::CliError;

#[derive(Subcommand, Debug)]
pub enum AuthCmd {
    /// Open browser for Notion login; pass --pre-auth-token after redirect.
    Login {
        /// Pre-auth token from the OAuth redirect.
        #[arg(long)]
        pre_auth_token: Option<String>,
    },
    /// Import tokens from the Notion Calendar desktop app into the CLI keychain.
    FromApp,
    /// Show where credentials are loaded from and token expiry.
    Status,
    /// Force a token refresh now.
    Refresh,
    /// Remove stored CLI credentials from the OS keychain.
    Logout,
}

pub async fn run(cli: &Cli, config: &AppConfig, cmd: &AuthCmd) -> Result<(), CliError> {
    match cmd {
        AuthCmd::Login { pre_auth_token } => login(cli, config, pre_auth_token.as_deref()).await,
        AuthCmd::FromApp => from_app(cli, config).await,
        AuthCmd::Status => status(cli, config).await,
        AuthCmd::Refresh => refresh(config).await,
        AuthCmd::Logout => logout(config).await,
    }
}

fn make_http_client() -> Result<reqwest::Client, CliError> {
    reqwest::Client::builder()
        .user_agent(concat!("ncal-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| CliError::Api(ncal_api::error::ApiError::Network(e)))
}

fn make_client_config(config: &AppConfig) -> Result<ClientConfig, CliError> {
    let mut cc = ClientConfig::production_defaults().map_err(CliError::Api)?;
    if let Some(tz) = &config.defaults.timezone {
        cc.timezone = tz.clone();
    }
    if let Some(loc) = &config.defaults.locale {
        cc.locale = loc.clone();
    }
    Ok(cc)
}

pub(crate) fn resolve_credentials(config: &AppConfig) -> Result<Credentials, CliError> {
    let env = EnvVarSource;
    let keychain = KeychainSource { service: config.auth.keychain_service.clone() };
    let desktop = DesktopAppSource::from_env_or_default();
    let chain: [&dyn CredentialSource; 3] = [&env, &keychain, &desktop];
    ncal_api::auth::resolve_credentials(&chain).map_err(CliError::Auth)
}

pub(crate) fn build_client(config: &AppConfig, creds: Credentials) -> Result<NotionCalendarClient, CliError> {
    let service = config.auth.keychain_service.clone();
    let on_refresh: OnTokenRefresh = Arc::new(move |c: &Credentials| {
        let _ = store_credentials(&service, c);
    });
    NotionCalendarClient::builder(make_http_client()?)
        .config(make_client_config(config)?)
        .credentials(creds)
        .on_token_refresh(on_refresh)
        .build()
        .map_err(CliError::Api)
}

async fn login(cli: &Cli, config: &AppConfig, pre_auth_token: Option<&str>) -> Result<(), CliError> {
    let anon = NotionCalendarClient::for_anonymous_requests(make_http_client()?, make_client_config(config)?);

    if let Some(token) = pre_auth_token {
        let req = CreateNotionSessionRequest { pre_auth_token: token.to_string(), context: None };
        let resp = create_notion_session(&anon, &req).await.map_err(CliError::Api)?;
        let creds = Credentials::try_from(resp.user).map_err(CliError::Auth)?;
        store_credentials(&config.auth.keychain_service, &creds).map_err(CliError::Auth)?;
        if cli.json {
            return print_json(cli, &serde_json::json!({ "userId": creds.user_id, "stored": true }));
        }
        eprintln!("Session established; credentials stored in the OS keychain.");
        hint(&[
            "ncal whoami         — see your profile",
            "ncal events list    — list upcoming events",
        ]);
        return Ok(());
    }

    let url_resp = get_notion_login_url(&anon, &Default::default()).await.map_err(CliError::Api)?;
    if cli.json {
        return print_json(cli, &serde_json::json!({ "url": url_resp.url }));
    }
    eprintln!("Open this URL in a browser to sign in:\n{}", url_resp.url);
    if open::that(&url_resp.url).is_ok() {
        eprintln!("(Opened in your default browser.)");
    }
    hint(&["ncal auth login --pre-auth-token <TOKEN>  — complete login after redirect"]);
    Ok(())
}

async fn from_app(cli: &Cli, config: &AppConfig) -> Result<(), CliError> {
    let creds = DesktopAppSource::from_env_or_default().obtain().map_err(CliError::Auth)?;
    store_credentials(&config.auth.keychain_service, &creds).map_err(CliError::Auth)?;
    if cli.json {
        return print_json(cli, &serde_json::json!({ "stored": true, "source": "desktop-app" }));
    }
    eprintln!("Imported credentials from the desktop app.");
    hint(&[
        "ncal auth status    — verify credentials",
        "ncal whoami         — see your profile",
        "ncal events list    — list upcoming events",
    ]);
    Ok(())
}

async fn status(cli: &Cli, config: &AppConfig) -> Result<(), CliError> {
    let sources: [(&str, Box<dyn CredentialSource>); 3] = [
        ("env", Box::new(EnvVarSource)),
        ("keychain", Box::new(KeychainSource { service: config.auth.keychain_service.clone() })),
        ("desktop-app", Box::new(DesktopAppSource::from_env_or_default())),
    ];
    let (source_name, creds) = sources.into_iter()
        .find_map(|(name, src)| src.obtain().ok().map(|c| (name, c)))
        .ok_or(CliError::Auth(ncal_api::error::AuthError::NoCredentials))?;

    if cli.json {
        return print_json(cli, &serde_json::json!({
            "source": source_name,
            "userId": creds.user_id,
            "accessTokenExpiresAt": creds.access_token_expires_at,
            "hasRefreshToken": !creds.refresh_token.is_empty(),
        }));
    }

    print_kv(&[
        ("Source", source_name.into()),
        ("User ID", creds.user_id.clone()),
        ("Token expires", if creds.access_token_expires_at.is_empty() {
            "(unknown)".into()
        } else {
            creds.access_token_expires_at.clone()
        }),
        ("Refresh token", if creds.refresh_token.is_empty() { "no".into() } else { "yes".into() }),
    ]);
    Ok(())
}

async fn refresh(config: &AppConfig) -> Result<(), CliError> {
    let creds = resolve_credentials(config)?;
    build_client(config, creds)?.refresh_session_now().await.map_err(CliError::Api)?;
    eprintln!("Token refresh OK.");
    Ok(())
}

async fn logout(config: &AppConfig) -> Result<(), CliError> {
    ncal_api::auth::delete_credentials(&config.auth.keychain_service).map_err(CliError::Auth)?;
    eprintln!("Removed CLI credentials from keychain.");
    hint(&["ncal auth login     — sign in again",
           "ncal auth from-app  — import from desktop app"]);
    Ok(())
}
