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
    
    // SQLite-specific tests
    #[cfg(feature = "sqlite")]
    mod sqlite_tests {
        use super::*;
        use uuid::Uuid;
        use crate::models::{NewDbSnippet, UpdateSnippet};
        use crate::schema::snippets::dsl as snippets_dsl;
        
        #[test]
        fn test_search_snippets() -> Result<()> {
            let pool = create_test_pool()?;
            let mut conn = pool.get()?;
            
            // Check if FTS5 is available
            let fts5_available = match diesel::sql_query(
                "SELECT 1 as available FROM pragma_compile_options WHERE compile_options = 'ENABLE_FTS5'"
            ).execute(&mut *conn) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("WARNING: Could not check FTS5 availability: {}", e);
                    false
                }
            };
            
            if !fts5_available {
                eprintln!("Skipping FTS5 test - FTS5 is not available in this SQLite build");
                return Ok(());
            }
            
            // Create FTS5 virtual table if it doesn't exist
            diesel::sql_query(
                "CREATE VIRTUAL TABLE IF NOT EXISTS snippets_fts USING fts5(title, content, content='snippets', content_rowid='id')"
            ).execute(&mut *conn)?;
            
            // Clear any existing test data
            diesel::sql_query("DELETE FROM snippets_fts").execute(&mut *conn)?;
            diesel::delete(snippets_dsl::snippets).execute(&mut *conn)?;
        
        // Helper function to create and save a snippet
        fn create_and_save_snippet(
            conn: &mut SqliteConnection,
            snippet_title: &str,
            snippet_content: &str,
            snippet_tags: Vec<&str>,
        ) -> Result<()> {
            use crate::models::NewDbSnippet;
            
            let new_snippet = NewDbSnippet {
                uuid: Uuid::new_v4().to_string(),
                title: snippet_title.to_string(),
                content: snippet_content.to_string(),
                tags: serde_json::to_string(&snippet_tags).unwrap(),
                embedding: None,
            };
            
            diesel::insert_into(crate::schema::snippets::table)
                .values(&new_snippet)
                .execute(conn)?;
                
            Ok(())
        }
        
        // Add test snippets with varied content
        create_and_save_snippet(
            &mut conn,
            "Rust Ownership",
            "Ownership is a set of rules that governs how Rust manages memory.",
            vec!["rust", "memory"],
        )?;
        
        create_and_save_snippet(
            &mut conn,
            "Rust Error Handling",
            "Rust groups errors into two major categories: recoverable and unrecoverable.",
            vec!["rust", "error"],
        )?;
        
        create_and_save_snippet(
            &mut conn,
            "Python Lists",
            "Lists are one of 4 built-in data types in Python used to store collections of data.",
            vec!["python", "data-structures"],
        )?;
        
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
    
    // PostgreSQL-specific tests
    #[cfg(feature = "postgres")]
    mod postgres_tests {
        use super::*;
        use crate::models::{NewDbSnippet, UpdateSnippet};
        use crate::schema::snippets::dsl as snippets_dsl;
        use uuid::Uuid;
        
        #[test]
        fn test_search_snippets() -> Result<()> {
            let pool = create_test_pool()?;
            let mut conn = pool.get()?;
            
            // Clear any existing test data
            diesel::delete(snippets_dsl::snippets).execute(&mut *conn)?;
            
            // TODO: Add PostgreSQL-specific full-text search tests here
            eprintln!("PostgreSQL full-text search tests not yet implemented");
            
            Ok(())
        }
    }
}