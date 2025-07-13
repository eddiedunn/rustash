use super::{add::AddCommand, list::ListCommand, use_snippet::UseCommand};
use anyhow::Result;
use clap::{Args, Subcommand};
use rustash_core::storage::StorageBackend;
use std::sync::Arc;

#[derive(Args)]
pub struct SnippetCommand {
    #[command(subcommand)]
    pub command: SnippetCommands,
}

#[derive(Subcommand)]
pub enum SnippetCommands {
    /// Add a new snippet
    Add(AddCommand),
    /// List and search snippets
    List(ListCommand),
    /// Use a snippet (expand and copy to clipboard)
    Use(UseCommand),
}

impl SnippetCommand {
    pub async fn execute(self, backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        match self.command {
            SnippetCommands::Add(cmd) => cmd.execute(backend).await,
            SnippetCommands::List(cmd) => cmd.execute(backend).await,
            SnippetCommands::Use(cmd) => cmd.execute(backend).await,
        }
    }
}
