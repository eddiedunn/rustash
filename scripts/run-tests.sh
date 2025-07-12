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

# Function to run tests for a specific backend
run_tests_for_backend() {
    local backend=$1
    local features=$2
    
    header "Running $backend tests..."
    
    # Build with the specific backend
    if ! run_command "cargo build --no-default-features --features '$features' --tests"; then
        warn "Failed to build $backend tests"
        return 1
    fi
    
    # Run tests with the specific backend
    if ! run_command "cargo test --no-default-features --features '$features' -- --nocapture"; then
        warn "Some $backend tests failed"
        return 1
    fi
    
    return 0
}

# Start the test database if not already running
header "Starting test database..."
if ! docker ps | grep -q rustash-test-db; then
    if ! run_command "docker-compose -p rustash up -d test-db"; then
        warn "Failed to start test database. Some tests may fail."
    else
        # Wait for PostgreSQL to be ready
        echo -e "\n${GREEN}Waiting for PostgreSQL to be ready...${NC}"
        local max_attempts=30
        local attempt=0
        until docker exec rustash-test-db pg_isready -U postgres > /dev/null 2>&1; do
            echo -n "."
            attempt=$((attempt + 1))
            if [ $attempt -ge $max_attempts ]; then
                echo -e "\n${YELLOW}PostgreSQL is not ready after $max_attempts attempts. Some tests may fail.${NC}"
                break
            fi
            sleep 1
        done
        echo -e "\n${GREEN}PostgreSQL is ready!${NC}"
    fi
fi

# Build the test container
header "Building test container..."
if ! run_command "docker-compose -p rustash build test"; then
    warn "Failed to build test container"
    exit 1
fi

# Initialize test results
declare -A test_results=(
    [sqlite]=0
    [postgres]=0
)

# Test SQLite backend
if run_command "docker-compose -p rustash run --rm test \
    /bin/bash -c 'cd /app && ./scripts/test-backend.sh sqlite sqlite'"; then
    test_results[sqlite]=1
    echo -e "${GREEN}✅ SQLite tests passed!${NC}"
else
    warn "SQLite tests failed"
fi

# Test PostgreSQL backend
if run_command "docker-compose -p rustash run --rm test \
    /bin/bash -c 'cd /app && ./scripts/test-backend.sh postgres postgres'"; then
    test_results[postgres]=1
    echo -e "${GREEN}✅ PostgreSQL tests passed!${NC}"
else
    warn "PostgreSQL tests failed"
fi

# Print summary
header "Test Summary"
echo -e "SQLite: ${test_results[sqlite]:-0} (1=passed, 0=failed)"
echo -e "PostgreSQL: ${test_results[postgres]:-0} (1=passed, 0=failed)"

# Determine overall success
if [ ${test_results[sqlite]} -eq 1 ] && [ ${test_results[postgres]} -eq 1 ]; then
    header "✅ All tests passed successfully!"
    exit 0
else
    warn "Some tests failed"
    exit 1
fi
