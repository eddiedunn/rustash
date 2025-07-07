//! Add snippet command

use anyhow::Result;
use clap::Args;
use crate::db;
use rustash_core::{add_snippet, models::NewDbSnippet};

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

        // Create new snippet
        let new_snippet = NewSnippet::new(self.title.clone(), content, self.tags.clone());

        // Add to database and get the created snippet
        let snippet = add_snippet(&mut *conn, new_snippet)?;

        // Print success message
        if let Some(id) = snippet.id {
            println!("✓ Added snippet '{}' with ID: {}", self.title, id);

            if !self.tags.is_empty() {
                println!("  Tags: {}", self.tags.join(", "));
            }
        } else {
            println!("✓ Added snippet '{}'", self.title);
        }

        Ok(())
    }
}
