# 🛠️ Rustash Developer Guide

A quickstart for contributors. See [`ARCHITECTURE.md`](ARCHITECTURE.md) for in-depth design notes.

## Setup
- Install the Rust toolchain with `rustup`.
- Add `clippy` and `rustfmt`: `rustup component add clippy rustfmt`.
- Fetch workspace dependencies: `cargo fetch`.

## Workflow
- Format: `cargo fmt --all`.
- Lint: `cargo clippy --all -- --deny warnings`.
- Test: `cargo test --workspace`.
- Use `make` targets for database-backed tests.

## More Documentation
- [User Guide](USER_GUIDE.md)
- [Architecture](ARCHITECTURE.md)
- [AI Contribution Guide](CONTRIBUTING_WITH_AI.md)
