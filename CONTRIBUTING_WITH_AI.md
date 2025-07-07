# Contributing to Rustash with AI

Welcome to the Rustash project! This document outlines our innovative AI-driven development methodology that leverages Product Requirement Prompts (PRPs) to guide development. This approach helps maintain consistency, quality, and efficiency across the codebase.

## Table of Contents

1. [What is AI-Driven Development?](#1-what-is-ai-driven-development)
2. [The PRP (Product Requirement Prompt) Methodology](#2-the-prp-methodology)
3. [Development Workflow](#3-development-workflow)
4. [Creating Effective PRPs](#4-creating-effective-prps)
5. [Example PRP](#5-example-prp)
6. [Best Practices](#6-best-practices)
7. [Getting Help](#7-getting-help)

## 1. What is AI-Driven Development?

AI-Driven Development is an approach where AI assistants (like Claude) are actively involved in the software development process. In this project, we use structured prompts called PRPs to guide the AI in making coherent, high-quality contributions.

### Key Benefits:
- **Consistency**: PRPs ensure all code follows the same patterns and standards.
- **Efficiency**: Rapid prototyping and implementation of features.
- **Documentation**: PRPs serve as living documentation of design decisions.
- **Knowledge Sharing**: Makes onboarding easier for new contributors.

## 2. The PRP Methodology

A **Product Requirement Prompt (PRP)** is a structured document that:
1. Defines what needs to be built
2. Provides context about the existing codebase
3. Specifies implementation details and constraints
4. Includes validation criteria

### PRP Structure:
1. **Objective**: Clear statement of what needs to be achieved
2. **Context**: Background information and rationale
3. **Requirements**: Detailed specifications
4. **Implementation Notes**: Technical guidance and considerations
5. **Validation**: How to test and verify the implementation
6. **Dependencies**: Related components or features

## 3. Development Workflow

### Prerequisites
- Python 3.8+
- `uv` package manager
- Claude CLI installed and configured

### Step-by-Step Process

1. **Create a PRP**
   - Use the template in `.claude/PRPs/template.md`
   - Be as specific and detailed as possible

2. **Run the PRP**
   ```bash
   # Install dependencies
   uv sync
   
   # Run the PRP
   uv run PRPs/scripts/prp_runner.py --prp path/to/your/prp.md --interactive
   ```

3. **Review the Changes**
   - The AI will generate code, tests, and documentation
   - Review all changes carefully
   - Run tests to verify functionality

4. **Commit and Push**
   - Create a descriptive commit message
   - Reference the PRP in your commit
   - Push to a feature branch and open a pull request

## 4. Creating Effective PRPs

### Do:
- Be specific about requirements and constraints
- Include examples of similar patterns from the codebase
- Specify error handling and edge cases
- Define acceptance criteria
- Reference relevant files and functions

### Don't:
- Be too vague or open-ended
- Assume knowledge of the codebase
- Forget to include validation steps
- Overlook error handling

## 5. Example PRP

```markdown
# PRP: Add User Authentication

## Objective
Add user authentication to the Rustash CLI using JWT tokens.

## Context
Currently, Rustash stores all snippets in a local database without user separation. We need to add authentication to support multi-user environments and future cloud sync features.

## Requirements
1. Use JWT for stateless authentication
2. Store user credentials securely (hashed passwords)
3. Add `login`, `logout`, and `register` commands
4. Modify existing commands to require authentication
5. Store tokens securely in the system keychain

## Implementation Notes
- Use the `jsonwebtoken` crate for JWT
- Use `keyring` crate for secure token storage
- Follow the existing CLI patterns in `src/commands/`
- Add appropriate error types in `src/error.rs`

## Validation
- Test registration flow
- Test login/logout
- Verify unauthorized access is prevented
- Check token expiration

## Dependencies
- #123 (Database schema updates)
- #124 (Configuration system)
```

## 6. Best Practices

### Code Quality
- Follow Rust idioms and best practices
- Write clear, self-documenting code
- Include doc comments for all public items
- Keep functions small and focused

### Testing
- Write unit tests for all new functionality
- Include integration tests for user flows
- Test error conditions and edge cases
- Ensure tests are deterministic

### Documentation
- Update relevant documentation when making changes
- Include examples in documentation
- Document any breaking changes
- Keep the changelog up to date

## 7. Getting Help

If you need help with the PRP process or have questions:

1. Check the existing PRPs in the `PRPs/` directory
2. Review the codebase for similar patterns
3. Open an issue with the `question` label
4. Join our community chat (link in README.md)

## License

By contributing to Rustash, you agree that your contributions will be licensed under its MIT OR Apache-2.0 license.
