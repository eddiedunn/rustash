name: "Base PRP Template v2 - Context-Rich with Validation Loops"
description: |

## Purpose

Template optimized for AI agents to implement features with sufficient context and self-validation capabilities to achieve working code through iterative refinement.

## Core Principles

1. **Context is King**: Include ALL necessary documentation, examples, and caveats
2. **Validation Loops**: Provide executable tests/lints the AI can run and fix
3. **Information Dense**: Use keywords and patterns from the codebase
4. **Progressive Success**: Start simple, validate, then enhance

---

## Goal

Build **Rustash 1.0** – a Rust‑based snippet manager (CLI + optional Tauri desktop) that persists data with **Diesel ORM**.  
* Default backend: **SQLite** (single‑file, no server)  
* Alternate backend (via Cargo feature flag): **PostgreSQL** for multi‑machine or server deployment  
* Optional vector search via **`sqlite-vss`** today and `pgvector` later  
* Schema, code, and migrations stay identical across both databases to maximise portability.

## Why

- **Developer productivity & local reliability**: fast, offline access to prompts/snippets boosts workflow speed on Apple‑silicon Macs.
- **Unifies and modernises existing shell scripts**: replaces disparate Bash + MySQL/PHP pieces with one Rust codebase and typed migrations.
- **Solves snippet recall for individual developers now, scales to shared memory for teams later** – the same codebase can move to Postgres or even Apache AGE for graph queries without redesign.

## What

* CLI commands: `rustash add`, `rustash list`, `rustash use` with placeholder expansion and clipboard copy.
* Desktop option: minimal Tauri window calling the same core library.
* Diesel ORM with compile‑time schema checks; migrations managed via `diesel migration`.
* Feature‑gated build: `--features sqlite` (default) or `--features postgres`.
* Vector column (`embedding`) using `sqlite-vss`; future support for `pgvector`.
* Clear path to graph queries via Apache AGE when Postgres backend is enabled.

### Success Criteria

- [ ] `cargo build --no-default-features --features sqlite` and `--features postgres` both compile and run tests.
- [ ] Diesel migrations apply cleanly on SQLite and Postgres.
- [ ] `rustash list` opens fuzzy finder and returns results within 150 ms on 1 000 snippets.
- [ ] `rustash use <id>` expands placeholders and puts text on clipboard (verified in unit test).
- [ ] Vector similarity search returns top‑5 nearest embeddings with `sqlite-vss` example query.

## All Needed Context

### Documentation & References (list all context needed to implement the feature)

