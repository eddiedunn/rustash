//! Rustash CLI Application

mod commands;
mod fuzzy;
#[cfg(feature = "gui")]
mod gui;
mod utils;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use commands::SnippetCommands;
use rustash_core::stash::{ServiceType, Stash};
use std::sync::Arc;

// Command-line interface definition
#[derive(Parser)]
#[command(name = "rustash")]
#[command(about = "A developer-first, multi-modal data stash.")]
#[command(version)]
pub struct Cli {
    /// The name of the stash to use.
    /// If not provided, uses the default_stash from your config.
    #[arg(long, short, global = true, env = "RUSTASH_STASH")]
    pub stash: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

// Top-level commands
#[derive(Subcommand)]
pub enum Commands {
    /// Manage snippets in a Snippet-type stash
    #[command(alias = "s")]
    Snippets(commands::SnippetCommand),

    /// Manage stashes
    #[command(alias = "st")]
    Stash(commands::StashCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = rustash_core::config::load_config()?;

    if let Commands::Stash(cmd) = cli.command {
        return commands::stash_cmds::execute_stash_command(cmd.command, config).await;
    }

    let stash_name = cli.stash.or(config.default_stash).context(
        "No stash specified and no default_stash is set. Use `rustash stash list` to see options.",
    )?;

    let stash_config = config
        .stashes
        .get(&stash_name)
        .with_context(|| format!("Stash '{}' not found in your configuration.", stash_name))?;

    let stash = Arc::new(Stash::new(&stash_name, stash_config.clone()).await?);

    match cli.command {
        Commands::Snippets(cmd) => {
            anyhow::ensure!(
                stash.config.service_type == ServiceType::Snippet,
                "The stash '{}' is a '{:?}' stash, but this command requires a 'Snippet' stash.",
                stash.name,
                stash.config.service_type
            );
            cmd.execute(stash.backend.clone()).await?;
        }
        Commands::Stash(cmd) => {
            commands::stash_cmds::execute_stash_command(cmd.command, config).await?;
        }
    }

    Ok(())
}
