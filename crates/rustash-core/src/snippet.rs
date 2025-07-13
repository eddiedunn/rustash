//! Snippet helper functions

use std::collections::HashMap;

use crate::error::{Error, Result};

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
        return Err(Error::validation("Snippet title is too long (max 255 characters)"));
    }
    
    if snippet_content.len() > 100_000 {
        return Err(Error::validation("Snippet content is too long (max 100,000 characters)"));
    }
    
    Ok(())
}
