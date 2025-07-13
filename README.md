# 🚀 Rustash

**A developer-first data platform built around flexible _Stashes_.**

Rustash lets you manage snippets, RAG documents, and knowledge graphs through one CLI. Each stash can use SQLite or PostgreSQL, so you can keep data local or scale to a server.

## ✨ Features

- **Stash System** – Create named stores for Snippets, RAG, or KnowledgeGraph data.
- **Multi-Backend** – Point a stash at a local SQLite file or a remote Postgres DB.
- **Unified CLI** – Common commands for adding, querying, and linking data across modes.
- **Developer Friendly** – Written in modern Rust with async I/O and first-class tests.

## 🚀 Quick Start

1. **Install**
   ```bash
   cargo install --path .
   ```
2. **Configure a stash**
   Create `~/.config/rustash/stashes.toml`:
   ```toml
   default_stash = "my-snippets"

   [stashes.my-snippets]
   service_type = "Snippet"
   database_url = "sqlite://snippets.db"
   ```
3. **Use the CLI**
   ```bash
   rustash stash list
   rustash snippets add "Hello" "echo hello" --tags example
   rustash snippets list
   ```

See `USER_GUIDE.md` for full usage details.

## 📚 Documentation

- [USER_GUIDE.md](USER_GUIDE.md) – Complete CLI and configuration guide.
- [ARCHITECTURE.md](ARCHITECTURE.md) – System design overview.
- [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md) – Contributor setup and workflow.

## 🤝 Contributing

Contributions are welcome! See [CONTRIBUTING_WITH_AI.md](CONTRIBUTING_WITH_AI.md).

## 📄 License

Licensed under MIT OR Apache-2.0.
