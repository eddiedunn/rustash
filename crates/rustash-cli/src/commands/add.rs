//! Add snippet command

use crate::db;
use anyhow::Result;
use clap::Args;
use rustash_core::{add_snippet, models::Snippet};
use uuid::Uuid;

// Conditionally compile the GUI module usage
#[cfg(feature = "gui")]
use crate::gui;

#[derive(Args)]
pub struct AddCommand {
    /// Title of the snippet
    #[arg(short = 'i', long)]
    pub title: Option<String>,

    /// Content of the snippet
    #[arg(short, long)]
    pub content: Option<String>,

    /// Tags for the snippet
    #[arg(short, long, value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Read content from stdin instead of command line
    #[arg(long)]
    pub stdin: bool,
}

impl AddCommand {
    pub async fn execute(self) -> Result<()> {
        // If we're reading from stdin, use CLI mode
        if self.stdin {
            return self.execute_cli().await;
        }

        // If both title and content are provided, use CLI mode
        if self.title.is_some() && self.content.is_some() {
            self.execute_cli().await
        } else if self.title.is_none() && self.content.is_none() {
            // If neither is provided, launch the GUI
            self.launch_gui().await
        } else {
            // If only one is provided, show an error
            anyhow::bail!("Both --title and --content must be provided for command-line mode")
        }
    }

    /// Handles the command-line logic for adding a snippet.
    async fn execute_cli(self) -> Result<()> {
        let title = self.title.unwrap_or_default();
        let content = if self.stdin {
            use std::io::{self, Read};
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer.trim().to_string()
        } else {
            self.content.unwrap_or_default()
        };

        if title.trim().is_empty() {
            anyhow::bail!("Title cannot be empty for CLI usage.");
        }
        if content.trim().is_empty() {
            anyhow::bail!("Content cannot be empty for CLI usage.");
        }

        let pool = db::get_pool().await?;
        let new_snippet =
            Snippet::with_uuid(Uuid::new_v4(), title.clone(), content, self.tags.clone());
        let snippet = add_snippet(&pool, new_snippet).await?;
        println!(
            "✓ Added snippet '{}' with ID: {}",
            snippet.title, snippet.uuid
        );

        // The original `tags` is a JSON string, so we need to parse it to display nicely.
        let snippet_tags: Vec<String> = serde_json::from_str(&snippet.tags).unwrap_or_default();
        if !snippet_tags.is_empty() {
            println!("  Tags: {}", snippet_tags.join(", "));
        }

        Ok(())
    }

    /// Launches the GUI. This function is only compiled if the 'gui' feature is enabled.
    #[cfg(feature = "gui")]
    async fn launch_gui(&self) -> Result<()> {
        println!("No arguments provided. Launching GUI to add snippet...");

        // Launch the GUI window and wait for it to close.
        // It returns data for the new snippet if the user saved it.
        if let Some(new_snippet_data) = gui::show_add_window()? {
            // The GUI returns the data; the CLI is responsible for saving it.
            let pool = db::get_pool().await?;
            let snippet_to_add = Snippet::with_uuid(
                Uuid::new_v4(),
                new_snippet_data.title,
                new_snippet_data.content,
                new_snippet_data.tags,
            );
            add_snippet(&pool, snippet_to_add).await?;
            println!("✓ Snippet added via GUI.");
        } else {
            println!("Operation cancelled.");
        }
        Ok(())
    }

    /// Fallback function if the 'gui' feature is disabled at compile time.
    #[cfg(not(feature = "gui"))]
    async fn launch_gui(&self) -> Result<()> {
        anyhow::bail!(
            "No arguments provided. To use the GUI, please compile with the 'gui' feature."
        )
    }
}
