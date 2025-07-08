# Context7 Integration Implementation Guide

## Overview
This document outlines the complete implementation plan for integrating Context7 documentation import functionality into Rustash. The integration will allow users to import up-to-date documentation and code examples directly into their snippet library.

## Table of Contents
1. [Database Schema Updates](#1-database-schema-updates)
2. [Core Implementation](#2-core-implementation)
3. [CLI Commands](#3-cli-commands)
4. [Testing Strategy](#4-testing-strategy)
5. [Documentation](#5-documentation)
6. [Deployment](#6-deployment)

## 1. Database Schema Updates

### 1.1 Migration Script
Create a new migration to add source tracking fields:

```bash
diesel migration generate add_context7_fields
```

### 1.2 Migration Files
`migrations/YYYYMMDD-hhmmss_add_context7_fields/up.sql`:
```sql
-- Add source tracking columns
ALTER TABLE snippets ADD COLUMN source TEXT;
ALTER TABLE snippets ADD COLUMN source_id TEXT;

-- Add indexes for faster lookups
CREATE INDEX idx_snippets_source ON snippets(source);
CREATE INDEX idx_snippets_source_id ON snippets(source_id);
```

`migrations/YYYYMMDD-hhmmss_add_context7_fields/down.sql`:
```sql
-- Drop indexes first
DROP INDEX IF EXISTS idx_snippets_source;
DROP INDEX IF EXISTS idx_snippets_source_id;

-- Then drop columns
ALTER TABLE snippets DROP COLUMN source;
ALTER TABLE snippets DROP COLUMN source_id;
```

### 1.3 Run Migration
```bash
diesel migration run
```

## 2. Core Implementation

### 2.1 Add Dependencies
Add to `crates/rustash-core/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
indicatif = "0.17"  # For progress bars
```

### 2.2 Context7 Client
Create `crates/rustash-core/src/context7.rs`:

```rust
use anyhow::{Context as _, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

const BASE_URL: &str = "https://context7.upstash.io";
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: u64 = 1; // seconds

#[derive(Debug, Serialize)]
struct ResolveLibraryIdRequest {
    library_name: String,
}

#[derive(Debug, Deserialize)]
struct ResolveLibraryIdResponse {
    id: String,
}

#[derive(Debug, Serialize)]
struct GetLibraryDocsRequest {
    context7_compatible_library_id: String,
    topic: Option<String>,
    tokens: Option<u32>,
}

pub struct Context7Client {
    client: Client,
    base_url: String,
}

impl Context7Client {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: std::env::var("CONTEXT7_BASE_URL")
                .unwrap_or_else(|_| BASE_URL.to_string()),
        }
    }

    pub async fn resolve_library_id(&self, name: &str) -> Result<String> {
        // Implementation with retry logic
        let mut retries = 0;
        let mut last_error = None;

        while retries < MAX_RETRIES {
            let request = self
                .client
                .post(&format!("{}/resolve-library-id", self.base_url))
                .json(&ResolveLibraryIdRequest {
                    library_name: name.to_string(),
                });

            match request.send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        return Err(anyhow::anyhow!(
                            "API request failed with status {}: {}",
                            status,
                            text
                        ));
                    }

                    let response: ResolveLibraryIdResponse = response
                        .json()
                        .await
                        .context("Failed to parse resolve-library-id response")?;

                    return Ok(response.id);
                }
                Err(e) => {
                    last_error = Some(e);
                    retries += 1;
                    if retries < MAX_RETRIES {
                        sleep(Duration::from_secs(RETRY_DELAY * retries as u64)).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed after {} retries: {:?}",
            MAX_RETRIES,
            last_error
        ))
    }

    pub async fn get_library_docs(
        &self,
        library_id: &str,
        topic: Option<&str>,
        tokens: Option<u32>,
    ) -> Result<String> {
        // Implementation with retry logic
        let mut retries = 0;
        let mut last_error = None;

        while retries < MAX_RETRIES {
            let request = self
                .client
                .post(&format!("{}/get-library-docs", self.base_url))
                .json(&GetLibraryDocsRequest {
                    context7_compatible_library_id: library_id.to_string(),
                    topic: topic.map(|t| t.to_string()),
                    tokens,
                });

            match request.send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        return Err(anyhow::anyhow!(
                            "API request failed with status {}: {}",
                            status,
                            text
                        ));
                    }

                    return response
                        .text()
                        .await
                        .context("Failed to read response body");
                }
                Err(e) => {
                    last_error = Some(e);
                    retries += 1;
                    if retries < MAX_RETRIES {
                        sleep(Duration::from_secs(RETRY_DELAY * retries as u64)).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed after {} retries: {:?}",
            MAX_RETRIES,
            last_error
        ))
    }

    pub async fn list_available_libraries(&self) -> Result<HashMap<String, String>> {
        // Note: This is a placeholder - in a real implementation, we'd need an API endpoint
        let mut libraries = HashMap::new();
        libraries.insert("react".to_string(), "/facebook/react".to_string());
        libraries.insert("nextjs".to_string(), "/vercel/next.js".to_string());
        libraries.insert("typescript".to_string(), "/microsoft/typescript".to_string());
        Ok(libraries)
    }
}
```

### 2.3 Update Snippet Model
Update `crates/rustash-core/src/models.rs`:

```rust
// Add to the Snippet struct
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone, PartialEq, QueryableByName)]
#[diesel(table_name = crate::schema::snippets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Snippet {
    // ... existing fields ...
    
    #[diesel(sql_type = Text)]
    pub source: Option<String>,
    
    #[diesel(sql_type = Text)]
    pub source_id: Option<String>,
}

// Update NewSnippet
#[derive(Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = snippets)]
pub struct NewSnippet {
    // ... existing fields ...
    pub source: Option<String>,
    pub source_id: Option<String>,
}
```

### 2.4 Add Import Functions
Add to `crates/rustash-core/src/snippet.rs`:

```rust
use crate::context7::Context7Client;
use chrono::Utc;

pub async fn import_context7_docs(
    conn: &mut DbConnection,
    library_name: &str,
    topic: Option<&str>,
) -> Result<usize> {
    let client = Context7Client::new();
    let library_id = client.resolve_library_id(library_name).await?;
    
    let docs = client.get_library_docs(&library_id, topic, None).await?;
    
    let snippet = NewSnippet {
        uuid: Uuid::new_v4().to_string(),
        title: format!("{} Documentation", library_name),
        content: docs,
        tags: Some(serde_json::to_string(&vec![
            "context7".to_string(),
            format!("library:{}", library_name),
            topic.map(|t| format!("topic:{}", t)).unwrap_or_default(),
        ].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>())?),
        source: Some("context7".to_string()),
        source_id: Some(library_id),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };
    
    use crate::schema::snippets::dsl::*;
    diesel::insert_into(snippets)
        .values(&snippet)
        .execute(conn)?;
    
    Ok(1)
}

pub fn is_library_imported(
    conn: &mut DbConnection,
    library_id: &str,
) -> Result<bool> {
    use crate::schema::snippets::dsl::*;
    use diesel::select;
    use diesel::dsl::exists;
    
    let exists = select(exists(
        snippets.filter(source.eq("context7").and(source_id.eq(library_id)))
    ))
    .get_result(conn)?;
    
    Ok(exists)
}
```

## 3. CLI Commands

### 3.1 Add Dependencies
Add to `crates/rustash-cli/Cargo.toml`:

```toml
[dependencies]
indicatif = "0.17"
```

### 3.2 Create Import Commands
Create `crates/rustash-cli/src/commands/import.rs`:

```rust
use crate::prelude::*;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug, Subcommand)]
pub enum ImportCommand {
    /// Import documentation from Context7
    Context7(ImportContext7Command),
    
    /// List available libraries in Context7
    #[command(alias = "ls")]
    ListContext7Libraries,
}

#[derive(Debug, Parser)]
pub struct ImportContext7Command {
    /// Name of the library to import (e.g., "react", "nextjs")
    pub library: String,
    
    /// Optional topic to focus on (e.g., "hooks", "routing")
    #[arg(short, long)]
    pub topic: Option<String>,
    
    /// Force import even if already imported
    #[arg(short, long)]
    pub force: bool,
}

pub async fn handle_import(cmd: ImportCommand) -> Result<()> {
    match cmd {
        ImportCommand::Context7(cmd) => handle_import_context7(cmd).await,
        ImportCommand::ListContext7Libraries => handle_list_context7_libraries().await,
    }
}

async fn handle_import_context7(cmd: ImportContext7Command) -> Result<()> {
    use crate::context7::Context7Client;
    use crate::snippet::{import_context7_docs, is_library_imported};
    
    let mut conn = crate::db::establish_connection()?;
    let client = Context7Client::new();
    
    // Resolve library ID with progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner} {msg}")?,
    );
    pb.set_message(format!("Resolving library '{}'...", cmd.library));
    
    let library_id = match client.resolve_library_id(&cmd.library).await {
        Ok(id) => {
            pb.finish_with_message(format!("Resolved '{}' to ID: {}", cmd.library, id));
            id
        }
        Err(e) => {
            pb.finish_with_message("Failed to resolve library");
            return Err(e.into());
        }
    };
    
    // Check if already imported
    if !cmd.force && is_library_imported(&mut conn, &library_id)? {
        println!("Documentation for '{}' is already imported. Use --force to re-import.", cmd.library);
        return Ok(());
    }
    
    // Import with progress
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")?
            .progress_chars("#>-"),
    );
    
    println!("Importing documentation for '{}'...", cmd.library);
    if let Some(topic) = &cmd.topic {
        println!("Focusing on topic: {}", topic);
    }
    
    pb.set_message("Downloading documentation...");
    let count = import_context7_docs(&mut conn, &cmd.library, cmd.topic.as_deref()).await?;
    pb.finish_with_message(format!("Imported {} items", count));
    
    Ok(())
}

async fn handle_list_context7_libraries() -> Result<()> {
    use crate::context7::Context7Client;
    
    println!("Fetching available libraries from Context7...");
    let client = Context7Client::new();
    let libraries = client.list_available_libraries().await?;
    
    let mut libraries: Vec<_> = libraries.into_iter().collect();
    libraries.sort_by_key(|(name, _)| name.clone());
    
    println!("\nAvailable libraries:");
    for (name, id) in libraries {
        println!("  {:<20} {}", name, id);
    }
    
    Ok(())
}
```

### 3.3 Update Main CLI
Update `crates/rustash-cli/src/main.rs`:

```rust
mod commands;
use commands::import::{handle_import, ImportCommand};

#[derive(Debug, Parser)]
#[command(name = "rustash")]
#[command(about = "A modern snippet manager", long_about = None)]
enum Cli {
    // ... other variants ...
    
    /// Import documentation from external sources
    #[command(subcommand)]
    Import(ImportCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    // ... existing code ...
    
    match cli {
        // ... other matches ...
        Cli::Import(cmd) => handle_import(cmd).await?,
    }
    
    Ok(())
}
```

## 4. Testing Strategy

### 4.1 Unit Tests
Add to `crates/rustash-core/src/context7.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    #[tokio::test]
    async fn test_resolve_library_id_success() {
        let mut server = Server::new_async().await;
        let mock_response = json!({ "id": "/test/library" });
        
        let _m = server
            .mock("POST", "/resolve-library-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        let client = Context7Client::with_base_url(server.url());
        let result = client.resolve_library_id("test").await.unwrap();
        assert_eq!(result, "/test/library");
    }

    // Add more test cases...
}
```

### 4.2 Integration Tests
Create `tests/import_tests.rs`:

```rust
use rustash_core::context7::Context7Client;
use rustash_core::db::test_connection;
use rustash_core::snippet::import_context7_docs;

#[tokio::test]
async fn test_import_documentation() {
    let mut conn = test_connection().await;
    
    // Test importing documentation
    let result = import_context7_docs(&mut conn, "react", None).await;
    assert!(result.is_ok());
    
    // Verify the snippet was created
    use rustash_core::schema::snippets::dsl::*;
    let count: i64 = snippets.count().first(&mut conn).unwrap();
    assert!(count > 0);
}
```

## 5. Documentation

### 5.1 Update README.md
Add a new section:

```markdown
## Importing Documentation

Rustash can import documentation directly from Context7:

```bash
# List available libraries
rustash import list-context7

# Import React documentation
rustash import context7 react

# Import with specific topic
rustash import context7 nextjs --topic routing

# Force re-import
rustash import context7 typescript --force
```

### Environment Variables

- `CONTEXT7_BASE_URL`: Override the default Context7 API endpoint (default: `https://context7.upstash.io`)
- `RUST_LOG`: Set to `debug` for detailed logging
```

### 5.2 Update User Guide
Add a section to `USER_GUIDE.md`:

```markdown
## Importing Documentation

### From Context7

Context7 provides up-to-date documentation for popular libraries and frameworks. To import:

1. List available libraries:
   ```bash
   rustash import list-context7
   ```

2. Import documentation:
   ```bash
   rustash import context7 <library> [--topic TOPIC] [--force]
   ```

### Managing Imports

- Use `--force` to re-import documentation
- Documentation is tagged with `context7` and `library:<name>`
- Use topics to filter documentation (e.g., `--topic routing`)
```

## 6. Deployment

### 6.1 Version Bumping
Update version in `Cargo.toml`:

```toml
[package]
version = "0.2.0"  # Update as needed
```

### 6.2 Build and Test
```bash
# Run tests
cargo test

# Build release
cargo build --release
```

### 6.3 Release Notes
Update `CHANGELOG.md`:

```markdown
## [0.2.0] - YYYY-MM-DD

### Added
- Context7 documentation import
  - New `import` command
  - Support for listing available libraries
  - Topic-based filtering
  - Progress indicators for long-running imports
```

## Future Enhancements

1. **Batch Imports**: Import multiple libraries at once
2. **Update Mechanism**: Check for and apply updates to imported documentation
3. **Interactive Mode**: Interactive selection of libraries and topics
4. **Offline Mode**: Cache documentation for offline use
5. **Custom Sources**: Support for custom documentation sources

## Troubleshooting

### Common Issues

1. **API Rate Limiting**
   - Error: "Too many requests"
   - Solution: Wait before retrying or contact Context7 for higher limits

2. **Library Not Found**
   - Error: "Library not found"
   - Solution: Check available libraries with `rustash import list-context7`

3. **Network Issues**
   - Error: Connection timeout
   - Solution: Check your internet connection and proxy settings

### Getting Help

For additional help, open an issue on our [GitHub repository](https://github.com/yourorg/rustash/issues).

## License

This integration is available under the same license as Rustash.

---

*Note: This document was generated on 2025-07-07. For the latest updates, check the [official documentation](https://github.com/yourorg/rustash).*
