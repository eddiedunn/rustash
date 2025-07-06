//! List snippets command

use crate::fuzzy::fuzzy_select_snippet;
use crate::utils::format_snippet_list;
use anyhow::Result;
use clap::Args;
use rustash_core::{establish_connection, list_snippets_with_tags};

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
}

impl ListCommand {
    pub fn execute(self) -> Result<()> {
        let mut conn = establish_connection()?;

        // Get snippets with filtering and searching
        let snippets = list_snippets_with_tags(
            &mut conn,
            self.filter.as_deref(),
            self.tag.as_deref(),
            Some(self.limit),
        )?;

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
                        if let Some(id) = selected.id {
                            println!("{}", id);
                        }
                    }
                    _ => format_snippet_list(&[selected], &self.format)?,
                }
            }
        } else {
            // Display all results
            match self.format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&snippets)?),
                "ids" => {
                    for snippet in snippets {
                        if let Some(id) = snippet.id {
                            println!("{}", id);
                        }
                    }
                }
                _ => format_snippet_list(&snippets, &self.format)?,
            }
        }

        Ok(())
    }
}