```yaml
# MUST READ - Include these in your context window
- url: https://diesel.rs/guides/getting-started/
  why: Setup for SQLite & Postgres backends, migration commands, compile‑time schema generation

- file: rustash-core/src/models.rs
  why: Canonical Diesel entity definitions and derives; follow field naming & timestamp helpers

- doc: https://github.com/asg017/sqlite-vss
  section: README → “Usage” and “Installing the extension”
  critical: Shows how to create a VECTOR column and perform nearest‑neighbor queries in SQLite

- docfile: docs/apache_age_future.md
  why: Notes on enabling Apache AGE extension when migrating to Postgres for graph capabilities
````

### Current Codebase tree (run `tree` in the root of the project) to get an overview of the codebase

```bash
/Users/tmwsiy/code/rustash
├── .claude/              # Claude Code configuration prompts
├── .venv/               # Python virtual environment
├── Cargo.toml           # Rust workspace configuration
├── claude_md_files/
│   └── CLAUDE-RUST.md   # Comprehensive Rust development guidelines
├── CLAUDE.md            # Project-specific Claude instructions
├── images/
│   └── logo.png
├── PRPs/                # Product Requirement Prompts directory
│   ├── ai_docs/         # Claude Code documentation
│   ├── scripts/
│   │   └── prp_runner.py
│   ├── templates/
│   │   └── prp_rustash.md
│   └── README.md
├── pyproject.toml       # Python project configuration for PRP tooling
└── uv.lock             # Python dependency lockfile
```

### Desired Codebase tree with files to be added and responsibility of file

```bash
/Users/tmwsiy/code/rustash
├── Cargo.toml                    # Workspace root with feature flags
├── crates/
│   ├── rustash-core/            # Core library crate
│   │   ├── Cargo.toml           # Core dependencies (diesel, serde)
│   │   ├── src/
│   │   │   ├── lib.rs           # Public API exports
│   │   │   ├── models.rs        # Diesel ORM models
│   │   │   ├── schema.rs        # Generated Diesel schema
│   │   │   ├── database.rs      # Database connection management
│   │   │   ├── snippet.rs       # Snippet CRUD operations
│   │   │   └── search.rs        # Vector search functionality
│   │   ├── migrations/          # Diesel database migrations
│   │   │   └── 2024-01-01-000000_create_snippets/
│   │   │       ├── up.sql       # Create tables
│   │   │       └── down.sql     # Drop tables
│   │   └── tests/
│   │       └── integration.rs   # Database integration tests
│   ├── rustash-cli/             # CLI application crate
│   │   ├── Cargo.toml           # CLI dependencies (clap, clipboard)
│   │   ├── src/
│   │   │   ├── main.rs          # CLI entry point
│   │   │   ├── commands/
│   │   │   │   ├── add.rs       # Add snippet command
│   │   │   │   ├── list.rs      # List/search snippets
│   │   │   │   └── use.rs       # Use snippet with expansion
│   │   │   └── fuzzy.rs         # Fuzzy finder integration
│   │   └── tests/
│   │       └── cli.rs           # CLI command tests
│   └── rustash-desktop/         # Optional Tauri desktop app
│       ├── Cargo.toml           # Tauri dependencies
│       ├── src/
│       │   └── main.rs          # Tauri app entry
│       ├── src-tauri/
│       │   └── tauri.conf.json  # Tauri configuration
│       └── dist/                # Frontend build output
├── diesel.toml                   # Diesel CLI configuration
├── .env                          # Environment variables
└── README.md                     # Project documentation
```

### Known Gotchas of our codebase & Library Quirks

```rust
// CRITICAL: Diesel requires exactly ONE backend feature ("sqlite" OR "postgres") at compile time; builds fail if both are enabled or neither.
// GOTCHA: sqlite-vss extension must be loaded at runtime with sqlite3_load_extension
// GOTCHA: Diesel schema.rs is auto-generated; never edit manually - use migrations instead
// GOTCHA: Vector columns in SQLite need special handling vs PostgreSQL pgvector
// GOTCHA: Clipboard access requires different crates on different platforms (clipboard-win, x11-clipboard)
// GOTCHA: Fuzzy finder (fzf) integration requires spawning external process
// GOTCHA: Tauri async commands must use #[tauri::command] attribute and return Result<T, String>
```

## Implementation Blueprint

### Data models and structure

Create the core data models, we ensure type safety and consistency.

```rust
// Core Diesel ORM models
#[derive(Queryable, Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = snippets)]
pub struct Snippet {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,        // JSON array in database
    pub embedding: Option<Vec<f32>>, // Vector for similarity search
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = snippets)]
pub struct NewSnippet {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

// CLI command structs
#[derive(Parser)]
pub struct AddCommand {
    pub title: String,
    pub content: String,
    #[clap(short, long)]
    pub tags: Vec<String>,
}
```

### list of tasks to be completed to fullfill the PRP in the order they should be completed

```yaml
Task 1: Project Setup
  CREATE Cargo.toml:
    - SETUP workspace with crates: rustash-core, rustash-cli, rustash-desktop
    - DEFINE feature flags: default = ["sqlite"], sqlite = ["diesel/sqlite"], postgres = ["diesel/postgres"]
    - CONFIGURE workspace dependencies

Task 2: Core Library Foundation
  CREATE crates/rustash-core/:
    - SETUP Cargo.toml with diesel, serde, chrono dependencies
    - CREATE src/lib.rs with public API exports
    - SETUP diesel.toml configuration

Task 3: Database Schema & Models
  CREATE database migration:
    - RUN diesel setup
    - CREATE migration: diesel migration generate create_snippets
    - DEFINE schema in up.sql with vector column support
    - IMPLEMENT models.rs with Diesel derives

Task 4: Core CRUD Operations
  CREATE crates/rustash-core/src/snippet.rs:
    - IMPLEMENT add_snippet() function
    - IMPLEMENT list_snippets() with filtering
    - IMPLEMENT get_snippet_by_id()
    - IMPLEMENT update_snippet() and delete_snippet()

Task 5: CLI Application
  CREATE crates/rustash-cli/:
    - SETUP Cargo.toml with clap, clipboard dependencies
    - CREATE src/main.rs with command parsing
    - IMPLEMENT commands/add.rs, commands/list.rs, commands/use.rs
    - INTEGRATE fuzzy finder (fzf) in fuzzy.rs

Task 6: Vector Search (Optional)
  MODIFY crates/rustash-core/src/search.rs:
    - IMPLEMENT sqlite-vss integration
    - CREATE embedding generation (mock for now)
    - IMPLEMENT similarity search functions

Task 7: Tauri Desktop App (Optional)
  CREATE crates/rustash-desktop/:
    - SETUP Tauri project structure
    - CREATE Tauri commands calling rustash-core
    - BUILD minimal frontend interface

Task 8: Testing & Validation
  CREATE comprehensive tests:
    - UNIT tests for each module
    - INTEGRATION tests with real database
    - CLI command tests
    - FEATURE flag compilation tests
```

### Per task pseudocode as needed added to each task

```rust
// Task 4: Core CRUD Operations
// Pseudocode with CRITICAL details dont write entire code
pub fn add_snippet(conn: &mut SqliteConnection, new_snippet: NewSnippet) -> Result<Snippet, Error> {
    // PATTERN: Always validate input first
    validate_snippet_content(&new_snippet.content)?;
    
    // GOTCHA: Diesel requires explicit connection type for feature-gated backends
    use crate::schema::snippets::dsl::*;
    
    // PATTERN: Use Diesel insert with returning clause
    let result = diesel::insert_into(snippets)
        .values(&new_snippet)
        .returning(Snippet::as_returning())  // CRITICAL: Use as_returning() for type safety
        .get_result(conn)?;
    
    // PATTERN: Generate embedding after insert if vector search enabled
    #[cfg(feature = "vector-search")]
    generate_and_store_embedding(conn, result.id, &result.content)?;
    
    Ok(result)
}

// Task 5: CLI fuzzy finder integration
pub fn fuzzy_select_snippet(snippets: Vec<Snippet>) -> Result<Option<Snippet>, Error> {
    // GOTCHA: fzf requires spawning external process with stdin/stdout
    let mut child = Command::new("fzf")
        .arg("--height=40%")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    
    // PATTERN: Format snippet list for fzf display
    let input = snippets.iter()
        .map(|s| format!("{}: {}", s.id, s.title))
        .collect::<Vec<_>>()
        .join("\n");
    
    // CRITICAL: Handle broken pipe if user cancels fzf
    if let Some(stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes()); // Ignore broken pipe
    }
    
