//! Snippet CRUD operations

use crate::database::DbConnection;
use crate::error::{Error, Result};
use crate::models::{NewSnippet, Snippet, SnippetWithTags, UpdateSnippet};
use diesel::prelude::*;
use std::collections::HashMap;

/// Add a new snippet to the database
pub fn add_snippet(conn: &mut DbConnection, new_snippet: NewSnippet) -> Result<Snippet> {
    // Validate input
    validate_snippet_content(&new_snippet.title, &new_snippet.content)?;
    
    use crate::schema::snippets::dsl::*;
    
    // SQLite doesn't support RETURNING, so we insert and then fetch
    diesel::insert_into(snippets)
        .values(&new_snippet)
        .execute(conn)?;
    
    // Get the last inserted row
    let result = snippets
        .order(id.desc())
        .select(Snippet::as_select())
        .first(conn)?;
    
    Ok(result)
}

/// Get a snippet by ID
pub fn get_snippet_by_id(conn: &mut DbConnection, snippet_id: i32) -> Result<Option<Snippet>> {
    use crate::schema::snippets::dsl::*;
    
    let result = snippets
        .filter(id.eq(snippet_id))
        .select(Snippet::as_select())
        .first(conn)
        .optional()?;
    
    Ok(result)
}

/// List all snippets with optional filtering
pub fn list_snippets(
    conn: &mut DbConnection,
    filter_text: Option<&str>,
    tag_filter: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<Snippet>> {
    use crate::schema::snippets::dsl::*;
    
    let mut query = snippets.into_boxed();
    
    // Apply text filter if provided
    if let Some(filter) = filter_text {
        let pattern = format!("%{filter}%");
        query = query.filter(
            title.like(pattern.clone())
                .or(content.like(pattern))
        );
    }
    
    // Apply tag filter if provided
    if let Some(tag) = tag_filter {
        let tag_pattern = format!("%\"{tag}%");
        query = query.filter(tags.like(tag_pattern));
    }
    
    // Apply limit if provided
    if let Some(limit_val) = limit {
        query = query.limit(limit_val);
    }
    
    // Order by most recently updated
    query = query.order(updated_at.desc());
    
    let results = query.select(Snippet::as_select()).load(conn)?;
    
    Ok(results)
}

/// Update an existing snippet
pub fn update_snippet(
    conn: &mut DbConnection,
    snippet_id: i32,
    update_data: UpdateSnippet,
) -> Result<Snippet> {
    use crate::schema::snippets::dsl::*;
    
    // Validate input if title or content is being updated
    if let (Some(title_val), Some(content_val)) = (&update_data.title, &update_data.content) {
        validate_snippet_content(title_val, content_val)?;
    }
    
    // SQLite doesn't support RETURNING, so we update and then fetch
    diesel::update(snippets.filter(id.eq(snippet_id)))
        .set(&update_data)
        .execute(conn)?;
    
    // Get the updated row
    let result = snippets
        .filter(id.eq(snippet_id))
        .select(Snippet::as_select())
        .first(conn)?;
    
    Ok(result)
}

/// Delete a snippet by ID
pub fn delete_snippet(conn: &mut DbConnection, snippet_id: i32) -> Result<bool> {
    use crate::schema::snippets::dsl::*;
    
    let affected_rows = diesel::delete(snippets.filter(id.eq(snippet_id)))
        .execute(conn)?;
    
    Ok(affected_rows > 0)
}

/// Search snippets using full-text search with FTS5
/// 
/// This function uses SQLite's FTS5 virtual table for efficient full-text search.
/// The search is performed across the title, content, and tags fields.
/// 
/// # Arguments
/// * `conn` - Database connection
/// * `query_text` - Search query text (supports FTS5 syntax)
/// * `limit` - Maximum number of results to return
/// 
/// # Returns
/// A vector of matching snippets, ordered by relevance
pub fn search_snippets(
    conn: &mut DbConnection,
    query_text: &str,
    limit: Option<i64>,
) -> Result<Vec<Snippet>> {
    use crate::schema::snippets::dsl::*;
    use diesel::prelude::*;
    
    if query_text.trim().is_empty() {
        return list_snippets(conn, None, None, limit);
    }
    
    // Build the FTS5 query
    // The ' OR ' in the query allows searching across all FTS columns (title, content, tags)
    let fts_query = format!("{} OR {} OR {}", 
        query_text, // Title (boosted by position in the query)
        query_text, // Content
        query_text  // Tags
    );
    
    // Create a raw SQL query with proper parameter binding
    // We need to use the raw SQL interface since we're using FTS5-specific features
    let query = format!(
        "SELECT snippets.* FROM snippets 
        JOIN snippets_fts ON snippets.id = snippets_fts.rowid 
        WHERE snippets_fts MATCH ? 
        ORDER BY bm25(snippets_fts) DESC, snippets.updated_at DESC {}",
        limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default()
    );
    
    // Execute the query with proper error handling
    let results: Vec<Snippet> = diesel::sql_query(query)
        .bind::<diesel::sql_types::Text, _>(&fts_query)
        .load(conn)
        .map_err(|e| Error::other(format!("Failed to execute search query: {}", e)))?;
    
    Ok(results)
}

/// Expand placeholders in snippet content
pub fn expand_placeholders(content: &str, variables: &HashMap<String, String>) -> String {
    let mut result = content.to_string();
    
    for (key, value) in variables {
        let placeholder = format!("{{{{{key}}}}}");
        result = result.replace(&placeholder, value);
    }
    
    result
}

/// Get snippets with parsed tags
pub fn list_snippets_with_tags(
    conn: &mut DbConnection,
    filter_text: Option<&str>,
    tag_filter: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<SnippetWithTags>> {
    let snippets = list_snippets(conn, filter_text, tag_filter, limit)?;
    let with_tags = snippets.into_iter().map(SnippetWithTags::from).collect();
    Ok(with_tags)
}

/// Validate snippet content
fn validate_snippet_content(title: &str, content: &str) -> Result<()> {
    if title.trim().is_empty() {
        return Err(Error::validation("Title cannot be empty"));
    }
    
    if content.trim().is_empty() {
        return Err(Error::validation("Content cannot be empty"));
    }
    
    if title.len() > 255 {
        return Err(Error::validation("Title cannot exceed 255 characters"));
    }
    
    if content.len() > 100_000 {
        return Err(Error::validation("Content cannot exceed 100,000 characters"));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::establish_test_connection;
    use std::collections::HashMap;
    
    #[test]
    fn test_expand_placeholders() {
        let content = "Hello {{name}}, welcome to {{place}}!";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("place".to_string(), "Rustland".to_string());
        
        let result = expand_placeholders(content, &vars);
        assert_eq!(result, "Hello Alice, welcome to Rustland!");
    }
    
    #[test]
    fn test_expand_placeholders_missing_var() {
        let content = "Hello {{name}}, missing {{unknown}}";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        
        let result = expand_placeholders(content, &vars);
        assert_eq!(result, "Hello Alice, missing {{unknown}}");
    }
    
    #[test]
    fn test_validate_snippet_content() {
        // Valid content
        assert!(validate_snippet_content("Test", "Content").is_ok());
        
        // Empty title
        assert!(validate_snippet_content("", "Content").is_err());
        
        // Empty content
        assert!(validate_snippet_content("Test", "").is_err());
        
        // Title too long
        let long_title = "a".repeat(256);
        assert!(validate_snippet_content(&long_title, "Content").is_err());
    }
    
    #[test]
    fn test_add_and_get_snippet() -> Result<()> {
        let mut conn = establish_test_connection()?;
        
        let new_snippet = NewSnippet::new(
            "Test Snippet".to_string(),
            "Hello {{name}}!".to_string(),
            vec!["test".to_string(), "greeting".to_string()],
        );
        
        let snippet = add_snippet(&mut conn, new_snippet)?;
        assert!(snippet.id.is_some());
        assert_eq!(snippet.title, "Test Snippet");
        assert_eq!(snippet.content, "Hello {{name}}!");
        
        let retrieved = get_snippet_by_id(&mut conn, snippet.id.unwrap())?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Snippet");
        
        Ok(())
    }
    
    #[test]
    fn test_list_and_filter_snippets() -> Result<()> {
        let mut conn = establish_test_connection()?;
        
        // Add test snippets
        let snippet1 = NewSnippet::new(
            "Rust Code".to_string(),
            "fn main() {}".to_string(),
            vec!["rust".to_string(), "code".to_string()],
        );
        let snippet2 = NewSnippet::new(
            "Python Code".to_string(),
            "print('hello')".to_string(),
            vec!["python".to_string(), "code".to_string()],
        );
        
        add_snippet(&mut conn, snippet1)?;
        add_snippet(&mut conn, snippet2)?;
        
        // List all snippets
        let all_snippets = list_snippets(&mut conn, None, None, None)?;
        assert_eq!(all_snippets.len(), 2);
        
        // Filter by title
        let rust_snippets = list_snippets(&mut conn, Some("Rust"), None, None)?;
        assert_eq!(rust_snippets.len(), 1);
        assert_eq!(rust_snippets[0].title, "Rust Code");
        
        // Filter by tag
        let python_snippets = list_snippets(&mut conn, None, Some("python"), None)?;
        assert_eq!(python_snippets.len(), 1);
        assert_eq!(python_snippets[0].title, "Python Code");
        
        Ok(())
    }
    
    #[test]
    fn test_search_snippets_fts5() -> Result<()> {
        use crate::models::NewSnippet;
        use crate::schema::snippets::dsl as snippets_dsl;
        
        let mut conn = establish_test_connection()?;
        
        // Clear any existing test data
        diesel::delete(snippets_dsl::snippets).execute(&mut conn)?;
        
        // Add test snippets with varied content
        let test_data = [
            (
                "Rust Ownership".to_owned(),
                "Ownership is a set of rules that governs how Rust manages memory.".to_owned(),
                vec!["rust".to_owned(), "memory".to_owned()],
            ),
            (
                "Python List Comprehension".to_owned(),
                "List comprehensions provide a concise way to create lists.".to_owned(),
                vec!["python".to_owned(), "lists".to_owned()],
            ),
            (
                "Rust Error Handling".to_owned(),
                "Rust groups errors into two major categories: recoverable and unrecoverable.".to_owned(),
                vec!["rust".to_owned(), "error".to_owned()],
            ),
        ];
        
        for (snippet_title, snippet_content, snippet_tags) in test_data.into_iter() {
            let new_snippet = NewSnippet::new(snippet_title, snippet_content, snippet_tags);
            add_snippet(&mut conn, new_snippet)?;
        }
        
        // Test basic search
        let rust_results = search_snippets(&mut conn, "Rust", None)?;
        assert_eq!(rust_results.len(), 2);
        assert!(rust_results.iter().any(|s| s.title.contains("Rust")));
        
        // Test search with phrase
        let ownership_results = search_snippets(&mut conn, "\"Ownership is a set of rules\"", None)?;
        assert_eq!(ownership_results.len(), 1);
        assert_eq!(ownership_results[0].title, "Rust Ownership");
        
        // Test search with multiple terms (AND by default in FTS5)
        let rust_memory_results = search_snippets(&mut conn, "Rust memory", None)?;
        assert_eq!(rust_memory_results.len(), 1);
        assert_eq!(rust_memory_results[0].title, "Rust Ownership");
        
        // Test search with tag
        let error_results = search_snippets(&mut conn, "error", None)?;
        assert_eq!(error_results.len(), 1);
        assert_eq!(error_results[0].title, "Rust Error Handling");
        
        // Test limit parameter
        let limited_results = search_snippets(&mut conn, "Rust", Some(1))?;
        assert_eq!(limited_results.len(), 1);
        
        // Test empty search returns all snippets
        let empty_search = search_snippets(&mut conn, "", None)?;
        assert_eq!(empty_search.len(), 3);
        
        Ok(())
    }
}