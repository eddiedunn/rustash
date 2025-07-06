# Rustash User Guide

Welcome to Rustash! This guide provides everything you need to know to install, configure, and master the Rustash snippet manager.

## 1. Security Notice

Rustash is designed with security in mind, but there are important security considerations you should be aware of:

### Important Security Notes

1. **Snippet Execution**
   - Rustash **does not** execute snippets automatically, but be cautious when copying and pasting commands from untrusted sources.
   - Always review snippets before executing them, especially those from untrusted sources.

2. **Database Security**
   - By default, Rustash stores snippets in a SQLite database file (`rustash.db`) in your current directory.
   - You can customize the database location using the `DATABASE_URL` environment variable.
   - **Warning**: Be careful when setting `DATABASE_URL` to avoid accidentally overwriting important files.

3. **Sensitive Information**
   - Avoid storing sensitive information like passwords or API keys directly in snippets.
   - Consider using environment variables or a secure credential manager for sensitive data.

For a complete security analysis, see [SECURITY.md](./SECURITY.md).

## 7. Improved Search and Filtering

Rustash now features enhanced search capabilities powered by SQLite's FTS5 full-text search engine. This provides faster and more accurate search results across snippet titles, content, and tags.

### Key Improvements

- **Unified Search**: A single `--filter` flag is used for all text-based searching, providing a consistent and powerful experience.
- **Faster Performance**: Full-text search is significantly faster, especially for large snippet collections.
- **Better Relevance**: Results are ranked by relevance using the BM25 algorithm.
- **Powerful Syntax**: The search supports advanced operators for more precise queries.

### Examples

```bash
# General search for "database" in title or content
rustash list --filter "database"

# Search for snippets tagged "sql"
# This is now as fast as a text search!
rustash list --tag "sql"

# Combined search: find snippets with "postgres" in the text AND tagged "backup"
rustash list --filter "postgres" --tag "backup"

# Search for an exact phrase using quotes
rustash list --filter '\"database migration\"'

# Use boolean operators (NOTE: must be uppercase)
rustash list --filter "postgres OR mysql"
rustash list --filter "backup AND NOT nightly"
```

### Search Syntax

The search supports the following operators:

- `OR`: Match any term (e.g., `database OR postgres`)
- `AND`: Match all terms (e.g., `database AND migration`)
- `-` or `NOT`: Exclude terms (e.g., `database -mysql`)
- `""`: Phrase search (e.g., `"database migration"`)
- `*`: Prefix matching (e.g., `data*` matches "database", "data", etc.)

## 8. Security FAQ

### Is Rustash secure to use?
Yes, Rustash is designed with security in mind. It's written in Rust, which provides memory safety by default, and avoids common security pitfalls like SQL injection by using the Diesel ORM.

### Can Rustash execute commands automatically?
No, Rustash only copies snippets to your clipboard. You must explicitly paste and execute commands yourself.

### How can I secure my snippets?
- Store your database in a secure location (e.g., `~/.config/rustash/`).
- Use filesystem permissions to restrict access to your database.
- Consider using full-disk encryption for sensitive data.

### What should I do if I find a security issue?
Please report security issues by opening an issue on our GitHub repository. For sensitive issues, you can contact the maintainers directly.

## 2. Introduction

**Rustash** is a modern, high-performance snippet manager built in Rust. It lives in your command line, allowing you to quickly add, search, and use code snippets, shell commands, or any other text you need to access frequently.

Powered by a local SQLite database and a fast, fuzzy-searchable interface, Rustash is designed to boost developer productivity by keeping your most-used snippets just a few keystrokes away.

### Key Features

