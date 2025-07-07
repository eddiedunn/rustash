# Rustash

**A modern, high-performance snippet manager built in Rust.**

Rustash is a command-line tool that helps you manage and use code snippets efficiently. It's designed for developers who want quick access to their frequently used commands, code blocks, or any text snippets.

## ✨ Features

- **Lightning Fast**: Built in Rust for maximum performance
- **Powerful Search**: Find snippets instantly with fuzzy search and tags
- **Template Variables**: Use `{{variables}}` in your snippets for dynamic content
- **Clipboard Integration**: Copy snippets to your clipboard with a single command
- **SQLite Backend**: All your snippets stored in a single, portable file
- **Tag System**: Organize snippets with multiple tags for easy retrieval
- **Interactive Mode**: Fill in template variables on the fly

## 🚀 Quick Start

### Installation

```bash
# Build from source (requires Rust 1.70+)
git clone https://github.com/yourusername/rustash.git
cd rustash
cargo install --path .
```

### Basic Usage

```bash
# Add a new snippet
rustash add "Git Commit" "git commit -m '{{message}}'" --tags git,version-control

# List all snippets
rustash list

# Search snippets
rustash list --filter "commit"
rustash list --tag git

# Use a snippet (copies to clipboard by default)
rustash use 1 --var message="Initial commit"

# Interactive mode (prompts for variables)
rustash use 1 --interactive
```

## 📚 Documentation

For detailed documentation, see the [User Guide](USER_GUIDE.md).

## 💻 For Developers

This project is built using an innovative **AI-driven development methodology**. To learn how to contribute, please read our guide: [CONTRIBUTING_WITH_AI.md](CONTRIBUTING_WITH_AI.md).

### Project Structure

The project is organized as a Cargo workspace with two main crates:
- `crates/rustash-core`: Core library with database and business logic
- `crates/rustash-cli`: Command-line interface

### Building and Testing

```bash
# Build the project
cargo build --release

# Run tests
cargo test

# Run linter
cargo clippy

# Run formatter
cargo fmt
```

## 🤝 Contributing

Contributions are welcome! Please see our [contribution guidelines](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🔗 Related Projects

- [PromptManager](https://github.com/siekman-io/PromptManager) - Original inspiration for this project