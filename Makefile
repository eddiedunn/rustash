# Makefile for Rustash
# 
# Development Commands:
#   setup           - Install development dependencies and setup environment
#   db-setup       - Setup development database and run migrations
#   db-reset       - Reset the development database
#   run            - Run the CLI application with default settings
#   run-gui        - Run the GUI application (if available)
#   lint           - Run linters and code style checks
#   fmt            - Format the code
#   clean          - Clean build artifacts
#   help           - Show this help message
#
# Testing Commands:
#   test           - Run all tests (alias for test-all)
#   test-sqlite    - Run tests with SQLite backend
#   test-postgres  - Run tests with PostgreSQL backend
#   test-all       - Run all tests
#   test-container - Run tests in a containerized environment
#   test-coverage  - Generate test coverage report

# Docker Compose project name
COMPOSE_PROJECT_NAME = rustash

# Docker Compose command with project name
DOCKER_COMPOSE = docker-compose -p $(COMPOSE_PROJECT_NAME)

# Default target
all: help

# Show help message
.PHONY: help
help:
	@echo "\n\033[1mDevelopment Commands:\033[0m"
	@echo "  \033[36msetup\033[0m           - Install development dependencies and setup environment"
	@echo "  \033[36mdb-setup\033[0m       - Setup development database and run migrations"
	@echo "  \033[36mdb-reset\033[0m       - Reset the development database"
	@echo "  \033[36mrun\033[0m            - Run the CLI application with default settings"
	@echo "  \033[36mrun-gui\033[0m        - Run the GUI application (if available)"
	@echo "  \033[36mlint\033[0m           - Run linters and code style checks"
	@echo "  \033[36mfmt\033[0m            - Format the code"
	@echo "  \033[36mclean\033[0m           - Clean build artifacts"
	@echo ""
	@echo "\033[1mTesting Commands:\033[0m"
	@echo "  \033[36mtest\033[0m            - Run all tests (alias for test-all)"
	@echo "  \033[36mtest-sqlite\033[0m    - Run tests with SQLite backend"
	@echo "  \033[36mtest-postgres\033[0m  - Run tests with PostgreSQL backend"
	@echo "  \033[36mtest-all\033[0m       - Run all tests"
	@echo "  \033[36mtest-container\033[0m - Run tests in a containerized environment"
	@echo "  \033[36mtest-coverage\033[0m  - Generate test coverage report"
	@echo ""
	@echo "Use 'make <target>' to run a specific command."

# Run SQLite tests
.PHONY: test-sqlite
test-sqlite:
	@echo "Running SQLite tests..."
	cargo test --no-default-features --features "sqlite" -- --nocapture

# Run PostgreSQL tests
.PHONY: test-postgres
test-postgres: postgres-up
	@echo "Running PostgreSQL tests..."
	RUST_LOG=debug \
	DATABASE_URL=postgres://postgres:postgres@localhost:5432/rustash_test \
	cargo test --no-default-features --features "postgres" -- --nocapture --ignored

# Run all tests
.PHONY: test-all
test-all: test-sqlite test-postgres

# Run tests in a containerized environment
.PHONY: test-container
test-container: build-test-image
	@echo "Running tests in container..."
	docker run --rm -t --network host \
	-e DATABASE_URL=postgres://postgres:postgres@host.docker.internal:5432/rustash_test \
	rustash-test:latest

# Build test container image
.PHONY: build-test-image
build-test-image: Dockerfile.test
	docker build -f Dockerfile.test -t rustash-test .

# Development setup
.PHONY: setup
setup: db-setup
	@echo "Installing development dependencies..."
	cargo install cargo-edit cargo-watch sqlx-cli
	cargo install --path crates/rustash-cli

# Database setup
.PHONY: db-setup
db-setup: db-reset
	@echo "Running database migrations..."
	@mkdir -p $(HOME)/.local/share/rustash
	@touch $(HOME)/.local/share/rustash/rustash.db
	cd crates/rustash-core && \
	DATABASE_URL=sqlite://$(HOME)/.local/share/rustash/rustash.db \
	diesel migration run

# Reset development database
.PHONY: db-reset
db-reset:
	@echo "Resetting development database..."
	@mkdir -p $(HOME)/.local/share/rustash
	rm -f $(HOME)/.local/share/rustash/rustash.db

# Run the application
.PHONY: run
run:
	@echo "Running Rustash CLI..."
	cargo run -- list

# Run the GUI application
.PHONY: run-gui
run-gui:
	@echo "Running Rustash GUI..."
	cargo run -- add

# Lint the code
.PHONY: lint
lint:
	@echo "Running linters..."
	cargo clippy --all-targets --all-features -- -D warnings

# Format the code
.PHONY: fmt
fmt:
	@echo "Formatting code..."
	cargo fmt --all

# Test coverage
.PHONY: test-coverage
test-coverage:
	@echo "Generating test coverage report..."
	cargo tarpaulin --out Html

# Start PostgreSQL container
.PHONY: postgres-up
postgres-up:
	@if [ -z "$$(docker ps -q -f name=$(COMPOSE_PROJECT_NAME)-postgres)" ]; then \
		echo "Starting PostgreSQL container..."; \
		$(DOCKER_COMPOSE) up -d postgres; \
		echo "Waiting for PostgreSQL to be ready..."; \
		sleep 5; \
	else \
		echo "PostgreSQL container is already running"; \
	fi

# Stop PostgreSQL container
.PHONY: postgres-down
postgres-down:
	$(DOCKER_COMPOSE) down

# Clean build artifacts
.PHONY: clean
clean:
	cargo clean
	rm -rf target/

# Show help
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  test-sqlite     - Run tests with SQLite backend"
	@echo "  test-postgres   - Run tests with PostgreSQL backend"
	@echo "  test-all        - Run all tests"
	@echo "  test-container  - Run tests in a containerized environment"
	@echo "  postgres-up     - Start PostgreSQL container"
	@echo "  postgres-down   - Stop PostgreSQL container"
	@echo "  clean           - Clean build artifacts"

.DEFAULT_GOAL := help
