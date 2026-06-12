use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const DEFAULT_SERVICE: &str = "notion-calendar-cli";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub auth: AuthSection,
    #[serde(default)]
    pub defaults: DefaultsSection,
    #[serde(default)]
    pub sync: SyncSection,
    #[serde(default)]
    pub cache: CacheSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSection {
    #[serde(default = "default_keychain_service")]
    pub keychain_service: String,
    #[serde(default = "default_credentials_file")]
    pub credentials_file: PathBuf,
}

fn default_keychain_service() -> String {
    DEFAULT_SERVICE.to_string()
}

impl Default for AuthSection {
    fn default() -> Self {
        Self {
            keychain_service: default_keychain_service(),
            credentials_file: default_credentials_file(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsSection {
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub calendar: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSection {
    #[serde(default = "default_sync_tokens_file")]
    pub tokens_file: PathBuf,
    #[serde(default = "default_sync_interval")]
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSection {
    #[serde(default = "default_cache_file")]
    pub file: PathBuf,
    /// Maximum age in seconds before the cache is considered stale (default: 300).
    #[serde(default = "default_cache_max_age")]
    pub max_age: u64,
}

fn default_cache_file() -> PathBuf {
    dirs()
        .map(|d| d.data_dir().join("cache.json"))
        .unwrap_or_else(|| PathBuf::from("cache.json"))
}

fn default_credentials_file() -> PathBuf {
    dirs()
        .map(|d| d.config_dir().join("credentials.json"))
        .unwrap_or_else(|| PathBuf::from("credentials.json"))
}

fn default_cache_max_age() -> u64 {
    300
}

impl Default for CacheSection {
    fn default() -> Self {
        Self {
            file: default_cache_file(),
            max_age: default_cache_max_age(),
        }
    }
}

fn default_sync_tokens_file() -> PathBuf {
    dirs()
        .map(|d| d.config_dir().join("ncal").join("sync-tokens.json"))
        .unwrap_or_else(|| PathBuf::from("sync-tokens.json"))
}

fn default_sync_interval() -> u64 {
    60
}

impl Default for SyncSection {
    fn default() -> Self {
        Self {
            tokens_file: default_sync_tokens_file(),
            interval: default_sync_interval(),
        }
    }
}

fn dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("so", "notion", "ncal")
}

impl AppConfig {
    pub fn load(explicit: Option<&Path>) -> Result<Self, String> {
        let path = if let Some(p) = explicit {
            p.to_path_buf()
        } else {
            dirs()
                .map(|d| d.config_dir().join("config.toml"))
                .ok_or_else(|| "cannot resolve config directory".to_string())?
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let raw =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        toml::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
    }

    pub fn tokens_file(&self) -> PathBuf {
        self.sync.tokens_file.clone()
    }

    pub fn cache_file(&self) -> PathBuf {
        self.cache.file.clone()
    }

    pub fn cache_max_age(&self) -> u64 {
        self.cache.max_age
    }
}
