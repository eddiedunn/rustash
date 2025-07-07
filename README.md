# Rustash

**A modern, high-performance, multi-backend snippet manager built in Rust.**

Rustash is a command-line tool that helps you manage, search, and use code snippets efficiently. It supports multiple storage backends (SQLite and PostgreSQL with Apache AGE) and is designed for developers who need fast, reliable access to their code snippets and commands.

This project is built using an innovative **AI-driven development methodology**. To learn more, see our [AI Contribution Guide](CONTRIBUTING_WITH_AI.md).

## ✨ Features

- **Multi-Backend Support**: Choose between SQLite (local) or PostgreSQL with Apache AGE (scalable)
- **Blazing Fast**: Built in Rust for maximum performance
- **Powerful Search**: Full-text search with advanced filtering
- **Template Variables**: Use `{{placeholders}}` for dynamic content
- **Clipboard Integration**: Copy snippets with a single command
- **Tag System**: Organize snippets with multiple tags
- **Interactive Mode**: Fill in variables on the fly
- **Multiple Formats**: View output as table, JSON, or simple lists
- **Containerized Testing**: Comprehensive test suite with Docker support
- **Graph Relationships**: Create relationships between snippets with Apache AGE

## 🚀 Quick Start

### Prerequisites

- Rust (1.70+)
- Docker and Docker Compose (for PostgreSQL/AGE backend and testing)
- SQLite (for local development)

### Installation

```bash
# Install from source
cargo install --path .

# Or install from crates.io (when published)
# cargo install rustash
```

### Basic Usage

```bash
# Add a new snippet (defaults to SQLite)
rustash add "Git Commit" "git commit -m '{{message}}'" --tags git,template

# List all snippets
rustash list

# Search snippets with a text filter and a tag
rustash list --filter "commit" --tag git

# Use a snippet with a variable (copies to clipboard)
rustash use 1 --var message="feat: Add new feature"

# Interactive mode (prompts for variables)
rustash use 1 --interactive

# Use PostgreSQL backend
DATABASE_URL=postgres://user:pass@localhost:5432/rustash rustash list
```

## 🧪 Testing

Run tests with the built-in test runner:

```bash
# Run SQLite tests (no Docker required)
make test-sqlite

# Run PostgreSQL tests (requires Docker)
make test-postgres

# Run all tests
make test-all

# Run tests in a containerized environment
make test-container
```

See the [Testing Documentation](USER_GUIDE.md#testing-infrastructure) for more details.

## 📚 Documentation

*   **[User Guide](USER_GUIDE.md)**: For users of the Rustash CLI.
*   **[Architecture Guide](ARCHITECTURE.md)**: For developers contributing to Rustash.
*   **[AI Contribution Guide](CONTRIBUTING_WITH_AI.md)**: Our guide to AI-driven development.

## 💻 Development

The project is a Rust workspace managed with Cargo. We use an `xtask` based build system for automation.

### Project Structure

The project is organized as a Cargo workspace with two main crates:
- `crates/rustash-core`: The core library containing all business logic, database models, and operations.
- `crates/rustash-cli`: The command-line interface application.

### Building and Testing

```bash
# Build in release mode
cargo build --release

# Run tests
cargo nextest run

# Run linter
cargo clippy -- -Dwarnings

# Run formatter
cargo fmt --check
```

## 🤝 Contributing

Contributions are welcome! Please see our [Contribution Guidelines](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Related Projects

- [PromptManager](https://github.com/siekman-io/PromptManager) - Original inspiration for this project