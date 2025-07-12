//! Use snippet command

use crate::utils::copy_to_clipboard;
use anyhow::{Context, Result};
use clap::Args;
use dialoguer::Input;
use regex::Regex;
use rustash_core::{expand_placeholders, models::SnippetWithTags, storage::StorageBackend};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Args)]
pub struct UseCommand {
    /// UUID of the snippet to use
    pub uuid: String,
    #[arg(short, long, value_parser = parse_variable)]
    pub var: Vec<(String, String)>,
    #[arg(short, long, default_value = "true")]
    pub copy: bool,
    #[arg(short, long, alias = "fzf")]
    pub interactive: bool,
    #[arg(long)]
    pub print_only: bool,
}

impl UseCommand {
    pub async fn execute(self, backend: Arc<Box<dyn StorageBackend>>) -> Result<()> {
        let snippet_uuid = self.uuid.parse::<Uuid>().context("Invalid UUID format")?;
        
        let snippet_dyn = backend.get(&snippet_uuid).await?.context("Snippet not found")?;
        let snippet = snippet_dyn.as_any().downcast_ref::<SnippetWithTags>().context("Internal error: Could not downcast to SnippetWithTags")?.clone();

        let mut variables: HashMap<String, String> = self.var.into_iter().collect();
        let placeholders = extract_placeholders(&snippet.content);

        if self.interactive {
            for placeholder in &placeholders {
                if !variables.contains_key(placeholder) {
                    let value: String = Input::new().with_prompt(format!("Enter value for '{}'", placeholder)).interact_text()?;
                    variables.insert(placeholder.clone(), value);
                }
            }
        }

        let expanded_content = expand_placeholders(&snippet.content, &variables);

        if self.print_only {
            println!("{}", expanded_content);
        } else {
            println!("Snippet: {}", snippet.title);
            copy_to_clipboard(&expanded_content)?;
            println!("\n\u2713 Copied to clipboard");
        }

        Ok(())
    }
}

// Helper functions (parse_variable, extract_placeholders) remain the same
fn parse_variable(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid variable format '{}'. Use key=value", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn extract_placeholders(content: &str) -> Vec<String> {
    let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    let mut placeholders: Vec<String> = re.captures_iter(content).map(|cap| cap[1].to_string()).collect();
    placeholders.sort();
    placeholders.dedup();
    placeholders
}
