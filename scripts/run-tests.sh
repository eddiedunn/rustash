#!/bin/bash

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print a header
header() {
    echo -e "\n${GREEN}=== $1 ===${NC}"
}

# Function to run a command and check its exit status
run_command() {
    echo -e "\n$ ${GREEN}$1${NC}"
    eval $1
    if [ $? -ne 0 ]; then
        echo -e "${RED}Command failed with status $?${NC}"
        exit 1
    fi
}

# Start the test database if not already running
header "Starting test database..."
if ! docker ps | grep -q rustash-test-db; then
    run_command "docker-compose -p rustash up -d test-db"
    
    # Wait for PostgreSQL to be ready
    echo -e "\n${GREEN}Waiting for PostgreSQL to be ready...${NC}"
    until docker exec rustash-test-db pg_isready -U postgres > /dev/null 2>&1; do
        echo -n "."
        sleep 1
    done
    echo -e "\n${GREEN}PostgreSQL is ready!${NC}"
fi

# Build the test container if not already built
header "Building test container..."
if ! docker images | grep -q rustash-test; then
    run_command "docker-compose -p rustash build test"
fi

# Run the tests
header "Running tests..."
run_command "docker-compose -p rustash run --rm test \
    cargo test --no-default-features --features 'postgres sqlite' -- --nocapture"

# If we get here, all tests passed
header "✅ All tests passed successfully!"
exit 0
