//! Snippet CRUD operations

use crate::database::DbConnection;
use crate::error::{Error, Result, UuidExt};
use crate::models::{NewDbSnippet, Snippet, SnippetListItem, SnippetWithTags, UpdateSnippet};
use diesel::prelude::*;
use std::collections::HashMap;

/// Add a new snippet to the database
pub fn add_snippet(conn: &mut DbConnection, new_snippet: Snippet) -> Result<Snippet> {
    // Validate input
    validate_snippet_content(&new_snippet.title, &new_snippet.content)?;
    
    use crate::schema::snippets::dsl::*;
    
    // Convert Snippet to NewDbSnippet for insertion
    let new_db_snippet = NewDbSnippet::from(new_snippet);
    
    // Insert the new snippet and get the result
    // Note: SQLite doesn't support RETURNING, so we need to fetch the inserted row separately
    let snippet_uuid = new_db_snippet.uuid.clone();
    
    diesel::insert_into(snippets)
        .values(&new_db_snippet)
        .execute(conn)?;
    
    // Fetch the newly inserted snippet
    let result = snippets
        .filter(uuid.eq(&snippet_uuid))
        .select(Snippet::as_select())
        .first(conn)
        .map_err(|e| {
            if let diesel::result::Error::NotFound = e {
                Error::not_found(format!("Failed to retrieve newly created snippet with UUID: {}", snippet_uuid))
            } else {
                e.into()
            }
        })?;
    
    Ok(result)
}

/// Get a snippet by UUID
pub fn get_snippet_by_id(conn: &mut DbConnection, snippet_uuid: &str) -> Result<Option<Snippet>> {
    // Validate UUID format
    snippet_uuid.parse_uuid()?;
    use crate::schema::snippets::dsl::*;
    
    let result = snippets
        .filter(uuid.eq(snippet_uuid))
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
        // For simplicity, we'll build a single query string with the search terms
        let mut search_terms = Vec::new();

        // Add text search terms
        if let Some(text) = filter_text.and_then(|t| if t.trim().is_empty() { None } else { Some(t) }) {
            search_terms.push(text.to_string());
        }

        // Add tag filter
        if let Some(tag) = tag_filter.and_then(|t| if t.trim().is_empty() { None } else { Some(t) }) {
            search_terms.push(format!("tags:{}", tag));
        }

        if !search_terms.is_empty() {
            // Join all search terms with AND
            let query_str = search_terms.join(" AND ");
            
            // Build and execute the FTS query
            let query = format!(
                "SELECT s.uuid, s.title, s.tags, s.updated_at 
                 FROM snippets s 
                 JOIN snippets_fts ON s.rowid = snippets_fts.rowid 
                 WHERE snippets_fts MATCH ?
                 ORDER BY bm25(snippets_fts, 2.0, 1.0, 0.5), s.updated_at DESC {}",
                limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default()
            );
            
            // Execute the query with the search terms as a single parameter
            let results = diesel::sql_query(&query)
                .bind::<diesel::sql_types::Text, _>(&query_str)
                .load::<SnippetListItem>(conn)
                .map_err(|e| Error::other(format!("Failed to execute FTS query: {}", e)))?;
                
            return Ok(results);
        }
    }

    // No filters provided, return all snippets ordered by last updated.
    use crate::schema::snippets::dsl::*;
    
    // Fallback to listing all snippets if no filters
    let results = if let Some(limit_val) = limit {
        snippets
            .select((uuid, title, tags, updated_at))
            .order(updated_at.desc())
            .limit(limit_val)
            .load::<SnippetListItem>(conn)?
    } else {
        snippets
            .select((uuid, title, tags, updated_at))
            .order(updated_at.desc())
            .load::<SnippetListItem>(conn)?
    };
    
    Ok(results)
}

