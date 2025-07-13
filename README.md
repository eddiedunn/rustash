# 🚀 Rustash

**A modern, high-performance snippet manager for developers, built in Rust.**

Rustash helps you manage, search, and use code snippets efficiently across multiple storage backends. Whether you're working locally with SQLite or need the power of PostgreSQL with Apache AGE for graph relationships, Rustash has you covered.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://github.com/rustash/rustash/actions/workflows/rust.yml/badge.svg)](https://github.com/rustash/rustash/actions)
[![Documentation](https://img.shields.io/badge/Docs-USER_GUIDE-blue)](USER_GUIDE.md)

## ✨ Features

- **Multi-Backend Support**
  - SQLite for local development (default)
  - PostgreSQL with Apache AGE for graph capabilities
  - In-memory backend for testing
  - Named *Stashes* choose which backend to use via `stashes.toml`

- **Powerful Snippet Management**
  - Full-text search with advanced filtering
  - Template variables with `{{placeholders}}`
  - Tag system for organization
  - Clipboard integration
  - Interactive mode for variable input

- **Developer Friendly**
  - Blazing fast (built in Rust)
  - Containerized testing
  - Multiple output formats (table, JSON, simple lists)
  - Comprehensive documentation

## 🚀 Quick Start

### Installation

```bash
# Install from source (requires Rust 1.70+)
cargo install --path .

# Or install from crates.io (when published)
# cargo install rustash
```

### Your First Snippet

```bash
# Add a new snippet
rustash --stash main snippets add "Docker Run PostgreSQL" \
  "docker run --name postgres -e POSTGRES_PASSWORD=mysecretpassword -p 5432:5432 -d postgres" \
  --tags docker,postgres

# List snippets
rustash --stash main snippets list

# Search snippets
rustash --stash main snippets list --filter "postgres"

# Use a snippet (copies to clipboard)
rustash --stash main snippets use 1
```

### Using Template Variables

```bash
# Add a snippet with placeholders
rustash --stash main snippets add "Git Commit" "git commit -m '{{message}}'" --tags git

# Use with variables
rustash --stash main snippets use 1 --var message="feat: Add new feature"

# Or use interactive mode
rustash --stash main snippets use 1 --interactive
```

### Stashes

Stashes are named collections of data. Define them in `~/.config/rustash/stashes.toml`:

```toml
default_stash = "main"

[stashes.main]
service_type = "Snippet"
database_url = "sqlite://~/.config/rustash/rustash.db"
```

Use the `--stash` flag to target one:

```bash
rustash --stash main snippets list
```

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [User Guide](USER_GUIDE.md) | Complete guide to using Rustash CLI |
| [Architecture](ARCHITECTURE.md) | Technical architecture and design decisions |
| [AI Contribution Guide](CONTRIBUTING_WITH_AI.md) | How we use AI in development |

## 🧪 Testing

```bash
# Run all tests (requires Docker for PostgreSQL tests)
make test-all

# Run SQLite tests only
make test-sqlite

# Run PostgreSQL tests (requires Docker)
make test-postgres
```

## 💻 Development

### Project Structure

```
rustash/
├── crates/
│   ├── rustash-core/    # Core library with business logic
│   └── rustash-cli/     # Command-line interface
├── PRPs/               # Product Requirement Prompts
└── .claude/            # AI development configurations
```

### Common Tasks

```bash
# Build in release mode
cargo build --release

# Run tests
cargo nextest run

# Run linter
cargo clippy -- -Dwarnings

# Format code
cargo fmt
```

## 🤝 Contributing

We welcome contributions! Please see our [Contribution Guidelines](CONTRIBUTING.md) for details on how to get started.

This project follows an **AI-driven development** approach. Check out our [AI Contribution Guide](CONTRIBUTING_WITH_AI.md) to learn more.

## 📄 License

This project is licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🔗 Related Projects

- [PromptManager](https://github.com/siekman-io/PromptManager) - Original inspiration for this project