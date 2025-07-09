# AGENTS Guide for Cascade

This file provides repository-specific instructions for AI assistants (Cascade, ChatGPT, etc.) working on Rustash. Follow these guidelines **before** making code or documentation changes.

## Quick Index
- [Overview](#overview)
- [Contribution Workflow](#contribution-workflow)
- [Style & Formatting](#style--formatting)
- [Validation / CI](#validation--ci)
- [Work Presentation](#work-presentation)
- [Legacy Developer Notes](#legacy-developer-notes)

## Overview

Workspace structure to focus on:
| Path | What to modify |
|------|----------------|
| `rustash-cli/` | CLI code & tests |
| `rustash-core/` | Core library & database layer |
| `rustash-utils/` | Shared utilities and helpers |
| (Optional) `rustash-desktop/` | Desktop GUI (if present) |
| Docs root | `README.md`, `GETTING_STARTED.md`, `USAGE.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md` |

Do **not** edit files under `docs/legacy/` (archived).

## Contribution Workflow
1. Prefer updating consolidated docs over creating new ones.
2. Combine edits per file in a single `replace_file_content` tool call.
3. After code changes, run:
   ```sh
   cargo fmt --all
   cargo clippy --all -- --deny warnings
   cargo test --workspace
   ```
4. Commit messages and PR titles: `[rustash] <concise description>`.

## Style & Formatting
- Rust 2021, `cargo fmt` for layout.
- No `unwrap()` in library code; use `?` + `thiserror`/`anyhow`.
- Zeroize secrets, avoid logging sensitive data.

## Validation / CI
CI mirrors the commands in §Contribution Workflow.  All checks must pass.

## Work Presentation
- Summaries should be **brief** (≤ 5 lines).
- Bullet lists over paragraphs.
- Link to modified files with backticks.
- Only call expensive tools (grep, view_file) when truly needed.

---

## Legacy Developer Notes

*(The section below is retained from the original `AGENTS.md` for historical context and deeper developer tips. Feel free to skim but the guidelines above take precedence.)*

# AGENTS.md

## Project Overview

This repository contains the code for **Rustash**—a modern, high-performance snippet manager written in Rust.

Key crates:
- `rustash-core`: Core logic and data structures.
- `rustash-cli`: Command-line interface.
- `rustash-desktop`: Desktop GUI (Tauri) – optional.
- `rustash-utils`: Shared utilities used across crates.
- `rustash-macros`: Procedural macros.

## Development Environment Setup

1.  **Install Rust:** Use `rustup` from [rust-lang.org](https://www.rust-lang.org/tools/install).
2.  **Add components:**
    ```sh
    rustup component add clippy rustfmt
    ```
3.  **Install dependencies:**
    ```sh
    cargo fetch
    ```

## Common Commands

- **Build all crates:**
  ```sh
  cargo build --workspace
  ```
- **Run all tests:**
  ```sh
  cargo test --workspace
  ```
- **Check formatting:**
  ```sh
  cargo fmt --all -- --check
  ```
- **Run linter:**
  ```sh
  cargo clippy --all -- --deny warnings
  ```
- **Apply formatting:**
  ```sh
  cargo fmt --all
  ```

## Agent Instructions & Guidelines

- **Primary Goal:** Assist with coding tasks, feature development, bug fixes, refactoring, and test generation for the `rustash` project.
- **Code Style:** Adhere to Rust best practices and the existing style. Use `cargo fmt` and `cargo clippy` as specified above.
- **Testing:** All new code or fixes must include relevant tests. Ensure `cargo test --workspace` passes.
- **Dependencies:** If adding or updating dependencies, use `cargo update` or `cargo add`. Ensure changes are compatible and tests pass.
- **Pull Requests (PRs):** When generating PRs, use the title format: `[rustash] <Concise Description of Changes>`.
- **Security:** Be mindful of security implications, especially when handling cryptographic operations or user data. Refer to `SECURITY.md` for project-specific security considerations.


## Example Agent Prompts

- `Refactor the function 'xyz' in 'rustash-cli/src/some_module.rs' for better error handling.`
- `Add unit tests for the 'search' module in 'rustash-core'.`
- `Investigate and fix issue #123 related to snippet tagging logic.`
- `Update the 'serde' dependency to the latest version across the workspace and ensure all tests pass.`
- `Review the changes in branch 'feature/new-backend-support' and suggest improvements.`

---
*This AGENTS.md is designed for an AI agent with continuous internet access. All previous instructions regarding vendoring, offline environments, and marker files are no longer applicable.*
  `cargo test --workspace --tests`, since they may rely on network communication.
- Run lints: `cargo clippy --all`
- Run formatter: `cargo fmt --all -- --check`
- (Optional) Generate coverage reports with `./coverage.sh`.
- Ensure no secrets or sensitive data are logged or leaked.

## Style & Security
- Follow idiomatic Rust practices.
- Prioritize security and reliability as described in `README.md`.
- Document all new public APIs and update usage examples.

## Pull Requests
- Title format: `[rustash] <Short Description>`
- All PRs must pass CI and review.



