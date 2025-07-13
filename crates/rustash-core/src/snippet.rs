//! Snippet helper functions

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::{
    models::{Query, Snippet, SnippetWithTags},
    storage::StorageBackend,
};
use std::sync::Arc;
use uuid::Uuid;

/// Expand placeholders in snippet content with provided variables
///
/// # Arguments
/// * `content_str` - The content with placeholders in the format `{{key}}`
/// * `variables` - A map of variable names to their values
///
/// # Returns
/// The content with all placeholders replaced by their corresponding values
pub fn expand_placeholders(content_str: &str, variables: &HashMap<String, String>) -> String {
    let mut result = content_str.to_string();

    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }

    result
}

pub fn validate_snippet_content(snippet_title: &str, snippet_content: &str) -> Result<()> {
    if snippet_title.trim().is_empty() {
        return Err(Error::validation("Snippet title cannot be empty"));
    }

    if snippet_content.trim().is_empty() {
        return Err(Error::validation("Snippet content cannot be empty"));
    }

    if snippet_title.len() > 255 {
        return Err(Error::validation(
            "Snippet title is too long (max 255 characters)",
        ));
    }

    if snippet_content.len() > 100_000 {
        return Err(Error::validation(
            "Snippet content is too long (max 100,000 characters)",
        ));
    }

    Ok(())
}

/// High level service for snippet-related operations.
pub struct SnippetService {
    backend: Arc<Box<dyn StorageBackend>>,
}

impl SnippetService {
    /// Create a new service with the given backend.
    pub fn new(backend: Arc<Box<dyn StorageBackend>>) -> Self {
        Self { backend }
    }

    /// Retrieve a snippet by its UUID.
    pub async fn get_snippet_by_id(&self, id: &Uuid) -> Result<Option<SnippetWithTags>> {
        match self.backend.get(id).await? {
            Some(item) => Ok(item.as_any().downcast_ref::<SnippetWithTags>().cloned()),
            None => Ok(None),
        }
    }

    /// List snippets matching the given query.
    pub async fn list_all_snippets(&self, query: &Query) -> Result<Vec<SnippetWithTags>> {
        let items = self.backend.query(query).await?;
        Ok(items
            .into_iter()
            .filter_map(|i| i.as_any().downcast_ref::<SnippetWithTags>().cloned())
            .collect())
    }

    /// Save a snippet to the backend.
    pub async fn save_snippet(&self, snippet: &Snippet) -> Result<()> {
        self.backend.save(snippet).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_placeholders() {
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "Alice".to_string());
        variables.insert("place".to_string(), "Wonderland".to_string());

        let content = "Hello {{name}}, welcome to {{place}}!";
        let expanded = expand_placeholders(content, &variables);
        assert_eq!(expanded, "Hello Alice, welcome to Wonderland!");

        let content_no_placeholders = "Hello world!";
        let expanded_no_change = expand_placeholders(content_no_placeholders, &variables);
        assert_eq!(expanded_no_change, "Hello world!");

        let content_missing_var = "Hello {{name}}, how is {{location}}?";
        let expanded_missing = expand_placeholders(content_missing_var, &variables);
        assert_eq!(expanded_missing, "Hello Alice, how is {{location}}?");
    }

    #[test]
    fn test_validate_snippet_content() {
        assert!(validate_snippet_content("Title", "Content").is_ok());

        assert!(validate_snippet_content("", "Content").is_err());
        assert!(validate_snippet_content(" \t\n", "Content").is_err());
        assert!(validate_snippet_content("Title", "").is_err());
        assert!(validate_snippet_content("Title", "  ").is_err());

        let long_title = "a".repeat(256);
        assert!(validate_snippet_content(&long_title, "Content").is_err());

        let long_content = "a".repeat(100_001);
        assert!(validate_snippet_content("Title", &long_content).is_err());
    }
}
