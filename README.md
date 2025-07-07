# Rustash

**A modern, high-performance, local-first snippet manager built in Rust.**

Rustash is a command-line tool that helps you add, search, and use code snippets efficiently directly from your terminal. It's designed for developers who want fast, offline access to their most-used commands, code blocks, and notes.

This project is built using an innovative **AI-driven development methodology**. To learn more, see our [AI Contribution Guide](CONTRIBUTING_WITH_AI.md).

## ✨ Features

- **Blazing Fast**: Built in Rust for maximum performance
- **Powerful Search**: Full-text search with SQLite FTS5 for instant results
- **Template Variables**: Use `{{placeholders}}` for dynamic content
- **Clipboard Integration**: Copy snippets with a single command
- **Local-First**: All data stored in a single SQLite file
- **Tag System**: Organize snippets with multiple tags
- **Interactive Mode**: Fill in variables on the fly
- **Multiple Formats**: View output as table, JSON, or simple lists

## 🚀 Quick Start

### Installation

See the [User Guide](USER_GUIDE.md) for detailed installation and setup instructions.

A quick installation from source:
```bash
cargo install --path .
```

### Basic Usage

```bash
# Add a new snippet
rustash add "Git Commit" "git commit -m '{{message}}'" --tags git,template

# List all snippets
rustash list

# Search snippets with a text filter and a tag
rustash list --filter "commit"
rustash list --tag git

# Use a snippet with a variable (copies to clipboard)
rustash use 1 --var message="feat: Add new feature"

# Interactive mode (prompts for variables)
rustash use 1 --interactive
```

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