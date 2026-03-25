use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::url_pattern::UrlPattern;

/// Configuration structure for persistent settings
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    pub last_browser: Option<String>,
    pub last_profile: Option<String>,
    pub last_container: Option<String>,
    pub last_incognito: bool,
    pub last_new_window: bool,
    /// URL patterns for auto-launching browsers
    pub url_patterns: Vec<UrlPattern>,
}

impl Config {
    /// Returns the path to the configuration file
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .expect("Could not find config directory")
            .join("bopen")
            .join("config.json")
    }

    /// Load configuration from file, returning default if file doesn't exist or is invalid
    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&data) {
                return config;
            }
        }
        Self::default()
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&path, data)?;
        Ok(())
    }
}
