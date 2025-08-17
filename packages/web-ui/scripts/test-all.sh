#!/bin/bash

# CrewChief Web UI - Comprehensive Test Runner
# This script runs all tests in the correct order with proper setup

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if PostgreSQL is running
check_postgres() {
    if command_exists psql; then
        if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
            return 0
        fi
    fi
    return 1
}

# Function to setup test databases
setup_databases() {
    print_status "Setting up test databases..."
    
    if ! check_postgres; then
        print_error "PostgreSQL is not running or not accessible"
        print_status "Please start PostgreSQL and ensure it's accessible on localhost:5432"
        exit 1
    fi

    # Create test databases if they don't exist
    local databases=("crewchief_test" "crewchief_integration_test" "crewchief_e2e_test")
    
    for db in "${databases[@]}"; do
        if psql -h localhost -lqt | cut -d \| -f 1 | grep -qw "$db"; then
            print_status "Database $db already exists"
        else
            print_status "Creating database $db"
            createdb "$db" || print_warning "Failed to create $db (might already exist)"
        fi
    done
    
    print_success "Database setup complete"
}

# Function to run linting and formatting checks
run_checks() {
    print_status "Running code quality checks..."
    
    print_status "Checking TypeScript compilation..."
    if ! pnpm build:server >/dev/null 2>&1; then
        print_error "TypeScript compilation failed for server"
        return 1
    fi
    
    if ! pnpm build:client >/dev/null 2>&1; then
        print_error "TypeScript compilation failed for client"
        return 1
    fi
    
    print_status "Running ESLint..."
    if ! pnpm lint; then
        print_error "Linting failed"
        return 1
    fi
    
    print_status "Checking code formatting..."
    if ! pnpm format:check; then
        print_error "Code formatting check failed"
        print_status "Run 'pnpm format' to fix formatting issues"
        return 1
    fi
    
    print_success "All code quality checks passed"
    return 0
}

# Function to run unit tests
run_unit_tests() {
    print_status "Running unit tests..."
    
    export NODE_ENV=test
    export DATABASE_URL="postgresql://test:test@localhost:5432/crewchief_test"
    
    if pnpm test:unit; then
        print_success "Unit tests passed"
        return 0
    else
        print_error "Unit tests failed"
        return 1
    fi
}

# Function to run integration tests
run_integration_tests() {
    print_status "Running integration tests..."
    
    export NODE_ENV=test
    export DATABASE_URL="postgresql://test:test@localhost:5432/crewchief_integration_test"
    export PGHOST=localhost
    export PGPORT=5432
    export PGUSER=test
    export PGPASSWORD=test
    export PGDATABASE=crewchief_integration_test
    
    if pnpm test:integration; then
        print_success "Integration tests passed"
        return 0
    else
        print_error "Integration tests failed"
        return 1
    fi
}

# Function to run E2E tests
run_e2e_tests() {
    print_status "Running E2E tests..."
    
    # Check if Playwright browsers are installed
    if ! npx playwright --version >/dev/null 2>&1; then
        print_error "Playwright is not installed"
        return 1
    fi
    
    # Install browsers if needed
    print_status "Ensuring Playwright browsers are installed..."
    npx playwright install --with-deps >/dev/null 2>&1 || print_warning "Failed to install some browsers"
    
    export NODE_ENV=test
    export DATABASE_URL="postgresql://test:test@localhost:5432/crewchief_e2e_test"
    export CI=true
    
    if pnpm test:e2e; then
        print_success "E2E tests passed"
        return 0
    else
        print_error "E2E tests failed"
        print_status "Check test-results/ directory for failure details"
        return 1
    fi
}

