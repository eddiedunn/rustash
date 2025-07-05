//! Configuration management utilities

use anyhow::Result;
use rustash_core::Config;
use std::path::Path;

/// Load configuration from a TOML file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let contents = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

/// Save configuration to a TOML file
pub fn save_config<P: AsRef<Path>>(config: &Config, path: P) -> Result<()> {
    let contents = toml::to_string_pretty(config)?;
    std::fs::write(path, contents)?;
    Ok(())
}

/// Load configuration with environment variable overrides
pub fn load_config_with_env<P: AsRef<Path>>(path: P) -> Result<Config> {
    let mut config = if path.as_ref().exists() {
        load_config(path)?
    } else {
        Config::default()
    };
    
    // Override with environment variables
    if let Ok(database_url) = std::env::var("RUSTASH_DATABASE_URL") {
        config.database_url = database_url;
    }
    
    if let Ok(vector_search) = std::env::var("RUSTASH_VECTOR_SEARCH") {
        config.vector_search = vector_search.parse().unwrap_or(false);
    }
    
    Ok(config)
}