#!/bin/bash
set -euo pipefail

# CrewChief Web UI Docker Test Script
# Tests Docker configuration and validates the setup

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Test configuration
TEST_TIMEOUT=120  # seconds
CLEANUP_ON_EXIT=true

# Function to print test status
print_test() {
    local status="$1"
    local message="$2"
    
    case $status in
        "PASS")
            echo -e "${GREEN}✅ PASS:${NC} $message"
            ;;
        "FAIL")
            echo -e "${RED}❌ FAIL:${NC} $message"
            ;;
        "WARN")
            echo -e "${YELLOW}⚠️  WARN:${NC} $message"
            ;;
        "INFO")
            echo -e "${BLUE}ℹ️  INFO:${NC} $message"
            ;;
        "TEST")
            echo -e "${BLUE}🧪 TEST:${NC} $message"
            ;;
    esac
}

# Function to cleanup test environment
cleanup() {
    if [[ "$CLEANUP_ON_EXIT" == "true" ]]; then
        print_test "INFO" "Cleaning up test environment..."
        cd "$PROJECT_ROOT"
        
        # Stop and remove test containers
        docker compose -f docker-compose.yml -f docker-compose.dev.yml --project-name crewchief-test down --remove-orphans --volumes 2>/dev/null || true
        
        # Remove test images
        docker rmi crewchief-test-web-ui:latest 2>/dev/null || true
    fi
}

# Set trap for cleanup
trap cleanup EXIT

# Function to wait for service to be healthy
wait_for_service() {
    local service="$1"
    local url="$2"
    local timeout="$3"
    local count=0
    
    print_test "TEST" "Waiting for $service to be healthy..."
    
    while [[ $count -lt $timeout ]]; do
        if curl -sf "$url" >/dev/null 2>&1; then
            print_test "PASS" "$service is healthy"
            return 0
        fi
        
        sleep 1
        ((count++))
        
        if [[ $((count % 10)) -eq 0 ]]; then
            print_test "INFO" "Still waiting for $service... ($count/${timeout}s)"
        fi
    done
    
    print_test "FAIL" "$service failed to become healthy within ${timeout}s"
    return 1
}

# Function to test HTTP endpoint
test_endpoint() {
    local name="$1"
    local url="$2"
    local expected_status="${3:-200}"
    
    print_test "TEST" "Testing $name endpoint: $url"
    
    local response
    response=$(curl -s -w "%{http_code}" -o /tmp/test_response "$url" 2>/dev/null) || {
        print_test "FAIL" "$name endpoint unreachable"
        return 1
    }
    
    if [[ "$response" == "$expected_status" ]]; then
        print_test "PASS" "$name endpoint returned HTTP $response"
        return 0
    else
        print_test "FAIL" "$name endpoint returned HTTP $response (expected $expected_status)"
        return 1
    fi
}

# Function to test database connection
test_database() {
    print_test "TEST" "Testing database connection..."
    
    if docker compose --project-name crewchief-test exec -T postgres psql -U postgres -d crewchief_test -c "SELECT version();" >/dev/null 2>&1; then
        print_test "PASS" "Database connection successful"
        return 0
    else
        print_test "FAIL" "Database connection failed"
        return 1
    fi
}

# Function to test Redis connection
test_redis() {
    print_test "TEST" "Testing Redis connection..."
    
    if docker compose --project-name crewchief-test exec -T redis redis-cli ping >/dev/null 2>&1; then
        print_test "PASS" "Redis connection successful"
        return 0
    else
        print_test "FAIL" "Redis connection failed"
        return 1
    fi
}

