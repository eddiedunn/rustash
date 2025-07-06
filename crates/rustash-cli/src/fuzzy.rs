//! Fuzzy finder integration

use anyhow::Result;
use rustash_core::SnippetWithTags;
use skim::prelude::*;

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
            let id_str =
                s.id.map(|id| id.to_string())
                    .unwrap_or_else(|| "?".to_string());
            let tags_str = if s.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", s.tags.join(", "))
            };

            format!("{}: {}{}", id_str, s.title, tags_str)
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

        // Parse the ID from the selected text (format: "ID: title [tags]")
        if let Some(colon_pos) = selected_text.find(':') {
            let id_str = &selected_text[..colon_pos];
            if let Ok(id) = id_str.parse::<i32>() {
                // Find the snippet with this ID
                if let Some(snippet) = snippets.iter().find(|s| s.id == Some(id)) {
                    return Ok(Some(snippet.clone()));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[cfg(test)]
    #[allow(dead_code)]
    fn create_test_snippet(
        id: i32,
        title: &str,
        content: &str,
        tags: Vec<String>,
    ) -> SnippetWithTags {
        SnippetWithTags {
            id: Some(id),
            title: title.to_string(),
            content: content.to_string(),
            tags,
            embedding: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
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
