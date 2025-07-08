//! Add snippet command

use anyhow::Result;
use crate::db;
use clap::Args;
use rustash_core::{add_snippet, models::Snippet};
use uuid::Uuid;

#[derive(Args)]
pub struct AddCommand {
    /// Title of the snippet
    pub title: String,

    /// Content of the snippet
    pub content: String,

    /// Tags for the snippet
    #[arg(short, long, value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Read content from stdin instead of command line
    #[arg(long)]
    pub stdin: bool,
}

impl AddCommand {
    pub fn execute(self) -> Result<()> {
        let content = if self.stdin {
            // Read content from stdin
            use std::io::{self, Read};
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer.trim().to_string()
        } else {
            self.content
        };

        // Validate input
        if self.title.trim().is_empty() {
            anyhow::bail!("Title cannot be empty");
        }

        if content.trim().is_empty() {
            anyhow::bail!("Content cannot be empty");
        }

        // Create connection from pool
        let mut conn = db::get_connection()?;

        // Create new snippet with UUID
        let new_snippet = Snippet::with_uuid(Uuid::new_v4(), self.title.clone(), content, self.tags.clone());

        // Add to database and get the created snippet
        let snippet = add_snippet(&mut *conn, new_snippet)?;

        // Print success message
        println!("✓ Added snippet '{}' with ID: {}", snippet.title, snippet.uuid);
        if !snippet.tags.is_empty() {
            println!("  Tags: {}", snippet.tags);
        }

        Ok(())
    }
}
