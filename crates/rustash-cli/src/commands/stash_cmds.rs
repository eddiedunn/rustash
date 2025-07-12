// crates/rustash-cli/src/commands/stash_cmds.rs

use anyhow::Result;
use clap::Subcommand;
use rustash_core::config::Config;

#[derive(Subcommand)]
pub enum StashCommands {
    /// List all configured stashes
    List,
}

pub async fn execute_stash_command(command: StashCommands, config: Config) -> Result<()> {
    match command {
        StashCommands::List => {
            println!("Available Stashes:");
            if config.stashes.is_empty() {
                println!("  No stashes configured. You can add one to ~/.config/rustash/stashes.toml");
                return Ok(());
            }
            for (name, conf) in config.stashes {
                let is_default = config.default_stash.as_deref() == Some(&name);
                let default_str = if is_default { " (default)" } else { "" };
                println!(
                    "  - {}{:<15} [type: {:?}, db: {}]",
                    name,
                    default_str,
                    conf.service_type,
                    conf.database_url
                );
            }
        }
    }
    Ok(())
}
