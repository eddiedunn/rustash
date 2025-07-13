// crates/rustash-cli/src/commands/snippets.rs
use super::{SnippetCommand, SnippetCommands};
use anyhow::Result;
use rustash_core::storage::StorageBackend;
use std::sync::Arc;

impl SnippetCommand {
    pub async fn execute(self, backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        match self.command {
            SnippetCommands::Add(cmd) => cmd.execute(backend).await,
            SnippetCommands::List(cmd) => cmd.execute(backend).await,
            SnippetCommands::Use(cmd) => cmd.execute(backend).await,
        }
    }
}
