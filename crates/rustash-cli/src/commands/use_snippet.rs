//! Use snippet command

use crate::utils::copy_to_clipboard;
use anyhow::Result;
use clap::Args;
use dialoguer::Input;
use crate::db;
use regex::Regex;
use rustash_core::{SnippetWithTags, expand_placeholders, get_snippet_by_id};
use std::collections::HashMap;

#[derive(Args)]
pub struct UseCommand {
    /// ID of the snippet to use
    pub id: i32,

    /// Variables for placeholder expansion (key=value format)
    #[arg(short, long, value_parser = parse_variable)]
    pub var: Vec<(String, String)>,

    /// Copy to clipboard after expansion
    #[arg(short, long, default_value = "true")]
    pub copy: bool,

    /// Interactive mode: prompt for missing variables
    ///
    /// Alias: --fzf (for fzf-like interactive selection)
    #[arg(short, long, alias = "fzf")]
    pub interactive: bool,

    /// Just print the expanded content without copying
    #[arg(long)]
    pub print_only: bool,
}

impl UseCommand {
    pub fn execute(self) -> Result<()> {
        let mut conn = db::get_connection()?;

        // Get the snippet
        let snippet = get_snippet_by_id(&mut *conn, self.id)?
            .ok_or_else(|| anyhow::anyhow!("Snippet with ID {} not found", self.id))?;

        let snippet_with_tags = SnippetWithTags::from(snippet);
        let _content = snippet_with_tags.content.clone();

        // Build variables map
        let mut variables: HashMap<String, String> = self.var.into_iter().collect();

        // Extract placeholders from content
        let placeholders = extract_placeholders(&snippet_with_tags.content);

        // Interactive mode: prompt for missing variables
        if self.interactive {
            for placeholder in &placeholders {
                if !variables.contains_key(placeholder) {
                    let value: String = Input::new()
                        .with_prompt(format!("Enter value for '{}'", placeholder))
                        .interact_text()?;
                    variables.insert(placeholder.clone(), value);
                }
            }
        }

        // Expand placeholders
        let expanded_content = expand_placeholders(&snippet_with_tags.content, &variables);

        // Check for remaining placeholders
        let remaining_placeholders = extract_placeholders(&expanded_content);
        if !remaining_placeholders.is_empty() && !self.interactive {
            eprintln!("Warning: The following placeholders were not expanded:");
            for placeholder in remaining_placeholders {
                eprintln!("  {{{{ {} }}}}", placeholder);
            }
            eprintln!("Use --interactive or provide values with --var key=value");
        }

        // Output results
        if self.print_only {
            println!("{}", expanded_content);
        } else {
            println!("Snippet: {}", snippet_with_tags.title);
            if !snippet_with_tags.tags.is_empty() {
                println!("Tags: {}", snippet_with_tags.tags.join(", "));
            }
            println!();
            println!("{}", expanded_content);

            if self.copy {
                copy_to_clipboard(&expanded_content)?;
                println!("\n✓ Copied to clipboard");
            }
        }

        Ok(())
    }
}

fn parse_variable(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid variable format '{}'. Use key=value", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn extract_placeholders(content: &str) -> Vec<String> {
    // Use a regex to find all occurrences of {{variable_name}}
    let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    
    // Collect all captured variable names
    let mut placeholders = re.captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect::<Vec<_>>();
    
    // Deduplicate and sort for consistent order
    placeholders.sort();
    placeholders.dedup();
    placeholders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_placeholders() {
        let content = "Hello {{name}}, welcome to {{place}}! {{name}} is great.";
        let mut placeholders = extract_placeholders(content);
        placeholders.sort();
        assert_eq!(placeholders, vec!["name", "place"]);
    }

    #[test]
    fn test_extract_placeholders_with_spaces() {
        let content = "{{ username }} and {{ project_name }}";
        let mut placeholders = extract_placeholders(content);
        placeholders.sort();
        assert_eq!(placeholders, vec!["project_name", "username"]);
    }

    #[test]
    fn test_parse_variable() {
        assert_eq!(
            parse_variable("name=Alice").unwrap(),
            ("name".to_string(), "Alice".to_string())
        );

        assert_eq!(
            parse_variable("url=https://example.com").unwrap(),
            ("url".to_string(), "https://example.com".to_string())
        );

        assert!(parse_variable("invalid").is_err());
    }
}
