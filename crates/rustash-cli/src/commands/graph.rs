use anyhow::Result;
use clap::{Args, Subcommand};
use rustash_core::storage::StorageBackend;
use std::sync::Arc;

#[derive(Args)]
pub struct GraphCommand {
    #[command(subcommand)]
    pub command: GraphSubcommand,
}

#[derive(Subcommand)]
pub enum GraphSubcommand {
    /// Link two items in the knowledge graph
    Link {
        from: String,
        to: String,
        #[arg(short, long, default_value = "related")]
        relation: String,
    },
}

impl GraphCommand {
    pub async fn execute(self, _backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        match self.command {
            GraphSubcommand::Link { .. } => {
                println!("graph link not implemented yet");
            }
        }
        Ok(())
    }
}
