# Makefile for Rustash
# 
# Available targets:
#   test-sqlite     - Run tests with SQLite backend
#   test-postgres   - Run tests with PostgreSQL backend
#   test-all        - Run all tests
#   test-container  - Run tests in a containerized environment
#   clean           - Clean build artifacts

# Docker Compose project name
COMPOSE_PROJECT_NAME = rustash

# Docker Compose command with project name
DOCKER_COMPOSE = docker-compose -p $(COMPOSE_PROJECT_NAME)

# Default target
all: test-all

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
