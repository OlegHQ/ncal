use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use ncal_api::types::{Calendar, Event, SyncToken};

use crate::config::AppConfig;
use crate::CliError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheFile {
    /// Epoch millis when this cache was last written.
    pub updated_at: i64,
    pub sync_tokens: Vec<SyncToken>,
    #[serde(default)]
    pub calendars: Vec<Calendar>,
    #[serde(default)]
    pub events: Vec<Event>,
}

pub fn cache_path(config: &AppConfig) -> PathBuf {
    config.cache_file()
}

/// Read the cache file. Returns `None` if missing, corrupt, or unreadable.
pub fn read_cache(config: &AppConfig) -> Option<CacheFile> {
    let path = cache_path(config);
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Read the cache only if it exists and is fresh (within `max_age` seconds).
pub fn read_fresh_cache(config: &AppConfig) -> Option<CacheFile> {
    let cache = read_cache(config)?;
    if is_fresh(&cache, config.cache_max_age()) {
        Some(cache)
    } else {
        None
    }
}

pub fn is_fresh(cache: &CacheFile, max_age_secs: u64) -> bool {
    let now = chrono::Utc::now().timestamp_millis();
    let age_ms = now.saturating_sub(cache.updated_at);
    age_ms < (max_age_secs as i64) * 1000
}

pub fn write_cache(config: &AppConfig, data: &CacheFile) -> Result<(), CliError> {
    let path = cache_path(config);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(CliError::Io)?;
    }
    std::fs::write(&path, serde_json::to_string(data)?).map_err(CliError::Io)
}
