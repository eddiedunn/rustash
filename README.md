# 🚀 Rustash

**A developer-first, multi-modal data platform, built in Rust.**

Rustash helps you manage, search, and use purpose-built data collections called **Stashes**. A Stash can be a simple snippet collection, a vector database for RAG, or a knowledge graph—all accessible through a single, powerful CLI.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://github.com/rustash/rustash/actions/workflows/rust.yml/badge.svg)](https://github.com/rustash/rustash/actions)
[![Documentation](https://img.shields.io/badge/Docs-USER_GUIDE-blue)](USER_GUIDE.md)

## ✨ Features

- **Stash System**: Manage multiple, independent data stashes for different purposes (Snippets, RAG, Graph).
- **Multi-Backend Support**: Configure any stash to use SQLite for local files or PostgreSQL for server-based storage.
- **Multi-Modal Data**: Natively supports relational, vector, and graph data operations through a unified API.
- **Powerful CLI**: A single, intuitive interface to manage all your stashes and their data.
- **Developer Friendly**: Blazing fast, containerized testing, and built with modern Rust.

## 🚀 Quick Start

### 1. Installation

```bash
# Install from source (requires Rust)
cargo install --path .
```

### 2. Configure Your First Stash

Create a config file at `~/.config/rustash/stashes.toml`:

```toml
# Set the default stash to use when --stash is omitted
default_stash = "my-snippets"

[stashes.my-snippets]
service_type = "Snippet"
database_url = "sqlite:///path/to/your/snippets.db"
```

### 3. Use the CLI

```bash
# List your configured stashes
rustash stash list

# Add a new snippet to your default stash
rustash snippets add "Hello World" "echo 'Hello, World!'" --tags shell,example

# List snippets in a specific stash
rustash --stash my-snippets snippets list
```

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [USER_GUIDE.md](USER_GUIDE.md) | Complete guide to using the Rustash CLI and managing stashes. |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Technical architecture and design decisions. |
| [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) | Setup and workflow for contributors. |
| [CONTRIBUTING_WITH_AI.md](CONTRIBUTING_WITH_AI.md) | How we use AI in our development process. |

## 🤝 Contributing

Contributions are welcome! Please see our [Contribution Guidelines](CONTRIBUTING_WITH_AI.md) to learn about our AI-driven development process.

## 📄 License

This project is licensed under the MIT OR Apache-2.0 license.
