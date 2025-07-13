# 🚀 Rustash User Guide

Welcome to Rustash! This guide helps you install, configure, and master the Rustash data platform.

## Table of Contents

- [Core Concept: Stashes](#-core-concept-stashes)
- [Installation](#-installation)
- [Configuration](#-configuration)
- [CLI Command Reference](#-command-reference)
- [Example: Snippet Stash Workflow](#-example-snippet-stash-workflow)

## 📦 Core Concept: Stashes

The central concept in Rustash is the **Stash**. A Stash is a named, self-contained data store designed for a specific purpose. Each Stash has:
- A unique **name** (e.g., `personal_snippets`).
- A **Service Type** (`Snippet`, `RAG`, or `KnowledgeGraph`), which defines what kind of data it holds and what operations it supports.
- A **Database URL**, which points to its physical storage (either a local SQLite file or a PostgreSQL server).

This allows you to manage different kinds of data for different projects, all through a single tool.

## 🛠️ Installation

```bash
# Install from source (requires Rust toolchain)
git clone https://github.com/your-repo/rustash.git
cd rustash
cargo install --path .
```

## ⚙️ Configuration

Rustash is configured via a TOML file located at `~/.config/rustash/stashes.toml`.

1.  Create the directory: `mkdir -p ~/.config/rustash`
2.  Create the file: `touch ~/.config/rustash/stashes.toml`
3.  Add your stash definitions to the file.

**Example `stashes.toml`:**
```toml
# Set the default stash to use when the --stash flag is omitted
default_stash = "my_snippets"

# A local stash for your code snippets
[stashes.my_snippets]
service_type = "Snippet"
database_url = "sqlite:///Users/your_user/.rustash_data/snippets.db"

# A shared team stash for a RAG system on Postgres
[stashes.team_rag]
service_type = "RAG"
database_url = "postgres://user:pass@db.example.com/rag_db"
```

## ⌨️ CLI Command Reference

The standard command structure is: `rustash [GLOBAL OPTIONS] <SERVICE> <COMMAND>`

**Global Option:**
- `--stash <NAME>`: Specifies which stash to use for the command. If omitted, the `default_stash` from your config is used.

### Stash Management (`rustash stash ...`)

- `rustash stash list`: Lists all stashes defined in your config file.
- `rustash stash add <name> --service-type <type> --database-url <url>`: Adds a new stash to your config.
- `rustash stash remove <name>`: Removes a stash from your config.
- `rustash stash set-default <name>`: Sets the default stash.

### Snippet Service (`rustash snippets ...`)

These commands operate on a stash with `service_type = "Snippet"`.

- `rustash snippets add <title> <content> --tags <t1,t2>`: Add a new snippet.
- `rustash snippets list --filter "text" --tag "tag"`: List and search snippets.
- `rustash snippets use <uuid> --var key=value`: Use a snippet, expanding placeholders and copying it to the clipboard.

*(Note: `RAG` and `KnowledgeGraph` service commands will be documented here as they are implemented.)*

## ✨ Example: Snippet Stash Workflow

1.  **Configure a snippet stash:** Add a `[stashes.my_snippets]` section to your `stashes.toml`.

2.  **Add a snippet:**
    ```bash
    rustash --stash my_snippets snippets add "Greeting" "Hello, {{name}}!" --tags example
    ```

3.  **List your snippets:**
    ```bash
    rustash --stash my_snippets snippets list
    ```

4.  **Use your snippet:**
    ```bash
    # This will prompt you for the 'name' variable interactively
    rustash --stash my_snippets snippets use <uuid-from-list> --interactive
    ```
