use clap::Subcommand;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use ncal_api::auth::{
    delete_credentials_file, store_credentials, store_credentials_file, CredentialSource,
    Credentials, DesktopAppSource, EnvVarSource, FileSource, KeychainSource,
};
use ncal_api::client::{ClientConfig, NotionCalendarClient, OnTokenRefresh};
use ncal_api::endpoints::{
    create_notion_session, get_notion_login_url, CreateNotionSessionRequest,
};
use ncal_api::types::User;

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
    /// Import browser LocalStorage `user.auth.currentUser` JSON into the CLI keychain.
    ImportUserJson {
        /// Read the `user.auth.currentUser` JSON from this file.
        #[arg(long, conflicts_with_all = ["json", "stdin"])]
        file: Option<PathBuf>,
        /// Read the `user.auth.currentUser` JSON from this argument.
        #[arg(long = "raw-json", conflicts_with_all = ["file", "stdin"])]
        raw_json: Option<String>,
        /// Read the `user.auth.currentUser` JSON from stdin.
        #[arg(long, conflicts_with_all = ["file", "json"])]
        stdin: bool,
    },
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
        AuthCmd::ImportUserJson {
            file,
            raw_json,
            stdin,
        } => import_user_json(cli, config, file.as_deref(), raw_json.as_deref(), *stdin).await,
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
    let keychain = KeychainSource {
        service: config.auth.keychain_service.clone(),
    };
    let file = FileSource {
        path: config.auth.credentials_file.clone(),
    };
    let desktop = DesktopAppSource::from_env_or_default();
    let chain: [&dyn CredentialSource; 4] = [&env, &keychain, &file, &desktop];
    ncal_api::auth::resolve_credentials(&chain).map_err(CliError::Auth)
}

pub(crate) fn build_client(
    config: &AppConfig,
    creds: Credentials,
) -> Result<NotionCalendarClient, CliError> {
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

async fn login(
    cli: &Cli,
    config: &AppConfig,
    pre_auth_token: Option<&str>,
) -> Result<(), CliError> {
    let anon = NotionCalendarClient::for_anonymous_requests(
        make_http_client()?,
        make_client_config(config)?,
    );

    if let Some(token) = pre_auth_token {
        let req = CreateNotionSessionRequest {
            pre_auth_token: token.to_string(),
            context: None,
        };
        let resp = create_notion_session(&anon, &req)
            .await
            .map_err(CliError::Api)?;
        let creds = Credentials::try_from(resp.user).map_err(CliError::Auth)?;
        let stored_in = store_cli_credentials(config, &creds)?;
        if cli.json {
            return print_json(
                cli,
                &serde_json::json!({ "userId": creds.user_id, "stored": true, "storedIn": stored_in }),
            );
        }
        eprintln!("Session established; credentials stored in {stored_in}.");
        hint(&[
            "ncal whoami         — see your profile",
            "ncal events list    — list upcoming events",
        ]);
        return Ok(());
    }

    let url_resp = get_notion_login_url(&anon, &Default::default())
        .await
        .map_err(CliError::Api)?;
    if cli.json {
        return print_json(cli, &serde_json::json!({ "url": url_resp.url }));
    }
    eprintln!("Open this URL in a browser to sign in:\n{}", url_resp.url);
    if open::that(&url_resp.url).is_ok() {
        eprintln!("(Opened in your default browser.)");
    }
    hint(&[
        "ncal auth login --pre-auth-token <TOKEN>       — complete login if you captured the redirect token",
        "ncal auth import-user-json --file user.json    — SSH/headless fallback after browser login",
    ]);
    Ok(())
}

async fn from_app(cli: &Cli, config: &AppConfig) -> Result<(), CliError> {
    let creds = DesktopAppSource::from_env_or_default()
        .obtain()
        .map_err(CliError::Auth)?;
    let stored_in = store_cli_credentials(config, &creds)?;
    if cli.json {
        return print_json(
            cli,
            &serde_json::json!({ "stored": true, "source": "desktop-app", "storedIn": stored_in }),
        );
    }
    eprintln!("Imported credentials from the desktop app into {stored_in}.");
    hint(&[
        "ncal auth status    — verify credentials",
        "ncal whoami         — see your profile",
        "ncal events list    — list upcoming events",
    ]);
    Ok(())
}

async fn import_user_json(
    cli: &Cli,
    config: &AppConfig,
    file: Option<&std::path::Path>,
    json_arg: Option<&str>,
    stdin: bool,
) -> Result<(), CliError> {
    let input = match (file, json_arg, stdin) {
        (Some(path), None, false) => std::fs::read_to_string(path).map_err(CliError::Io)?,
        (None, Some(json), false) => json.to_string(),
        (None, None, true) => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(CliError::Io)?;
            buf
        }
        (None, None, false) => {
            return Err(CliError::Usage(
                "provide one of --file <PATH>, --raw-json '<JSON>', or --stdin".into(),
            ));
        }
        _ => {
            return Err(CliError::Usage(
                "provide only one of --file, --raw-json, or --stdin".into(),
            ));
        }
    };

    let user = parse_user_json(&input)?;
    let creds = Credentials::try_from(user).map_err(CliError::Auth)?;
    let stored_in = store_cli_credentials(config, &creds)?;

    if cli.json {
        return print_json(
            cli,
            &serde_json::json!({
                "stored": true,
                "source": "browser-local-storage",
                "storedIn": stored_in,
                "userId": creds.user_id,
                "accessTokenExpiresAt": creds.access_token_expires_at,
                "hasRefreshToken": !creds.refresh_token.is_empty(),
            }),
        );
    }

    eprintln!("Imported credentials from browser LocalStorage JSON into {stored_in}.");
    hint(&[
        "ncal auth status    — verify credentials",
        "ncal whoami         — see your profile",
        "ncal events list    — list upcoming events",
    ]);
    Ok(())
}

