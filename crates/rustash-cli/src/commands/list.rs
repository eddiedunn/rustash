//! List snippets command

use crate::fuzzy::fuzzy_select_snippet;
use crate::utils::format_snippet_list;
use anyhow::Result;
use clap::Args;
use rustash_core::{models::Query, storage::StorageBackend};
use std::sync::Arc;

#[derive(Args)]
pub struct ListCommand {
    #[arg(short, long)]
    pub filter: Option<String>,
    #[arg(short, long)]
    pub tag: Option<String>,
    #[arg(short, long, default_value = "50")]
    pub limit: usize,
    #[arg(long)]
    pub interactive: bool,
    #[arg(long, default_value = "table")]
    pub format: String,
}

impl ListCommand {
    pub async fn execute(self, backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        let query = Query {
            text_filter: self.filter,
            tags: self.tag.map(|t| vec![t]),
            limit: Some(self.limit),
            ..Default::default()
        };

        let snippets_dyn = backend.query(&query).await?;
        let snippets: Vec<_> = snippets_dyn
            .iter()
            .filter_map(|item| item.as_any().downcast_ref::<rustash_core::SnippetWithTags>().cloned())
            .collect();

        if snippets.is_empty() {
            println!("No snippets found.");
            return Ok(());
        }

        if self.interactive {
            if let Some(selected) = fuzzy_select_snippet(&snippets)? {
                format_snippet_list(&[selected], "detailed")?;
            }
        } else {
            format_snippet_list(&snippets, &self.format)?;
        }

        Ok(())
    }
}
