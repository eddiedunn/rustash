//! Fuzzy finder integration

use anyhow::Result;
use rustash_core::models::SnippetWithTags;
use skim::prelude::*;
use std::sync::Arc;
use std::borrow::Cow;

// Simple wrapper for skim items
#[derive(Debug, Clone)]
struct SnippetItem {
    text: String,
}

impl SkimItem for SnippetItem {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.text)
    }
}

pub fn fuzzy_select_snippet(snippets: &[SnippetWithTags]) -> Result<Option<SnippetWithTags>> {
    if snippets.is_empty() {
        return Ok(None);
    }

    // Format snippets for display
    let items: Vec<String> = snippets
        .iter()
        .map(|s| {
            let tags_str = if s.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", s.tags.join(", "))
            };

            format!("{}: {}{}", s.uuid, s.title, tags_str)
        })
        .collect();

    // Create input channel for skim
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

    // Send items to skim in a separate thread
    let tx_clone = tx.clone();
    std::thread::spawn(move || {
        for item in items {
            let _ = tx_clone.send(Arc::new(SnippetItem { text: item }) as Arc<dyn SkimItem>);
        }
        drop(tx_clone);
    });
    drop(tx);

    // Configure skim options
    let options = SkimOptionsBuilder::default()
        .height(Some("40%"))
        .multi(false)
        .prompt(Some("Select snippet: "))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build skim options: {}", e))?;

    // Run skim
    let selected_items = Skim::run_with(&options, Some(rx))
        .map(|out| out.selected_items)
        .unwrap_or_else(Vec::new);

    if let Some(item) = selected_items.first() {
        let selected_text = item.output().to_string();

        // Parse the UUID from the selected text (format: "UUID: title [tags]")
        if let Some(colon_pos) = selected_text.find(':') {
            let uuid_str = selected_text[..colon_pos].trim();
            // Find the snippet with this UUID
            if let Some(snippet) = snippets.iter().find(|s| s.uuid == uuid_str) {
                return Ok(Some(snippet.clone()));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn create_test_snippet(
        id: &str,
        title: &str,
        content: &str,
        tags: Vec<String>,
    ) -> SnippetWithTags {
        SnippetWithTags {
            uuid: id.to_string(),
            id: Uuid::parse_str(id).unwrap(),
            title: title.to_string(),
            content: content.to_string(),
            tags,
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_fuzzy_select_empty() {
        let snippets: Vec<SnippetWithTags> = vec![];
        let result = fuzzy_select_snippet(&snippets).unwrap();
        assert!(result.is_none());
    }

    // Note: Interactive tests would require a proper test environment
    // These tests mainly verify the data structures and non-interactive logic
}
