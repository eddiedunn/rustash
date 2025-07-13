// crates/rustash-cli/src/commands/snippets.rs
use super::SnippetCommands;
use anyhow::Result;
use rustash_core::storage::StorageBackend;
use std::sync::Arc;

pub async fn execute_snippet_command(
    command: SnippetCommands,
    backend: Arc<Box<dyn StorageBackend>>,
) -> Result<()> {
    match command {
        SnippetCommands::Add(cmd) => cmd.execute(backend).await,
        SnippetCommands::List(cmd) => cmd.execute(backend).await,
        SnippetCommands::Use(cmd) => cmd.execute(backend).await,
    }
}
