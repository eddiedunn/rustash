//! Snippet CRUD operations

use crate::database::DbConnection;
use crate::error::{Error, Result};
use crate::models::{NewSnippet, Snippet, SnippetListItem, SnippetWithTags, UpdateSnippet};
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

/// List all snippets with optional filtering using FTS5 for text and tag search.
///
/// This function leverages SQLite's FTS5 virtual table for efficient full-text search.
/// When filters are provided, it constructs an FTS5 query to search across title,
/// content, and tags.
///
/// # Arguments
/// * `conn` - Database connection
/// * `filter_text` - Optional text to search for in title or content.
/// * `tag_filter` - Optional tag to filter by.
/// * `limit` - Maximum number of results to return.
///
/// # Returns
/// A vector of matching snippets, ordered by relevance (if searching) or update time.
pub fn list_snippets(
    conn: &mut DbConnection,
    filter_text: Option<&str>,
    tag_filter: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<SnippetListItem>> {
    // If there are any filters, build and execute an FTS query.
    if filter_text.is_some() || tag_filter.is_some() {
        let mut query_parts = Vec::new();

        if let Some(text) = filter_text.and_then(|t| if t.trim().is_empty() { None } else { Some(t) }) {
            // Tokenize the search text and join with AND for more intuitive search
            let fts_terms = text.split_whitespace()
                .map(|term| term.replace('"', "\"\""))
                .collect::<Vec<_>>()
                .join(" AND ");
            
            if !fts_terms.is_empty() {
                query_parts.push(format!("snippets_fts = '{}'", fts_terms));
            }
        }

        if let Some(tag) = tag_filter.and_then(|t| if t.trim().is_empty() { None } else { Some(t) }) {
            let escaped_tag = tag.replace('"', "\"\"");
            query_parts.push(format!("tags:\"{}\"", escaped_tag));
        }

        if !query_parts.is_empty() {
            let fts_query = query_parts.join(" AND ");
            
            // Build and execute the FTS query directly, selecting only the fields we need
            let query = format!(
                "SELECT s.id, s.title, s.tags, s.updated_at FROM snippets s 
                 JOIN snippets_fts ON s.id = snippets_fts.rowid 
                 WHERE snippets_fts MATCH ? 
                 ORDER BY bm25(snippets_fts, 2.0, 1.0, 0.5), s.updated_at DESC {}",
                limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default()
            );
            
            let results: Vec<SnippetListItem> = diesel::sql_query(query)
                .bind::<diesel::sql_types::Text, _>(&fts_query)
                .load(conn)
                .map_err(|e| Error::other(format!("Failed to execute FTS query: {}", e)))?;
                
            return Ok(results);
        }
    }

    // No filters provided, return all snippets ordered by last updated.
    use crate::schema::snippets::dsl::*;
    let mut query = snippets.into_boxed().order(updated_at.desc());

    if let Some(limit_val) = limit {
        query = query.limit(limit_val);
    }
    
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
/// This is a convenience wrapper around `list_snippets` that performs a full-text search
/// across all searchable fields (title, content, and tags). For more complex queries,
/// consider using `list_snippets` directly with a custom FTS5 query.
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
    // Delegate to list_snippets with the query as the filter text
    list_snippets(conn, Some(query_text), None, limit)
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
    // First get the lightweight snippet list
    let snippets = list_snippets(conn, filter_text, tag_filter, limit)?;
    
    // Only fetch full content for snippets that need it
    let snippets_with_tags: Result<Vec<SnippetWithTags>> = if snippets.is_empty() {
        Ok(Vec::new())
    } else {
        use crate::schema::snippets::dsl::*;
        
        // Get the IDs of the snippets we need to fetch
        let snippet_ids: Vec<i32> = snippets
            .iter()
            .filter_map(|s| s.id)
            .collect();
        
        // Fetch only the full snippets we need in a single query
        let full_snippets = snippets
            .filter(id.eq_any(snippet_ids))
            .select(Snippet::as_select())
            .load::<Snippet>(conn)?;
        
        // Convert to SnippetWithTags
        Ok(full_snippets.into_iter().map(Into::into).collect())
    };
    
    snippets_with_tags
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
            "Rust Code Snippet".to_string(),
            "fn main() {}".to_string(),
            vec!["rust".to_string(), "code".to_string()],
        );
        let snippet2 = NewSnippet::new(
            "Python Code Example".to_string(),
            "print('hello')".to_string(),
            vec!["python".to_string(), "code".to_string()],
        );
        let snippet3 = NewSnippet::new(
            "Another Rust Item".to_string(),
            "struct a;".to_string(),
            vec!["rust".to_string(), "structs".to_string()],
        );
        
        add_snippet(&mut conn, snippet1)?;
        add_snippet(&mut conn, snippet2)?;
        add_snippet(&mut conn, snippet3)?;

        // List all snippets (should be 3)
        let all_snippets = list_snippets(&mut conn, None, None, None)?;
        assert_eq!(all_snippets.len(), 3);
        
        // Filter by title text (FTS)
        let rust_snippets = list_snippets(&mut conn, Some("Snippet"), None, None)?;
        assert_eq!(rust_snippets.len(), 1);
        assert_eq!(rust_snippets[0].title, "Rust Code Snippet");
        
        // Filter by tag (FTS)
        let python_snippets = list_snippets(&mut conn, None, Some("python"), None)?;
        assert_eq!(python_snippets.len(), 1);
        assert_eq!(python_snippets[0].title, "Python Code Example");
        
        // Filter by a different tag (FTS)
        let struct_snippets = list_snippets(&mut conn, None, Some("structs"), None)?;
        assert_eq!(struct_snippets.len(), 1);
        assert_eq!(struct_snippets[0].title, "Another Rust Item");

        // Combined filter: text AND tag (FTS)
        let combined_snippets = list_snippets(&mut conn, Some("Rust"), Some("code"), None)?;
        assert_eq!(combined_snippets.len(), 1);
        assert_eq!(combined_snippets[0].title, "Rust Code Snippet");

        // Combined filter with no results
        let no_results = list_snippets(&mut conn, Some("Python"), Some("rust"), None)?;
        assert_eq!(no_results.len(), 0);

        // Test with limit
        let limited = list_snippets(&mut conn, None, None, Some(1))?;
        assert_eq!(limited.len(), 1);
        
        Ok(())
    }
    
    #[test]
    fn test_search_snippets_fts5() -> Result<()> {
        use crate::models::NewSnippet;
        use crate::schema::snippets::dsl as snippets_dsl;
        
        let mut conn = establish_test_connection()?;
        
        // Debug: Check if FTS5 is available
        #[derive(QueryableByName)]
        struct FtsCheck {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            available: i32,
        }
        
        let fts5_available = match diesel::sql_query("SELECT 1 as available FROM pragma_compile_options WHERE compile_options = 'ENABLE_FTS5'")
            .get_results::<FtsCheck>(&mut conn) {
                Ok(rows) => !rows.is_empty(),
                Err(e) => {
                    eprintln!("WARNING: Could not check FTS5 availability: {}", e);
                    false
                }
            };
        
        if !fts5_available {
            eprintln!("WARNING: FTS5 is not available in this SQLite build");
        }
        
        // Debug: List all tables
        #[derive(QueryableByName)]
        struct TableName {
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
        }
        
        let tables = match diesel::sql_query("SELECT name FROM sqlite_master WHERE type='table'")
            .get_results::<TableName>(&mut conn) {
                Ok(rows) => rows.into_iter().map(|r| r.name).collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("WARNING: Could not list tables: {}", e);
                    vec![]
                }
            };
            
        eprintln!("Available tables: {:?}", tables);
        
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
                "Rust Error Handling".to_owned(),
                "Rust groups errors into two major categories: recoverable and unrecoverable.".to_owned(),
                vec!["rust".to_owned(), "error".to_owned()],
            ),
            (
                "Python Lists".to_owned(),
                "Lists are one of 4 built-in data types in Python used to store collections of data.".to_owned(),
                vec!["python".to_owned(), "data-structures".to_owned()],
            ),
        ];
        
        for (title, content, tags) in &test_data {
            let new_snippet = NewSnippet::new(
                title.clone(),
                content.clone(),
                tags.clone(),
            );
            add_snippet(&mut conn, new_snippet)?;
        }
        
        // Test search by title
        let rust_results = search_snippets(&mut conn, "Rust", None)?;
        assert_eq!(rust_results.len(), 2);
        
        // Test search by content
        let memory_results = search_snippets(&mut conn, "memory", None)?;
        assert_eq!(memory_results.len(), 1);
        assert_eq!(memory_results[0].title, "Rust Ownership");
        
        // Test search with tag - note: search_snippets only searches title and content
        // For tag search, use list_snippets with tag_filter
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