/// Update an existing snippet
pub fn update_snippet(
    conn: &mut DbConnection,
    snippet_uuid: &str,
    update_data: UpdateSnippet,
) -> Result<Snippet> {
    use crate::schema::snippets::dsl::*;
    
    // Validate UUID format
    snippet_uuid.parse_uuid()?;
    
    // Validate input if title or content is being updated
    if let (Some(title_val), Some(content_val)) = (&update_data.title, &update_data.content) {
        validate_snippet_content(title_val, content_val)?;
    }
    
    // Update the snippet and get the number of affected rows
    let rows_updated = diesel::update(snippets.filter(uuid.eq(snippet_uuid)))
        .set(&update_data)
        .execute(conn)?;
    
    if rows_updated == 0 {
        return Err(Error::not_found(format!("Snippet with UUID: {}", snippet_uuid)));
    }
    
    // Fetch the updated snippet
    let result = snippets
        .filter(uuid.eq(snippet_uuid))
        .select(Snippet::as_select())
        .first(conn)
        .map_err(|e| {
            if let diesel::result::Error::NotFound = e {
                Error::other(format!("Failed to retrieve updated snippet with UUID: {}", snippet_uuid))
            } else {
                e.into()
            }
        })?;
    
    Ok(result)
}

/// Delete a snippet by UUID
pub fn delete_snippet(conn: &mut DbConnection, snippet_uuid: &str) -> Result<bool> {
    use crate::schema::snippets::dsl::*;
    
    // Validate UUID format
    snippet_uuid.parse_uuid()?;
    
    let num_deleted = diesel::delete(snippets.filter(uuid.eq(snippet_uuid)))
        .execute(conn)?;
    
    if num_deleted == 0 {
        return Err(Error::not_found(format!("Snippet with UUID: {}", snippet_uuid)));
    }
    
    Ok(true)
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
/// A vector of matching snippet list items (without content), ordered by relevance
pub fn search_snippets(
    conn: &mut DbConnection,
    query_text: &str,
    limit: Option<i64>,
) -> Result<Vec<SnippetListItem>> {
    // Delegate to list_snippets with the query as the filter text
    list_snippets(conn, Some(query_text), None, limit)
}

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

/// Get snippets with parsed tags and full content
/// 
/// This function first gets a list of snippet list items (without content) and then
/// fetches the full content for each snippet to include in the result.
/// 
/// # Arguments
/// * `conn` - Database connection
/// * `filter_text` - Optional text to search for in title or content
/// * `tag_filter` - Optional tag to filter by
/// * `limit` - Maximum number of results to return
/// 
/// # Returns
/// A vector of `SnippetWithTags` containing the full snippet data with parsed tags
pub fn list_snippets_with_tags(
    conn: &mut DbConnection,
    filter_text: Option<&str>,
    tag_filter: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<SnippetWithTags>> {
    use crate::schema::snippets::dsl::*;
    
    // First get the lightweight snippet list
    let snippet_items = list_snippets(conn, filter_text, tag_filter, limit)?;
    
    // If no snippets found, return early
    if snippet_items.is_empty() {
        return Ok(Vec::new());
    }
    
    // Extract UUIDs from the snippet items
    let snippet_uuids: Vec<&str> = snippet_items
        .iter()
        .map(|item| item.uuid.as_str())
        .collect();
    
    // Fetch the full snippets in a single query using the UUIDs
    let full_snippets = snippets
        .filter(uuid.eq_any(snippet_uuids))
        .select(Snippet::as_select())
        .load::<Snippet>(conn)?;
    
    // Convert Snippet to SnippetWithTags
    let mut results: Vec<SnippetWithTags> = full_snippets
        .into_iter()
        .map(SnippetWithTags::from)
        .collect();
    
    // Maintain the original order from list_snippets
    let uuid_to_index: std::collections::HashMap<_, _> = snippet_items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.uuid.as_str(), i))
        .collect();
    
    results.sort_by_cached_key(|s| {
        *uuid_to_index.get(s.uuid.as_str()).unwrap_or(&usize::MAX)
    });
    
    Ok(results)
}

/// Validate snippet content
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::create_test_pool;
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
        let pool = create_test_pool()?;
        let mut conn = pool.get()?;
        
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
        let pool = create_test_pool()?;
        let mut conn = pool.get()?;
        
        // Add test snippets
        let snippet1 = NewDbSnippet::new(
            "Rust Code Snippet".to_string(),
            "fn main() {}".to_string(),
            vec!["rust".to_string(), "code".to_string()],
        );
        let snippet2 = NewDbSnippet::new(
            "Python Code Example".to_string(),
            "print('hello')".to_string(),
            vec!["python".to_string(), "code".to_string()],
        );
        let snippet3 = NewDbSnippet::new(
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
        
        let pool = create_test_pool()?;
        let mut conn = pool.get()?;
        
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