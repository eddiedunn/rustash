// crates/rustash-cli/src/commands/stash_cmds.rs

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use rustash_core::{
    config::{load_config, save_config, Config},
    ServiceType, StashConfig,
};

#[derive(Args)]
pub struct StashCommand {
    #[command(subcommand)]
    pub command: StashCommands,
}

#[derive(Subcommand)]
pub enum StashCommands {
    /// List all configured stashes
    List,
    /// Add a new stash to your configuration
    Add(AddArgs),
    /// Remove an existing stash from your configuration
    Remove(RemoveArgs),
    /// Set the default stash
    SetDefault(SetDefaultArgs),
}

#[derive(Args)]
pub struct AddArgs {
    /// A unique name for the new stash
    pub name: String,
    /// The type of service this stash will provide
    #[arg(long, value_enum)]
    pub service_type: ServiceType,
    /// The database connection URL for this stash
    #[arg(long)]
    pub database_url: String,
}

#[derive(Args)]
pub struct RemoveArgs {
    /// The name of the stash to remove
    pub name: String,
}

#[derive(Args)]
pub struct SetDefaultArgs {
    /// The name of the stash to set as the default
    pub name: String,
}

pub async fn execute_stash_command(command: StashCommands, mut config: Config) -> Result<()> {
    match command {
        StashCommands::List => {
            println!("Available Stashes:");
            if config.stashes.is_empty() {
                println!(
                    "\nNo stashes configured. Use 'rustash stash add <name> ...' to create one."
                );
                return Ok(());
            }

            // Determine max name length for alignment
            let max_len = config.stashes.keys().map(String::len).max().unwrap_or(0);

            for (name, conf) in &config.stashes {
                let is_default = config.default_stash.as_deref() == Some(name);
                let default_str = if is_default { "(default)" } else { "" };
                println!(
                    "  - {:<width$}{:<10} [type: {:?}, db: {}]",
                    name,
                    default_str,
                    conf.service_type,
                    conf.database_url,
                    width = max_len + 2
                );
            }
        }
        StashCommands::Add(args) => {
            if config.stashes.contains_key(&args.name) {
                bail!(
                    "A stash named '{}' already exists. Use a different name.",
                    args.name
                );
            }
            let new_config = StashConfig {
                service_type: args.service_type,
                database_url: args.database_url,
            };
            config.stashes.insert(args.name.clone(), new_config);
            println!("✓ Stash '{}' added.", args.name);

            // If it's the first stash, make it the default
            if config.default_stash.is_none() {
                config.default_stash = Some(args.name.clone());
                println!("✓ Stash '{}' set as the default.", args.name);
            }
            save_config(&config)?;
        }
        StashCommands::Remove(args) => {
            if config.stashes.remove(&args.name).is_none() {
                bail!("Stash '{}' not found.", args.name);
            }
            println!("✓ Stash '{}' removed.", args.name);

            // If the removed stash was the default, clear the default
            if config.default_stash.as_deref() == Some(&args.name) {
                config.default_stash = None;
                println!("! Default stash has been cleared.");
            }
            save_config(&config)?;
        }
        StashCommands::SetDefault(args) => {
            if !config.stashes.contains_key(&args.name) {
                bail!("Stash '{}' not found.", args.name);
            }
            config.default_stash = Some(args.name.clone());
            println!("✓ Default stash set to '{}'.", args.name);
            save_config(&config)?;
        }
    }
    Ok(())
}
