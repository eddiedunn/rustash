//! Add snippet command

use anyhow::Result;
use clap::Args;
use rustash_core::{models::Snippet, storage::StorageBackend};
use std::sync::Arc;
use uuid::Uuid;

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
    pub async fn execute(self, backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        if self.stdin || (self.title.is_some() && self.content.is_some()) {
            self.execute_cli(backend).await
        } else if self.title.is_none() && self.content.is_none() {
            self.launch_gui(backend).await
        } else {
            anyhow::bail!("Both --title and --content must be provided for command-line mode")
        }
    }

    async fn execute_cli(self, backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        let title = self.title.unwrap_or_default();
        let content = if self.stdin {
            use std::io::{self, Read};
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer.trim().to_string()
        } else {
            self.content.unwrap_or_default()
        };

        anyhow::ensure!(
            !title.trim().is_empty(),
            "Title cannot be empty for CLI usage."
        );
        anyhow::ensure!(
            !content.trim().is_empty(),
            "Content cannot be empty for CLI usage."
        );

        let new_snippet =
            Snippet::with_uuid(Uuid::new_v4(), title.clone(), content, self.tags.clone());
        backend.save(&new_snippet).await?;
        println!("\u{2713} Added snippet '{}' to stash.", new_snippet.title);
        Ok(())
    }

    #[cfg(feature = "gui")]
    async fn launch_gui(&self, backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        println!("Launching GUI to add snippet...");
        if let Some(data) = gui::show_add_window()? {
            let snippet = Snippet::with_uuid(Uuid::new_v4(), data.title, data.content, data.tags);
            backend.save(&snippet).await?;
            println!("\u{2713} Snippet added via GUI.");
        } else {
            println!("Operation cancelled.");
        }
        Ok(())
    }

    #[cfg(not(feature = "gui"))]
    async fn launch_gui(&self, _backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        anyhow::bail!("No arguments provided. To use the GUI, recompile with the 'gui' feature.")
    }
}
