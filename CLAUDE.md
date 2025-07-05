# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

This is the **Rustash** project - a collection of Product Requirement Prompts (PRPs) and AI engineering assets for building software with AI agents. The repository serves as a framework for structured AI-driven development using Claude Code.

## Architecture

### PRP (Product Requirement Prompt) System
- **PRPs/** - Core directory containing structured prompts that guide AI development
- **PRPs/scripts/prp_runner.py** - Python script to execute PRPs with Claude Code
- **PRPs/ai_docs/** - Claude Code documentation and context files
- **PRPs/templates/** - Template files for creating new PRPs

### Key Components
- **pyproject.toml** - Python project configuration for PRP tooling
- **claude_md_files/CLAUDE-RUST.md** - Comprehensive Rust development guidelines
- **PRPs/README.md** - Detailed explanation of the PRP concept and methodology

## Common Commands

### Running PRPs
```bash
# Interactive mode with a specific PRP
uv run PRPs/scripts/prp_runner.py --prp test --interactive

# Headless mode with JSON output
uv run PRPs/scripts/prp_runner.py --prp test --output-format json

# Using a specific PRP file path
uv run PRPs/scripts/prp_runner.py --prp-path PRPs/feature.md --interactive
```

### Python Environment
```bash
# Install dependencies
uv install

# Run scripts with uv
uv run PRPs/scripts/prp_runner.py --help
```

## Development Workflow

### PRP Development Process
1. **Create PRP** - Define requirements in structured markdown format
2. **Execute PRP** - Run with Claude Code using prp_runner.py
3. **Iterate** - Refine based on results and move completed PRPs to PRPs/completed/
4. **Validate** - Ensure all validation gates pass before considering complete

### PRP Structure
PRPs combine traditional Product Requirements Documents with AI-specific context:
- **Context** - Precise file paths, library versions, code examples
- **Implementation Details** - Explicit build strategies, API endpoints, test patterns
- **Validation Gates** - Deterministic checks (tests, linting, type checking)

## Important Guidelines

### When Working with PRPs
- Always use the TodoWrite tool to track implementation progress
- Search existing codebase patterns before implementing new features
- Follow the three-phase approach: Planning → Implementation → Testing
- Move completed PRPs to PRPs/completed/ folder when finished
- Output "DONE" when all tests pass

### Rust Development
- Refer to `claude_md_files/CLAUDE-RUST.md` for comprehensive Rust guidelines
- Follow workspace-first architecture with crates in `crates/` directory
- Maintain 80%+ test coverage with `cargo nextest run`
- Use `cargo clippy -- -Dwarnings` for linting
- Format with `cargo fmt --edition 2024`

### PRP Runner Configuration
- Default model: "claude" (Claude Code CLI)
- Allowed tools: Edit, Bash, Write, MultiEdit, NotebookEdit, WebFetch, Agent, LS, Grep, Read, NotebookRead, TodoRead, TodoWrite, WebSearch
- Output formats: text, json, stream-json
- Interactive mode available for iterative development

## Context Management

The repository includes extensive Claude Code documentation in `PRPs/ai_docs/` covering:
- Memory management and CLAUDE.md usage
- Common workflows and extended thinking
- GitHub Actions integration
- MCP (Model Context Protocol) configuration
- Monitoring and troubleshooting guides

This documentation should be referenced when working with Claude Code features beyond basic coding tasks.