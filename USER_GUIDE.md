# Rustash User Guide

Welcome to Rustash! This guide shows how to configure Stashes and use the CLI for snippets, RAG search, and graph relationships.

## Table of Contents
- [Stash Configuration](#stash-configuration)
- [Snippet Commands](#snippet-commands)
- [RAG Commands](#rag-commands)
- [Graph Commands](#graph-commands)

## Stash Configuration
Create `~/.config/rustash/stashes.toml` and define one or more stashes.

```toml
# Default stash when --stash is omitted
default_stash = "my-snippets"

[stashes.my-snippets]
service_type = "Snippet"
database_url = "sqlite://snippets.db"

[stashes.my-rag]
service_type = "RAG"
database_url = "sqlite://rag.db"

[stashes.my-kg]
service_type = "KnowledgeGraph"
database_url = "sqlite://kg.db"
```

Use `rustash stash list` to view or manage these entries.

## Snippet Commands
Operate on a `Snippet` stash.

```bash
# Add a snippet
rustash --stash my-snippets snippets add "Greet" "echo hi" --tags example

# List snippets
rustash --stash my-snippets snippets list
```

## RAG Commands
Operate on a `RAG` stash for vector search.

```bash
# Add a document
rustash --stash my-rag rag add --path doc.txt --title "Doc"

# Query similar documents
rustash --stash my-rag rag query --text "some text" --limit 5
```

## Graph Commands
Operate on a `KnowledgeGraph` stash.

```bash
# Link two snippet UUIDs
rustash --stash my-kg graph link --from <UUID_A> --to <UUID_B> --relation CONNECTS_TO

# List neighbors of a snippet
rustash --stash my-kg graph neighbors --id <UUID_A>
```

Enjoy using Rustash!
