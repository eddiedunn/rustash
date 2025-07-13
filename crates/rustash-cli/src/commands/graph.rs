use anyhow::Result;
use clap::{Args, Subcommand};
use rustash_core::{models::Snippet, storage::StorageBackend, Uuid};
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
        from: Uuid,
        to: Uuid,
        #[arg(short, long, default_value = "RELATED_TO")]
        relation: String,
    },
    /// Find items related to a given item
    Neighbors {
        id: Uuid,
        #[arg(short, long)]
        relation: Option<String>,
    },
}

impl GraphCommand {
    pub async fn execute(self, backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        match self.command {
            GraphSubcommand::Link { from, to, relation } => {
                backend.add_relation(&from, &to, &relation).await?;
                println!("\u{2713} Linked {} -[{}]-> {}", from, relation, to);
            }
            GraphSubcommand::Neighbors { id, relation } => {
                let results = backend.get_related(&id, relation.as_deref()).await?;
                if results.is_empty() {
                    println!("No related items found for {}.", id);
                } else {
                    println!("Found {} related items for {}:", results.len(), id);
                    for item in results {
                        if let Some(snippet) = item.as_any().downcast_ref::<Snippet>() {
                            println!("  - {} ({})", snippet.uuid, snippet.title);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
