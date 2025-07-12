#!/bin/bash

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print a header
header() {
    echo -e "\n${GREEN}=== $1 ===${NC}"
}

# Function to print a warning
warn() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Function to run a command and check its exit status
run_command() {
    echo -e "\n$ ${GREEN}$1${NC}"
    eval $1
    local status=$?
    if [ $status -ne 0 ]; then
        echo -e "${RED}Command failed with status $status${NC}"
        return $status
    fi
    return 0
}

# Validate arguments
if [ $# -lt 1 ]; then
    echo "Usage: $0 <backend> [test-args]"
    echo "Backend must be one of: sqlite, postgres"
    exit 1
fi

BACKEND=$1
shift
TEST_ARGS=$@

# Set up environment variables based on backend
case $BACKEND in
    sqlite)
        export DATABASE_URL="sqlite:/tmp/rustash_test.db"
        export RUST_LOG=debug
        ;;
    postgres)
        export DATABASE_URL="postgres://postgres:postgres@test-db:5432/rustash_test"
        export RUST_LOG=debug
        
        # Wait for PostgreSQL to be ready
        header "Waiting for PostgreSQL to be ready..."
        until pg_isready -h test-db -U postgres -d rustash_test > /dev/null 2>&1; do
            echo -n "."
            sleep 1
        done
        echo -e "\n${GREEN}PostgreSQL is ready!${NC}"
        ;;
    *)
        echo "Unknown backend: $BACKEND"
        echo "Must be one of: sqlite, postgres"
        exit 1
        ;;
esac

# Clean up any existing test database
cleanup() {
    if [ "$BACKEND" = "sqlite" ] && [ -f "/tmp/rustash_test.db" ]; then
        rm -f "/tmp/rustash_test.db"
    fi
}

# Register cleanup on exit
trap cleanup EXIT

# Run migrations
header "Running migrations for $BACKEND..."
if ! run_command "cargo run --bin rustash-cli -- migrate"; then
    warn "Failed to run migrations"
    exit 1
fi

# Run tests
header "Running tests for $BACKEND..."
if ! run_command "cargo test --no-default-features --features $BACKEND $TEST_ARGS"; then
    warn "Tests failed for $BACKEND"
    exit 1
fi

header "✅ All $BACKEND tests passed!"
exit 0
