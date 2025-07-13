// crates/rustash-core/src/config.rs

use crate::stash::StashConfig;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub default_stash: Option<String>,
    #[serde(default)]
    pub stashes: HashMap<String, StashConfig>,
}

fn get_config_path() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or_else(|| crate::Error::other("Could not determine config directory"))?
        .join("rustash/stashes.toml"))
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Ok(Config {
            default_stash: None,
            stashes: HashMap::new(),
        });
    }

    let config_str = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)
        .map_err(|e| crate::Error::other(format!("Failed to parse stashes.toml: {}", e)))?;

    Ok(config)
}

/// Saves the given configuration to the stashes.toml file.
pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;

    // Ensure the parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| crate::Error::other(format!("Failed to serialize config to TOML: {}", e)))?;

    std::fs::write(config_path, toml_string)?;

    Ok(())
}