# Main test function
run_tests() {
    local test_failed=false
    
    echo -e "${BLUE}🧪 CrewChief Web UI Docker Test Suite${NC}"
    echo -e "${BLUE}=====================================${NC}"
    echo ""
    
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Test 1: Check Docker availability
    print_test "TEST" "Checking Docker availability..."
    if command -v docker >/dev/null 2>&1; then
        print_test "PASS" "Docker is available"
        print_test "INFO" "Docker version: $(docker --version)"
    else
        print_test "FAIL" "Docker is not available"
        return 1
    fi
    
    # Test 2: Check Docker Compose availability
    print_test "TEST" "Checking Docker Compose availability..."
    if docker compose version >/dev/null 2>&1; then
        print_test "PASS" "Docker Compose is available"
        print_test "INFO" "Docker Compose version: $(docker compose version --short)"
    else
        print_test "FAIL" "Docker Compose is not available"
        return 1
    fi
    
    # Test 3: Check required files
    print_test "TEST" "Checking required files..."
    local required_files=(
        "packages/web-ui/Dockerfile"
        "packages/web-ui/Dockerfile.dev"
        "packages/web-ui/.dockerignore"
        "docker-compose.yml"
        "docker-compose.dev.yml"
        ".env.example"
    )
    
    for file in "${required_files[@]}"; do
        if [[ -f "$file" ]]; then
            print_test "PASS" "$file exists"
        else
            print_test "FAIL" "$file is missing"
            test_failed=true
        fi
    done
    
    # Test 4: Check environment file
    print_test "TEST" "Checking environment configuration..."
    if [[ ! -f ".env" ]]; then
        print_test "WARN" ".env file not found, creating from .env.example"
        cp .env.example .env
        
        # Set test-specific values
        sed -i.bak \
            -e 's/CREWCHIEF_DB_NAME=crewchief/CREWCHIEF_DB_NAME=crewchief_test/' \
            -e 's/CREWCHIEF_DB_PASSWORD=your_secure_password_here/CREWCHIEF_DB_PASSWORD=test_password/' \
            -e 's/REDIS_PASSWORD=your_redis_password_here/REDIS_PASSWORD=test_password/' \
            -e 's/SESSION_SECRET=your_session_secret_change_in_production/SESSION_SECRET=test_session_secret/' \
            -e 's/JWT_SECRET=your_jwt_secret_change_in_production/JWT_SECRET=test_jwt_secret/' \
            .env
        
        print_test "INFO" "Created .env with test configuration"
    else
        print_test "PASS" ".env file exists"
    fi
    
    # Test 5: Build development image
    print_test "TEST" "Building development Docker image..."
    if docker build -f packages/web-ui/Dockerfile.dev -t crewchief-test-web-ui:latest . >/dev/null 2>&1; then
        print_test "PASS" "Development image built successfully"
    else
        print_test "FAIL" "Failed to build development image"
        test_failed=true
    fi
    
    # Test 6: Start services
    print_test "TEST" "Starting test services..."
    
    # Create test environment variables
    export CREWCHIEF_DB_NAME=crewchief_test
    export CREWCHIEF_DB_PASSWORD=test_password
    export REDIS_PASSWORD=test_password
    export SESSION_SECRET=test_session_secret
    export JWT_SECRET=test_jwt_secret
    
    if docker compose -f docker-compose.yml -f docker-compose.dev.yml --project-name crewchief-test up -d >/dev/null 2>&1; then
        print_test "PASS" "Services started successfully"
    else
        print_test "FAIL" "Failed to start services"
        test_failed=true
        return 1
    fi
    
    # Test 7: Wait for services to be healthy
    print_test "TEST" "Waiting for services to become healthy..."
    
    # Wait for PostgreSQL
    if ! wait_for_service "PostgreSQL" "tcp://localhost:5433" 30; then
        test_failed=true
    fi
    
    # Wait for Redis
    sleep 5  # Give Redis a moment to start
    if ! test_redis; then
        test_failed=true
    fi
    
    # Wait for Web UI
    if ! wait_for_service "Web UI" "http://localhost:3456/api/health" 60; then
        test_failed=true
    fi
    
    # Test 8: Test API endpoints
    if ! test_endpoint "Health Check" "http://localhost:3456/api/health"; then
        test_failed=true
    fi
    
    if ! test_endpoint "API Root" "http://localhost:3456/api"; then
        test_failed=true
    fi
    
    # Test 9: Test database
    if ! test_database; then
        test_failed=true
    fi
    
    # Test 10: Test application functionality
    print_test "TEST" "Testing application functionality..."
    
    # Test that the web server is serving content
    if curl -sf "http://localhost:3456/" >/dev/null 2>&1; then
        print_test "PASS" "Web server is serving content"
    else
        print_test "FAIL" "Web server is not serving content"
        test_failed=true
    fi
    
    # Test 11: Test environment variables in container
    print_test "TEST" "Testing environment variables in container..."
    
    local env_test
    env_test=$(docker compose --project-name crewchief-test exec -T web-ui printenv NODE_ENV 2>/dev/null || echo "")
    
    if [[ "$env_test" == "development" ]]; then
        print_test "PASS" "Environment variables properly set"
    else
        print_test "FAIL" "Environment variables not properly set (NODE_ENV=$env_test)"
        test_failed=true
    fi
    
    # Summary
    echo ""
    echo -e "${BLUE}📊 Test Summary${NC}"
    echo -e "${BLUE}===============${NC}"
    
    if [[ "$test_failed" == "true" ]]; then
        print_test "FAIL" "Some tests failed"
        echo ""
        echo -e "${RED}❌ Docker setup has issues that need to be resolved${NC}"
        
        # Show logs for debugging
        echo -e "${BLUE}🔍 Service logs for debugging:${NC}"
        docker compose --project-name crewchief-test logs --tail 20
        
        return 1
    else
        print_test "PASS" "All tests passed"
        echo ""
        echo -e "${GREEN}✅ Docker setup is working correctly!${NC}"
        echo ""
        echo -e "${BLUE}🚀 To start development environment:${NC}"
        echo "   ./scripts/docker-run.sh up"
        echo ""
        echo -e "${BLUE}🚀 To start with development tools:${NC}"
        echo "   ./scripts/docker-run.sh up --profile dev-tools"
        echo ""
        echo -e "${BLUE}📖 For more information, see:${NC}"
        echo "   DOCKER.md"
        
        return 0
    fi
}

# Parse command line arguments
CLEANUP_ON_EXIT=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-cleanup)
            CLEANUP_ON_EXIT=false
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Test CrewChief Web UI Docker setup"
            echo ""
            echo "Options:"
            echo "  --no-cleanup    Don't cleanup test environment on exit"
            echo "  -h, --help      Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Run tests
run_tests