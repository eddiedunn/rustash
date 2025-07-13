use anyhow::Result;
use clap::{Args, Subcommand};
use rustash_core::storage::StorageBackend;
use std::sync::Arc;

#[derive(Args)]
pub struct RagCommand {
    #[command(subcommand)]
    pub command: RagSubcommand,
}

#[derive(Subcommand)]
pub enum RagSubcommand {
    /// Add a document to the RAG stash
    AddDocument,
}

impl RagCommand {
    pub async fn execute(self, _backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        match self.command {
            RagSubcommand::AddDocument => {
                println!("add-document not implemented yet");
            }
        }
        Ok(())
    }
}
