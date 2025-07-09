//! List snippets command

use crate::db;
use crate::fuzzy::fuzzy_select_snippet;
use crate::utils::format_snippet_list;
use anyhow::Result;
use clap::Args;
use rustash_core::list_snippets_with_tags;

#[derive(Args)]
pub struct ListCommand {
    /// Filter snippets by text (title or content)
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Filter snippets by tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// Maximum number of results to show
    #[arg(short, long, default_value = "50")]
    pub limit: i64,

    /// Use fuzzy finder for interactive selection
    #[arg(long)]
    pub interactive: bool,

    /// Output format: table, json, ids
    #[arg(long, default_value = "table")]
    pub format: String,

    /// Use UUIDs for snippet IDs
    #[arg(long)]
    pub uuid: bool,
}

impl ListCommand {
    pub async fn execute(self) -> Result<()> {
        let pool = db::get_pool().await?;

        // Get snippets with filtering and searching
        let snippets = list_snippets_with_tags(
            &pool,
            self.filter.as_deref(),
            self.tag.as_deref(),
            Some(self.limit),
        )
        .await?;

        if snippets.is_empty() {
            println!("No snippets found.");
            return Ok(());
        }

        if self.interactive {
            // Use fuzzy finder for selection
            if let Some(selected) = fuzzy_select_snippet(&snippets)? {
                match self.format.as_str() {
                    "json" => println!("{}", serde_json::to_string_pretty(&selected)?),
                    "ids" => {
                        println!("{}", selected.id);
                    }
                    _ => format_snippet_list(&[selected], &self.format)?,
                }
            }
        } else {
            // Display all results
            match self.format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&snippets)?),
                "ids" => {
                    for snippet in &snippets {
                        println!("{}", snippet.id);
                    }
                }
                _ => format_snippet_list(&snippets, &self.format)?,
            }
        }

        Ok(())
    }
}