    let output = child.wait_with_output()?;
    // Parse selected ID from fzf output...
}
```

### Integration Points

```yaml
DATABASE:
  - migration: "CREATE TABLE snippets (id INTEGER PRIMARY KEY, title TEXT NOT NULL, content TEXT NOT NULL, tags TEXT, embedding BLOB, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)"
  - index: "CREATE INDEX idx_snippets_title ON snippets(title)"
  - vector: "CREATE VIRTUAL TABLE snippets_vector USING vss0(embedding(768))"

CONFIG:
  - add to: .env
  - pattern: "DATABASE_URL=sqlite:///rustash.db" or "DATABASE_URL=postgres://user:pass@localhost/rustash"
  - pattern: "EMBEDDING_MODEL=text-embedding-3-small"

CLI:
  - add to: src/main.rs
  - pattern: "#[derive(Parser)] struct Cli { #[command(subcommand)] command: Commands }"
  - pattern: "enum Commands { Add(AddCommand), List(ListCommand), Use(UseCommand) }"

TAURI:
  - add to: src-tauri/tauri.conf.json
  - pattern: "allowlist": { "clipboard": { "all": true }, "shell": { "all": false } }
```

## Validation Loop

### Level 1: Syntax & Style

```bash
# Run these FIRST - fix any errors before proceeding
cargo fmt --edition 2024             # Format code
cargo clippy -- -Dwarnings          # Lint with warnings as errors
cargo check --features sqlite        # Check sqlite feature
cargo check --features postgres      # Check postgres feature

