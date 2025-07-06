//! Rustash CLI Application

mod commands;
mod db;
mod fuzzy;
mod utils;

use anyhow::Result;

use clap::{Parser, Subcommand};
use commands::{add::AddCommand, list::ListCommand, use_snippet::UseCommand};

// Initialize the database connection pool
fn init_db() -> Result<()> {
    db::init()
}

#[derive(Parser)]
#[command(name = "rustash")]
#[command(about = "A modern snippet manager for developers")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new snippet
    Add(AddCommand),
    /// List and search snippets
    List(ListCommand),
    /// Use a snippet (expand and copy to clipboard)
    Use(UseCommand),
}

fn main() -> Result<()> {
    // Initialize the database connection pool
    init_db()?;
    
    let cli = Cli::parse();

    match cli.command {
        Commands::Add(cmd) => cmd.execute(),
        Commands::List(cmd) => cmd.execute(),
        Commands::Use(cmd) => cmd.execute(),
    }
}
