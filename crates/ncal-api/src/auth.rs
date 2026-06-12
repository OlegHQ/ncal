use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::AuthError;
use crate::types::User;

/// Shared credential shape for CLI persistence and client bootstrap.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub user_id: String,
    pub access_token: String,
    pub refresh_token: String,
    /// ISO 8601 timestamp from the API (`accessTokenExpiresAt`).
    pub access_token_expires_at: String,
}

impl TryFrom<User> for Credentials {
    type Error = AuthError;

    fn try_from(user: User) -> Result<Self, Self::Error> {
        let access_token = user
            .access_token
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AuthError::InvalidCredentials {
                origin: "API response",
                reason: "missing or empty accessToken field".into(),
            })?;
        let refresh_token = user.refresh_token.clone().unwrap_or_default();
        let access_token_expires_at = user.access_token_expires_at.clone().unwrap_or_default();
        Ok(Credentials {
            user_id: user.id,
            access_token,
            refresh_token,
            access_token_expires_at,
        })
    }
}

/// Strategy pattern: each credential source is a separate implementation.
pub trait CredentialSource: Send + Sync {
    fn name(&self) -> &'static str;
    fn obtain(&self) -> Result<Credentials, AuthError>;
}

/// `NCAL_USER_ID`, `NCAL_ACCESS_TOKEN`, optional `NCAL_REFRESH_TOKEN`, `NCAL_TOKEN_EXPIRES_AT`.
pub struct EnvVarSource;

impl CredentialSource for EnvVarSource {
    fn name(&self) -> &'static str {
        "env"
    }

    fn obtain(&self) -> Result<Credentials, AuthError> {
        let user_id = std::env::var("NCAL_USER_ID").map_err(|_| AuthError::NoCredentials)?;
        let access_token =
            std::env::var("NCAL_ACCESS_TOKEN").map_err(|_| AuthError::NoCredentials)?;
        Ok(Credentials {
            user_id,
            access_token,
            refresh_token: std::env::var("NCAL_REFRESH_TOKEN").unwrap_or_default(),
            access_token_expires_at: std::env::var("NCAL_TOKEN_EXPIRES_AT").unwrap_or_default(),
        })
    }
}

/// Read tokens previously stored by this CLI in the OS credential store.
pub struct KeychainSource {
    pub service: String,
}

impl CredentialSource for KeychainSource {
    fn name(&self) -> &'static str {
        "keychain"
    }

    fn obtain(&self) -> Result<Credentials, AuthError> {
        let entry =
            keyring::Entry::new(&self.service, "default").map_err(|e| AuthError::Keychain {
                operation: "open",
                reason: e.to_string(),
            })?;
        let json = entry.get_password().map_err(|_| AuthError::NoCredentials)?;
        serde_json::from_str(&json).map_err(|e| AuthError::Json {
            context: format!("keychain entry for service {:?}", self.service),
            source: e,
        })
    }
}

/// Chromium LocalStorage LevelDB path under the Notion Calendar app support directory.
pub struct DesktopAppSource {
    pub app_support_dir: PathBuf,
}

impl DesktopAppSource {
    pub fn macos_default() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        Self {
            app_support_dir: PathBuf::from(home)
                .join("Library/Application Support/Notion Calendar"),
        }
    }

    /// `NCAL_DESKTOP_SUPPORT_DIR`, else macOS default app support path, else current dir (usually yields `NoCredentials`).
    pub fn from_env_or_default() -> Self {
        if let Ok(p) = std::env::var("NCAL_DESKTOP_SUPPORT_DIR") {
            return Self {
                app_support_dir: PathBuf::from(p),
            };
        }
        #[cfg(target_os = "macos")]
        {
            Self::macos_default()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                app_support_dir: PathBuf::from("."),
            }
        }
    }
}

const LEVELDB_KEY_SUFFIX: &[u8] = b"_https://calendar.notion.so\x00\x01user.auth.currentUser";

impl CredentialSource for DesktopAppSource {
    fn name(&self) -> &'static str {
        "desktop-app"
    }

    fn obtain(&self) -> Result<Credentials, AuthError> {
        let db_path = self.app_support_dir.join("Local Storage/leveldb");
        if !db_path.is_dir() {
            return Err(AuthError::NoCredentials);
        }

        let tmp = tempfile::tempdir().map_err(|e| AuthError::Io {
            path: "tempdir for LevelDB copy".into(),
            source: e,
        })?;
        copy_leveldb_tree(&db_path, tmp.path())?;
        let lock = tmp.path().join("LOCK");
        let _ = fs::remove_file(&lock);

        let mut db = rusty_leveldb::DB::open(tmp.path(), rusty_leveldb::Options::default())
            .map_err(|e| AuthError::LevelDb {
                path: db_path.display().to_string(),
                reason: e.to_string(),
            })?;

        let key = LEVELDB_KEY_SUFFIX.to_vec();
        let value = db.get(&key).ok_or(AuthError::NoCredentials)?;

        // Chromium LocalStorage LevelDB values carry a 1-byte type prefix (0x01 = UTF-16 string
        // stored as UTF-8). Strip it before parsing JSON.
        let json_bytes = value
            .iter()
            .position(|&b| b == b'{')
            .map(|pos| &value[pos..])
            .unwrap_or(&value);

        let user: User = serde_json::from_slice(json_bytes).map_err(|e| AuthError::Json {
            context: format!(
                "desktop app credential in LevelDB ({}; {} bytes)",
                db_path.display(),
                json_bytes.len()
            ),
            source: e,
        })?;
        Credentials::try_from(user).map_err(|e| match e {
            AuthError::InvalidCredentials { reason, .. } => AuthError::InvalidCredentials {
                origin: "desktop app LevelDB",
                reason,
            },
            other => other,
        })
    }
}

fn copy_leveldb_tree(src: &Path, dst: &Path) -> Result<(), AuthError> {
    let io_err = |e| AuthError::Io {
        path: src.display().to_string(),
        source: e,
    };
    fs::create_dir_all(dst).map_err(io_err)?;
    for entry in fs::read_dir(src).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        let file_type = entry.file_type().map_err(io_err)?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_leveldb_tree(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path).map_err(io_err)?;
        }
    }
    Ok(())
}

/// Try each source in order. First success wins. `NoCredentials` skips to the next source.
pub fn resolve_credentials(sources: &[&dyn CredentialSource]) -> Result<Credentials, AuthError> {
    for source in sources {
        match source.obtain() {
            Ok(creds) => {
                info!(source = source.name(), "credentials loaded");
                return Ok(creds);
            }
            Err(AuthError::NoCredentials) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(AuthError::NoCredentials)
}

pub fn store_credentials(service: &str, creds: &Credentials) -> Result<(), AuthError> {
    let entry = keyring::Entry::new(service, "default").map_err(|e| AuthError::Keychain {
        operation: "open",
        reason: e.to_string(),
    })?;
    let json = serde_json::to_string(creds).map_err(|e| AuthError::Json {
        context: "serializing credentials for keychain".into(),
        source: e,
    })?;
    entry.set_password(&json).map_err(|e| AuthError::Keychain {
        operation: "store",
        reason: e.to_string(),
    })?;
    Ok(())
}

pub fn delete_credentials(service: &str) -> Result<(), AuthError> {
    let entry = keyring::Entry::new(service, "default").map_err(|e| AuthError::Keychain {
        operation: "open",
        reason: e.to_string(),
    })?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AuthError::Keychain {
            operation: "delete",
            reason: e.to_string(),
        }),
    }
}