*   **Blazing Fast:** Written in Rust for optimal performance.
*   **Local-First:** Uses a simple SQLite database file, so it works completely offline.
*   **Powerful Search:** Find snippets instantly with simple filters, tag-based lookups, or full-text search.
*   **Interactive Fuzzy Finder:** Visually search and select snippets with an interactive `fzf`-like interface.
*   **Dynamic Placeholders:** Create powerful, reusable templates with `{{variable}}` expansion.
*   **Clipboard Integration:** Automatically copies snippets to your clipboard, ready to paste.
*   **Flexible Output:** Display snippets in tables, compact lists, detailed views, or as JSON for scripting.

## 2. Installation

To use Rustash, you need to build it from the source code.

### Prerequisites

*   Rust toolchain (version 1.70+ recommended). You can install it from [rustup.rs](https://rustup.rs/).
*   Diesel CLI for database migrations:
    ```bash
    cargo install diesel_cli --no-default-features --features sqlite
    ```

### Building from Source

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/rustash/rustash.git
    cd rustash
    ```

2.  **Build the release binary:**
    ```bash
    # This builds the CLI with the default SQLite backend
    cargo build --release --features sqlite
    ```

3.  **Locate the binary:**
    The executable will be located at `target/release/rustash`.

4.  **Add to your PATH (Recommended):**
    For ease of use, move the binary to a directory in your system's `PATH`.
    ```bash
    # Example for Linux/macOS
    sudo mv target/release/rustash /usr/local/bin/
    ```

## 3. Quick Start

Once installed, you can get started right away.

1.  **Initialize the Database:**
    The first time you run a command, Rustash will automatically create a `rustash.db` file in your current directory. For better security, it's recommended to use a standard location:
    ```bash
    # Recommended: Use a standard config directory
    mkdir -p ~/.config/rustash
    export DATABASE_URL=~/.config/rustash/rustash.db
    ```
    
    > **Security Note**: Be cautious when setting a custom `DATABASE_URL` as it could potentially point to sensitive system files if not properly validated.

2.  **Add a Snippet:**
    Let's add a Git commit template with a placeholder.
    ```bash
    rustash add "Git Commit" "git commit -m '{{message}}'" --tags git,template
    ```
    > ✓ Added snippet 'Git Commit' with ID: 1
    >   Tags: git, template

3.  **List Your Snippets:**
    ```bash
    rustash list
    ```
    > ID   Title         Tags              Updated
    > ──── ───────────── ───────────────── ───────────────
    > 1    Git Commit    git, template     2025-07-06 10:30

4.  **Use Your Snippet:**
    Now, use the snippet and fill in the `{{message}}` placeholder.
    ```bash
    rustash use 1 --var message="feat: Add new user profile page"
    ```
    The command will print the expanded snippet and copy `git commit -m 'feat: Add new user profile page'` to your clipboard.

5.  **Interactive Use:**
    If you don't provide variables, use interactive mode to be prompted for them.
    ```bash
    rustash use 1 --interactive
    ```
    > Enter value for 'message': `refactor: Improve database queries`

## 4. Core Concepts

### Snippets
A snippet is a piece of text with a `title`, `content`, and one or more `tags`.

### Placeholders
Snippet content can contain dynamic placeholders using `{{variable_name}}` syntax. These are replaced with values when you use the `rustash use` command.

**Security Consideration**: Be cautious when using placeholders in commands, especially with untrusted input, as they could be used for command injection if the resulting command is executed in a shell.

**Example:**
Snippet Content: `ssh {{user}}@{{host}}`
Command: `rustash use 2 --var user=admin --var host=server1.com`
Result: `ssh admin@server1.com`

### Search and Filtering
Rustash offers three ways to find snippets:
1.  **Simple Filter (`--filter`):** A case-insensitive `LIKE` search on the `title` and `content` fields. Good for quick lookups.
2.  **Tag Filter (`--tag`):** Finds all snippets that include the specified tag.
3.  **Full-Text Search (`--filter`):** A more powerful search that uses a dedicated FTS5 index for relevance-based searching across title, content, and tags.

## 5. Security Best Practices

When using Rustash, follow these security best practices:

1. **Use Official Sources**
   - Only install Rustash from the official repository or trusted package managers.
   - Verify checksums when downloading pre-built binaries.

2. **Regular Updates**
   - Keep Rustash and its dependencies up to date to receive security patches.
   - Run `cargo update` regularly if you built from source.

3. **Secure Your Database**
   - Store your database file in a secure location with proper file permissions.
   - Consider encrypting the database if it contains sensitive information.
   - Back up your database regularly.

4. **Audit Dependencies**
   - Periodically run `cargo audit` to check for known vulnerabilities in dependencies.
   - Review and update dependencies with known security issues.

5. **Be Cautious with Automation**
   - When using Rustash in scripts, validate all inputs.
   - Be especially careful with the `--var` flag to prevent command injection.

## 6. Command Reference

### `rustash add`
Adds a new snippet to the database.

```
Usage: rustash add [OPTIONS] <TITLE> <CONTENT>
```
**Arguments:**
*   `<TITLE>`: The title of the snippet.
*   `<CONTENT>`: The content of the snippet. Can be read from stdin with `--stdin`.

**Options:**
*   `--tags <TAGS>`: A comma-separated list of tags (e.g., `--tags rust,api,example`).
*   `--stdin`: Reads the snippet content from standard input instead of an argument.

**Examples:**
```bash
# Basic add
rustash add "Docker Prune" "docker system prune -af" --tags docker,cli

# Add multi-line content from a file
cat script.sh | rustash add "My Script" --stdin --tags shell,script
```

---

### `rustash list`
Lists and searches for snippets.

```
Usage: rustash list [OPTIONS]
```

**Options:**
*   `-f, --filter <TEXT>`: Filter by text in title or content.
*   `-t, --tag <TAG>`: Filter by a specific tag.
*   `-l, --limit <LIMIT>`: Maximum number of results to show. (Default: 50)
*   `--interactive`: Use a fuzzy finder to interactively select a snippet from the results.
*   `--format <FORMAT>`: Output format. (Default: `table`)
    *   `table`: A detailed, multi-column table.
    *   `compact`: A compact `ID: Title [tags]` list.
    *   `detailed`: A full breakdown of each snippet.
    *   `json`: A JSON array of the snippet objects.
    *   `ids`: A plain list of snippet IDs, one per line.

**Examples:**
```bash
# List all snippets in a table
rustash list

# Find snippets tagged 'rust' in a compact format
rustash list --tag rust --format compact

# Search for snippets (powerful FTS5 search with relevance ranking)
rustash list --filter "database OR postgres"

# Interactively select a snippet
rustash list --interactive
```

---

### `rustash use`
Uses a snippet, expanding placeholders and copying it to the clipboard.

```
Usage: rustash use [OPTIONS] <ID>
```

**Arguments:**
*   `<ID>`: The ID of the snippet to use.

**Options:**
*   `--var <KEY=VALUE>`: Provides a value for a placeholder. Can be used multiple times.
*   `-i, --interactive`: Prompts you to enter values for any placeholders found in the snippet.
*   `--print-only`: Prints the expanded content to standard output without copying to the clipboard.
*   `-c, --copy <BOOL>`: Controls whether to copy to clipboard. (Default: `true`)

**Examples:**
```bash
# Use snippet 1 and provide a variable
rustash use 1 --var name=world

# Use a snippet and get prompted for variables
rustash use 1 --interactive

# Use a snippet but only print it, don't copy
rustash use 1 --print-only
```

## 6. For Developers

### Project Structure
The project is a Cargo workspace with two main crates:
*   `crates/rustash-core`: The core library containing all business logic, database models, and operations.
*   `crates/rustash-cli`: The command-line interface application built with `clap`.

### Database Migrations
Database schema changes are managed by `diesel_migrations`.
```bash
# Setup the database (creates the file and runs migrations)
diesel setup --database-url your_database.db

# Run pending migrations
diesel migration run --database-url your_database.db

# Create a new migration
diesel migration generate my_new_migration
