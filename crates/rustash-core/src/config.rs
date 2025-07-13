// crates/rustash-core/src/config.rs

use crate::stash::StashConfig;
use crate::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct Config {
    pub default_stash: Option<String>,
    #[serde(default)]
    pub stashes: HashMap<String, StashConfig>,
}

pub fn load_config() -> Result<Config> {
    let config_path = dirs::config_dir()
        .ok_or_else(|| crate::Error::other("Could not determine config directory"))?
        .join("rustash/stashes.toml");

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
    let config_path = dirs::config_dir()
        .ok_or_else(|| crate::Error::other("Could not determine config directory"))?
        .join("rustash");

    // Ensure the directory exists
    std::fs::create_dir_all(&config_path)?;

    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| crate::Error::other(format!("Failed to serialize config to TOML: {}", e)))?;

    std::fs::write(config_path.join("stashes.toml"), toml_string)?;

    Ok(())
}