# Function to generate coverage report
generate_coverage() {
    print_status "Generating coverage report..."
    
    export NODE_ENV=test
    export DATABASE_URL="postgresql://test:test@localhost:5432/crewchief_test"
    
    if pnpm test:coverage; then
        print_success "Coverage report generated"
        print_status "Coverage report available in coverage/ directory"
        
        # Show coverage summary if available
        if [ -f "coverage/coverage-summary.json" ]; then
            print_status "Coverage Summary:"
            node -e "
                const summary = require('./coverage/coverage-summary.json');
                const total = summary.total;
                console.log(\`  Lines: \${total.lines.pct}%\`);
                console.log(\`  Functions: \${total.functions.pct}%\`);
                console.log(\`  Branches: \${total.branches.pct}%\`);
                console.log(\`  Statements: \${total.statements.pct}%\`);
            " 2>/dev/null || print_status "Coverage summary not available"
        fi
        
        return 0
    else
        print_error "Coverage generation failed"
        return 1
    fi
}

# Function to clean up test artifacts
cleanup() {
    print_status "Cleaning up test artifacts..."
    rm -rf coverage/ test-results/ || true
    print_success "Cleanup complete"
}

# Main execution
main() {
    local run_checks_flag=true
    local run_unit_flag=true
    local run_integration_flag=true
    local run_e2e_flag=true
    local generate_coverage_flag=true
    local cleanup_flag=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --no-checks)
                run_checks_flag=false
                shift
                ;;
            --unit-only)
                run_integration_flag=false
                run_e2e_flag=false
                shift
                ;;
            --integration-only)
                run_unit_flag=false
                run_e2e_flag=false
                shift
                ;;
            --e2e-only)
                run_unit_flag=false
                run_integration_flag=false
                shift
                ;;
            --no-coverage)
                generate_coverage_flag=false
                shift
                ;;
            --cleanup)
                cleanup_flag=true
                shift
                ;;
            --help)
                echo "Usage: $0 [options]"
                echo "Options:"
                echo "  --no-checks         Skip linting and formatting checks"
                echo "  --unit-only         Run only unit tests"
                echo "  --integration-only  Run only integration tests"
                echo "  --e2e-only          Run only E2E tests"
                echo "  --no-coverage       Skip coverage report generation"
                echo "  --cleanup           Clean up test artifacts before running"
                echo "  --help              Show this help message"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    print_status "Starting CrewChief Web UI test suite..."
    
    # Check if we're in the right directory
    if [ ! -f "package.json" ] || ! grep -q "\"name\": \"@crewchief/web-ui\"" package.json; then
        print_error "Please run this script from the packages/web-ui directory"
        exit 1
    fi
    
    # Cleanup if requested
    if [ "$cleanup_flag" = true ]; then
        cleanup
    fi
    
    # Install dependencies
    print_status "Installing dependencies..."
    if ! pnpm install; then
        print_error "Failed to install dependencies"
        exit 1
    fi
    
    # Setup databases
    setup_databases
    
    local failed_tests=()
    
    # Run checks
    if [ "$run_checks_flag" = true ]; then
        if ! run_checks; then
            failed_tests+=("checks")
        fi
    fi
    
    # Run unit tests
    if [ "$run_unit_flag" = true ]; then
        if ! run_unit_tests; then
            failed_tests+=("unit")
        fi
    fi
    
    # Run integration tests
    if [ "$run_integration_flag" = true ]; then
        if ! run_integration_tests; then
            failed_tests+=("integration")
        fi
    fi
    
    # Run E2E tests
    if [ "$run_e2e_flag" = true ]; then
        if ! run_e2e_tests; then
            failed_tests+=("e2e")
        fi
    fi
    
    # Generate coverage
    if [ "$generate_coverage_flag" = true ] && [ "$run_unit_flag" = true ]; then
        if ! generate_coverage; then
            failed_tests+=("coverage")
        fi
    fi
    
    # Summary
    echo
    print_status "Test suite complete!"
    
    if [ ${#failed_tests[@]} -eq 0 ]; then
        print_success "All tests passed! ✅"
        exit 0
    else
        print_error "The following test suites failed:"
        for test in "${failed_tests[@]}"; do
            echo "  - $test"
        done
        print_error "Test suite failed! ❌"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"