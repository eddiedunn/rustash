# Rustash 1.0

A modern Rust-based snippet manager with CLI interface, powered by Diesel ORM and SQLite.

## Features

- **Fast snippet management**: Add, list, search, and use code snippets
- **Variable expansion**: Use placeholders like `{{name}}` in snippets
- **Tag-based organization**: Organize snippets with multiple tags
- **Multiple output formats**: Table, compact, detailed, and JSON
- **Fuzzy searching**: Filter snippets by title, content, or tags
- **Clipboard integration**: Automatically copy expanded snippets to clipboard
- **SQLite backend**: Lightweight, single-file database (PostgreSQL support planned)

## Quick Start

### Installation

```bash
# Build from source
cargo build --release --features sqlite

# The binary will be at target/release/rustash
```

### Basic Usage

```bash
# Set database location (optional, defaults to rustash.db)
export DATABASE_URL=path/to/your/snippets.db

# Add a snippet
rustash add "Hello World" "echo 'Hello {{name}}!'" --tags greeting,demo

# List all snippets
rustash list

# Filter snippets
rustash list --filter rust
rustash list --tag git

# Use a snippet with variable substitution
rustash use 1 --var name=Alice

# Interactive mode with variable prompting
rustash use 1 --interactive

# Different output formats
rustash list --format detailed
rustash list --format json
```

## Commands

### `rustash add`
Add a new snippet to the database.

```bash
rustash add "Snippet Title" "snippet content" --tags tag1,tag2

# Read content from stdin
echo "some content" | rustash add "Title" --stdin --tags tag1
```

### `rustash list`
List and search snippets.

```bash
# List all snippets (table format)
rustash list

# Filter by text in title or content
rustash list --filter "rust function"

# Filter by tag
rustash list --tag rust

# Use full-text search
rustash list --search --filter "search term"

# Limit results
rustash list --limit 10

# Different formats
rustash list --format compact     # ID: Title [tags]
rustash list --format detailed    # Full snippet information
rustash list --format json        # JSON output
rustash list --format ids         # Just IDs
```

### `rustash use`
Use a snippet with variable expansion and clipboard copy.

```bash
# Use snippet by ID with variables
rustash use 1 --var name=Alice --var project=rustash

# Interactive variable prompting
rustash use 1 --interactive

# Just print without copying to clipboard
rustash use 1 --print-only
```

## Database Features

- **SQLite backend**: Fast, embedded database with no server required
- **Diesel ORM**: Type-safe database queries and migrations
- **Full-text search**: Fast text search across titles and content
- **Automatic timestamps**: Created and updated timestamps for all snippets

## Variable Expansion

Snippets can contain variables in the format `{{variable_name}}`. When using a snippet:

```bash
# Snippet content: "git commit -m '{{message}}' && git push"
rustash use 3 --var message="feat: add new feature"
# Result: "git commit -m 'feat: add new feature' && git push"
```

## Project Structure

```
rustash/
├── crates/
│   ├── rustash-core/          # Core library with database logic
│   │   ├── src/
│   │   │   ├── models.rs      # Diesel ORM models
│   │   │   ├── schema.rs      # Generated database schema
│   │   │   ├── snippet.rs     # CRUD operations
│   │   │   └── database.rs    # Connection management
│   │   └── migrations/        # Database migrations
│   └── rustash-cli/           # CLI application
│       └── src/
│           ├── commands/      # CLI command implementations
│           ├── fuzzy.rs       # Fuzzy finder integration
│           └── utils.rs       # Utility functions
├── Cargo.toml                 # Workspace configuration
└── README.md                  # This file
```

## Development

### Prerequisites

- Rust 1.70+ (2024 edition)
- Diesel CLI: `cargo install diesel_cli --no-default-features --features sqlite`

### Building

```bash
# Check code
cargo check --features sqlite

# Run tests
cargo test --features sqlite

# Build release
cargo build --release --features sqlite

# Run linting
cargo clippy --features sqlite
```

### Database Setup

The database is created automatically when first run. For development:

```bash
# Run migrations manually
diesel setup
diesel migration run
```

## Validation Checklist

- [x] All tests pass: `cargo test --features sqlite`
- [x] No linting errors: `cargo clippy --features sqlite`
- [x] SQLite feature compiles: `cargo build --features sqlite`
- [x] CLI commands work: `rustash add/list/use` complete successfully
- [x] Database migrations: `diesel migration run` succeeds
- [x] Variable expansion: `rustash use <id> --var key=value` works
- [x] Filtering works: `rustash list --filter text` and `--tag tag` work
- [x] Multiple output formats work
- [x] Clipboard integration functional

## Inspiration

This project was inspired by [PromptManager](https://github.com/siekman-io/PromptManager), reimagined in Rust with modern tooling and type safety.

## License

MIT OR Apache-2.0