# Expected: No errors. If errors, READ the error and fix.
```

### Level 2: Unit Tests each new feature/file/function use existing test patterns

```rust
// CREATE tests/integration.rs with these test cases:
#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_add_snippet_happy_path() {
        // Basic functionality works
        let mut conn = establish_test_connection();
        let new_snippet = NewSnippet {
            title: "Test".to_string(),
            content: "Hello {{name}}".to_string(),
            tags: vec!["test".to_string()],
            embedding: None,
        };
        
        let result = add_snippet(&mut conn, new_snippet).unwrap();
        assert_eq!(result.title, "Test");
        assert!(result.id > 0);
    }
    
    #[test]
    fn test_add_snippet_validation_error() {
        // Invalid input returns error
        let mut conn = establish_test_connection();
        let new_snippet = NewSnippet {
            title: "".to_string(),  // Empty title should fail
            content: "content".to_string(),
            tags: vec![],
            embedding: None,
        };
        
        let result = add_snippet(&mut conn, new_snippet);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_expand_placeholders() {
        // Placeholder expansion works correctly
        let content = "Hello {{name}}, welcome to {{place}}";
        let mut vars = std::collections::HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("place".to_string(), "Rustland".to_string());
        
        let result = expand_placeholders(content, &vars);
        assert_eq!(result, "Hello Alice, welcome to Rustland");
    }
}
```

```bash
# Run and iterate until passing:
cargo test --features sqlite         # Test with SQLite
cargo test --features postgres       # Test with PostgreSQL
cargo nextest run                    # Use nextest if available
# If failing: Read error, understand root cause, fix code, re-run
```

### Level 3: Integration Test

```bash
# Build the CLI application
cargo build --release --features sqlite

# Test the CLI commands
./target/release/rustash add "Test Snippet" "Hello {{name}}" --tags test,demo
./target/release/rustash list --filter test
./target/release/rustash use 1

# Expected outputs:
# add: "Added snippet with ID: 1"
# list: Shows fuzzy finder with matching snippets
# use: Copies expanded content to clipboard

# Test database features
sqlite3 rustash.db "SELECT * FROM snippets;"
# Expected: Shows inserted snippet data

# Test vector search (if enabled)
./target/release/rustash search "greeting message"
# Expected: Returns similar snippets ranked by embedding similarity
```

## Final validation Checklist

* [ ] All tests pass: `cargo test --features sqlite && cargo test --features postgres`
* [ ] No linting errors: `cargo clippy -- -Dwarnings`
* [ ] No formatting issues: `cargo fmt --edition 2024 --check`
* [ ] Both feature flags compile: `cargo build --features sqlite && cargo build --features postgres`
* [ ] CLI commands work: `rustash add/list/use` complete successfully
* [ ] Fuzzy finder integration: `rustash list` opens fzf and returns selection
* [ ] Clipboard integration: `rustash use <id>` copies to clipboard
* [ ] Database migrations: `diesel migration run` succeeds on both backends
* [ ] Vector search: `rustash search` returns similarity-ranked results
* [ ] Performance: `rustash list` responds within 150ms on 1000+ snippets
* [ ] Error handling: Invalid commands show helpful error messages
* [ ] Documentation: README.md has usage examples and setup instructions

---

## Anti-Patterns to Avoid

* ❌ Don't create new patterns when existing ones work
* ❌ Don't skip validation because "it should work"
* ❌ Don't ignore failing tests - fix them
* ❌ Don't use sync functions in async context
* ❌ Don't hardcode values that should be config
* ❌ Don't catch all exceptions - be specific


The inspiration for the project is
https://github.com/siekman-io/PromptManager

please rfer to it for general high-level design and architecture