fn parse_user_json(input: &str) -> Result<User, CliError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CliError::Usage("empty JSON input".into()));
    }

    let value: serde_json::Value = serde_json::from_str(trimmed)?;
    let user_value = if let Some(s) = value.as_str() {
        serde_json::from_str::<serde_json::Value>(s)?
    } else {
        value
    };

    serde_json::from_value(user_value).map_err(CliError::Json)
}

async fn status(cli: &Cli, config: &AppConfig) -> Result<(), CliError> {
    let sources: [(&str, Box<dyn CredentialSource>); 4] = [
        ("env", Box::new(EnvVarSource)),
        (
            "keychain",
            Box::new(KeychainSource {
                service: config.auth.keychain_service.clone(),
            }),
        ),
        (
            "file",
            Box::new(FileSource {
                path: config.auth.credentials_file.clone(),
            }),
        ),
        (
            "desktop-app",
            Box::new(DesktopAppSource::from_env_or_default()),
        ),
    ];
    let (source_name, creds) = sources
        .into_iter()
        .find_map(|(name, src)| src.obtain().ok().map(|c| (name, c)))
        .ok_or(CliError::Auth(ncal_api::error::AuthError::NoCredentials))?;

    if cli.json {
        return print_json(
            cli,
            &serde_json::json!({
                "source": source_name,
                "userId": creds.user_id,
                "accessTokenExpiresAt": creds.access_token_expires_at,
                "hasRefreshToken": !creds.refresh_token.is_empty(),
            }),
        );
    }

    print_kv(&[
        ("Source", source_name.into()),
        ("User ID", creds.user_id.clone()),
        (
            "Token expires",
            if creds.access_token_expires_at.is_empty() {
                "(unknown)".into()
            } else {
                creds.access_token_expires_at.clone()
            },
        ),
        (
            "Refresh token",
            if creds.refresh_token.is_empty() {
                "no".into()
            } else {
                "yes".into()
            },
        ),
    ]);
    Ok(())
}

async fn refresh(config: &AppConfig) -> Result<(), CliError> {
    let creds = resolve_credentials(config)?;
    build_client(config, creds)?
        .refresh_session_now()
        .await
        .map_err(CliError::Api)?;
    eprintln!("Token refresh OK.");
    Ok(())
}

async fn logout(config: &AppConfig) -> Result<(), CliError> {
    let keychain_result = ncal_api::auth::delete_credentials(&config.auth.keychain_service);
    delete_credentials_file(&config.auth.credentials_file).map_err(CliError::Auth)?;
    if let Err(err) = keychain_result {
        eprintln!("warning: could not remove keychain credentials: {err}");
    }
    eprintln!("Removed CLI credentials from keychain and file storage.");
    hint(&[
        "ncal auth login     — sign in again",
        "ncal auth from-app  — import from desktop app",
    ]);
    Ok(())
}

fn store_cli_credentials(
    config: &AppConfig,
    creds: &Credentials,
) -> Result<&'static str, CliError> {
    match store_credentials(&config.auth.keychain_service, creds) {
        Ok(()) => Ok("OS keychain"),
        Err(err) => {
            eprintln!("warning: OS keychain unavailable ({err}); falling back to credentials file");
            store_credentials_file(&config.auth.credentials_file, creds).map_err(CliError::Auth)?;
            Ok("credentials file")
        }
    }
}
