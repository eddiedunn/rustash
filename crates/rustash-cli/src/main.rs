//! Rustash CLI Application

mod commands;
mod db;
mod fuzzy;
mod utils;

use std::path::PathBuf;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use commands::{add::AddCommand, list::ListCommand, use_snippet::UseCommand};

/// Supported database backends
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum DatabaseBackend {
    /// Use SQLite (default)
    Sqlite,
    /// Use PostgreSQL with Apache AGE
    Postgres,
}

impl Default for DatabaseBackend {
    fn default() -> Self {
        Self::Sqlite
    }
}

impl std::fmt::Display for DatabaseBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite => write!(f, "sqlite"),
            Self::Postgres => write!(f, "postgres"),
        }
    }
}

/// Initialize the database connection pool
fn init_db(backend: DatabaseBackend, db_path: Option<PathBuf>) -> Result<()> {
    db::init(backend, db_path)
}

#[derive(Parser)]
#[command(name = "rustash")]
#[command(about = "A modern snippet manager for developers")]
#[command(version)]
pub struct Cli {
    /// Database backend to use
    #[arg(long, value_enum, default_value_t = DatabaseBackend::Sqlite)]
    pub db_backend: DatabaseBackend,

    /// Path to the database file (for SQLite) or connection string (for PostgreSQL)
    /// For SQLite: path to the .db file (default: ~/.rustash/rustash.db)
    /// For PostgreSQL: connection string (e.g., postgres://user:password@localhost:5432/rustash)
    #[arg(long)]
    pub db_path: Option<PathBuf>,

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
    let cli = Cli::parse();
    
    // Initialize the database connection pool with the specified backend
    init_db(cli.db_backend, cli.db_path)?;

    match cli.command {
        Commands::Add(cmd) => cmd.execute(),
        Commands::List(cmd) => cmd.execute(),
        Commands::Use(cmd) => cmd.execute(),
    }
}
