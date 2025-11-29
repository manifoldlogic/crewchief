#!/bin/bash
# Integration Test: DKRHUB-2902 - Production Configuration (Image Pull)
# Tests that docker-compose.yml successfully pulls pre-built images from Docker Hub
# and starts all services without attempting local builds.
#
# Expected Behavior:
# - Images pull from Docker Hub (not built locally)
# - All services start correctly
# - Health checks pass
# - No build errors occur

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
COMPOSE_FILE="/workspace/packages/maproom-mcp/config/docker-compose.yml"
IMAGE_NAME="crewchief/maproom-mcp"
IMAGE_TAG="${MAPROOM_VERSION:-latest}"
FULL_IMAGE="${IMAGE_NAME}:${IMAGE_TAG}"
HEALTH_CHECK_TIMEOUT=60
TEST_START_TIME=$(date +%s)

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_BLOCKED=0

echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}DKRHUB-2902: Production Configuration Test${NC}"
echo -e "${BLUE}Testing Docker Hub Image Pull${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""

# Function to log test results
log_pass() {
    echo -e "${GREEN}✓ PASS:${NC} $1"
    ((TESTS_PASSED++))
}

log_fail() {
    echo -e "${RED}✗ FAIL:${NC} $1"
    ((TESTS_FAILED++))
}

log_blocked() {
    echo -e "${YELLOW}⊘ BLOCKED:${NC} $1"
    ((TESTS_BLOCKED++))
}

log_info() {
    echo -e "${BLUE}ℹ INFO:${NC} $1"
}

# Function to cleanup Docker resources
cleanup_docker() {
    echo ""
    echo -e "${BLUE}Cleaning up Docker resources...${NC}"

    # Stop and remove containers
    if docker-compose -f "$COMPOSE_FILE" ps -q 2>/dev/null | grep -q .; then
        docker-compose -f "$COMPOSE_FILE" down -v 2>&1 | sed 's/^/  /'
    fi

    # Remove maproom-related images
    if docker images | grep -q maproom; then
        docker images | grep maproom | awk '{print $3}' | xargs -r docker rmi -f 2>&1 | sed 's/^/  /' || true
    fi

    # Prune system
    docker system prune -af --volumes 2>&1 | sed 's/^/  /' || true

    log_info "Docker cleanup completed"
}

# Function to check if images exist on Docker Hub
check_dockerhub_availability() {
    echo ""
    echo -e "${BLUE}Phase 1: Checking Docker Hub Image Availability${NC}"
    echo "Image: $FULL_IMAGE"
    echo ""

    # Try to pull the image
    if docker pull "$FULL_IMAGE" 2>&1 | tee /tmp/docker-pull-test.log | sed 's/^/  /'; then
        log_pass "Image $FULL_IMAGE is available on Docker Hub"

        # Verify image exists locally
        if docker images "$FULL_IMAGE" | grep -q "$IMAGE_TAG"; then
            log_pass "Image successfully pulled and cached locally"
            return 0
        else
            log_fail "Image pull succeeded but image not found in local cache"
            return 1
        fi
    else
        # Check error message
        if grep -q "repository does not exist\|pull access denied" /tmp/docker-pull-test.log; then
            log_blocked "Image $FULL_IMAGE not found on Docker Hub"
            echo ""
            echo -e "${YELLOW}BLOCKER DETECTED:${NC}"
            echo "  This ticket is blocked by DKRHUB-1901: Images must be published to Docker Hub first"
            echo ""
            echo "  To resolve this blocker:"
            echo "  1. Execute the test plan in: .crewchief/work-tickets/DKRHUB-1901_TEST_PLAN.md"
            echo "  2. Create and push tag: git tag -a v1.1.10-rc1 -m 'Test release'"
            echo "  3. Monitor GitHub Actions workflow for image publication"
            echo "  4. Verify images appear on Docker Hub: https://hub.docker.com/r/crewchief/maproom-mcp"
            echo "  5. Re-run this test after images are published"
            echo ""
            return 2  # Blocked status
        else
            log_fail "Image pull failed with unexpected error"
            cat /tmp/docker-pull-test.log
            return 1
        fi
    fi
}

# Function to verify image source and metadata
verify_image_metadata() {
    echo ""
    echo -e "${BLUE}Phase 2: Verifying Image Metadata${NC}"
    echo ""

    # Check image source
    local image_id=$(docker images -q "$FULL_IMAGE")
    if [ -n "$image_id" ]; then
        log_pass "Image ID found: $image_id"
    else
        log_fail "Could not find image ID"
        return 1
    fi

    # Verify image repository
    local image_repo=$(docker inspect "$FULL_IMAGE" --format='{{.RepoTags}}' 2>/dev/null)
    if echo "$image_repo" | grep -q "$IMAGE_NAME"; then
        log_pass "Image repository verified: $IMAGE_NAME"
    else
        log_fail "Image repository mismatch"
        return 1
    fi

    # Check image size
    local image_size=$(docker images "$FULL_IMAGE" --format "{{.Size}}")
    log_info "Image size: $image_size"

    # Verify labels (metadata from build)
    log_info "Checking image labels..."
    docker inspect "$FULL_IMAGE" --format='{{json .Config.Labels}}' 2>/dev/null | jq '.' | sed 's/^/  /'

    return 0
}

# Function to test production configuration
test_production_configuration() {
    echo ""
    echo -e "${BLUE}Phase 3: Testing Production Configuration${NC}"
    echo ""

    # Change to config directory
    cd "$(dirname "$COMPOSE_FILE")"

    # Start services
    log_info "Starting services with docker-compose..."
    echo ""

    # Capture docker-compose up output
    # Use -f flag explicitly to avoid loading docker-compose.override.yml
    if docker-compose -f "$(basename "$COMPOSE_FILE")" up -d 2>&1 | tee /tmp/docker-compose-up.log | sed 's/^/  /'; then
        log_pass "docker-compose up completed successfully"
    else
        log_fail "docker-compose up failed"
        return 1
    fi

    # Verify no build was attempted
    if grep -q "Building\|build" /tmp/docker-compose-up.log; then
        log_fail "docker-compose attempted to build images (should only pull)"
        echo "Build output detected in docker-compose logs:"
        grep -i "build" /tmp/docker-compose-up.log | sed 's/^/  /'
        return 1
    else
        log_pass "No build attempted (pull-only configuration verified)"
    fi

    # Wait for services to start
    log_info "Waiting for services to stabilize..."
    sleep 15

    return 0
}

# Function to verify all containers are running
verify_containers_running() {
    echo ""
    echo -e "${BLUE}Phase 4: Verifying Container Status${NC}"
    echo ""

    # Check for maproom containers
    local container_count=$(docker ps --filter "name=maproom" --format "{{.Names}}" | wc -l)

    if [ "$container_count" -ge 3 ]; then
        log_pass "All 3 maproom containers are running"
        echo ""
        echo "Running containers:"
        docker ps --filter "name=maproom" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | sed 's/^/  /'
    else
        log_fail "Expected 3 containers, found $container_count"
        echo "Running containers:"
        docker ps --filter "name=maproom" | sed 's/^/  /'
        return 1
    fi

    # Verify specific containers
    local containers=("maproom-mcp" "maproom-postgres" "maproom-ollama")
    for container in "${containers[@]}"; do
        if docker ps --filter "name=$container" --format "{{.Names}}" | grep -q "$container"; then
            log_pass "Container $container is running"
        else
            log_fail "Container $container is not running"
        fi
    done

    return 0
}

# Function to verify health checks
verify_health_checks() {
    echo ""
    echo -e "${BLUE}Phase 5: Verifying Health Checks${NC}"
    echo ""

    log_info "Waiting up to ${HEALTH_CHECK_TIMEOUT}s for health checks to pass..."

    local elapsed=0
    local interval=5

    while [ $elapsed -lt $HEALTH_CHECK_TIMEOUT ]; do
        # Check postgres health
        local postgres_health=$(docker inspect maproom-postgres --format='{{.State.Health.Status}}' 2>/dev/null || echo "unknown")

        # Check mcp health
        local mcp_health=$(docker inspect maproom-mcp --format='{{.State.Health.Status}}' 2>/dev/null || echo "unknown")

        echo -e "  Postgres: $postgres_health | MCP: $mcp_health (${elapsed}s elapsed)"

        if [ "$postgres_health" = "healthy" ] && [ "$mcp_health" = "healthy" ]; then
            echo ""
            log_pass "All health checks passed within ${elapsed}s"
            return 0
        fi

        sleep $interval
        elapsed=$((elapsed + interval))
    done

    echo ""
    log_fail "Health checks did not pass within ${HEALTH_CHECK_TIMEOUT}s"

    # Show health check details
    echo ""
    echo "Health check details:"
    echo "Postgres:"
    docker inspect maproom-postgres --format='{{json .State.Health}}' | jq '.' | sed 's/^/  /'
    echo ""
    echo "MCP:"
    docker inspect maproom-mcp --format='{{json .State.Health}}' | jq '.' | sed 's/^/  /'

    return 1
}

# Function to check logs for errors
check_logs_for_errors() {
    echo ""
    echo -e "${BLUE}Phase 6: Checking Logs for Errors${NC}"
    echo ""

    local containers=("maproom-mcp" "maproom-postgres" "maproom-ollama")
    local found_errors=false

    for container in "${containers[@]}"; do
        log_info "Checking $container logs..."

        # Get recent logs
        local error_count=$(docker logs "$container" 2>&1 | grep -i "error\|fatal\|failed" | grep -v "0 error" | wc -l)

        if [ "$error_count" -gt 0 ]; then
            log_fail "$container has $error_count error messages"
            echo "Recent errors:"
            docker logs "$container" 2>&1 | grep -i "error\|fatal\|failed" | grep -v "0 error" | tail -10 | sed 's/^/  /'
            found_errors=true
        else
            log_pass "$container logs show no critical errors"
        fi
    done

    if [ "$found_errors" = true ]; then
        return 1
    fi

    return 0
}

# Function to verify image source (not locally built)
verify_image_not_locally_built() {
    echo ""
    echo -e "${BLUE}Phase 7: Verifying Image Source${NC}"
    echo ""

    # Check image in maproom-mcp container
    local container_image=$(docker inspect maproom-mcp --format='{{.Config.Image}}' 2>/dev/null)

    if echo "$container_image" | grep -q "crewchief/maproom-mcp"; then
        log_pass "Container using Docker Hub image: $container_image"
    else
        log_fail "Container not using Docker Hub image: $container_image"
        return 1
    fi

    # Verify no local build tags
    if docker images | grep "maproom-mcp" | grep -v "crewchief/maproom-mcp" | grep -q .; then
        log_fail "Found locally built maproom-mcp images (should only have Docker Hub images)"
        docker images | grep "maproom-mcp" | sed 's/^/  /'
        return 1
    else
        log_pass "No locally built images found (only Docker Hub images)"
    fi

    return 0
}

# Function to generate test report
generate_test_report() {
    local test_end_time=$(date +%s)
    local test_duration=$((test_end_time - TEST_START_TIME))

    echo ""
    echo -e "${BLUE}================================================${NC}"
    echo -e "${BLUE}Test Report: DKRHUB-2902${NC}"
    echo -e "${BLUE}================================================${NC}"
    echo ""
    echo "Test Duration: ${test_duration}s"
    echo "Tests Passed: $TESTS_PASSED"
    echo "Tests Failed: $TESTS_FAILED"
    echo "Tests Blocked: $TESTS_BLOCKED"
    echo ""

    if [ $TESTS_BLOCKED -gt 0 ]; then
        echo -e "${YELLOW}Status: BLOCKED${NC}"
        echo "Reason: Docker Hub images not yet published"
        echo "Blocker: DKRHUB-1901 (publish images to Docker Hub)"
        echo ""
        echo "Next Steps:"
        echo "1. Execute DKRHUB-1901 test plan manually"
        echo "2. Verify images published to Docker Hub"
        echo "3. Re-run this test"
        return 2
    elif [ $TESTS_FAILED -gt 0 ]; then
        echo -e "${RED}Status: FAILED${NC}"
        echo ""
        echo "Failed Tests: $TESTS_FAILED"
        echo "Review logs above for details"
        return 1
    else
        echo -e "${GREEN}Status: PASSED${NC}"
        echo ""
        echo "All acceptance criteria met:"
        echo "✓ Clean Docker environment verified"
        echo "✓ Images pulled from Docker Hub"
        echo "✓ All services started successfully"
        echo "✓ Health checks passed"
        echo "✓ No build errors detected"
        echo "✓ Logs show successful startup"
        echo "✓ Images verified from crewchief/maproom-mcp"
        return 0
    fi
}

# Main test execution
main() {
    # Phase 1: Clean environment
    cleanup_docker

    # Phase 2: Check Docker Hub availability
    check_dockerhub_availability
    local availability_status=$?

    if [ $availability_status -eq 2 ]; then
        # Blocked - images not available
        generate_test_report
        exit 2
    elif [ $availability_status -ne 0 ]; then
        # Failed - unexpected error
        generate_test_report
        exit 1
    fi

    # Phase 3: Verify image metadata
    verify_image_metadata || true

    # Phase 4: Test production configuration
    test_production_configuration || {
        cleanup_docker
        generate_test_report
        exit 1
    }

    # Phase 5: Verify containers
    verify_containers_running || {
        cleanup_docker
        generate_test_report
        exit 1
    }

    # Phase 6: Verify health checks
    verify_health_checks || {
        cleanup_docker
        generate_test_report
        exit 1
    }

    # Phase 7: Check logs
    check_logs_for_errors || true

    # Phase 8: Verify image source
    verify_image_not_locally_built || {
        cleanup_docker
        generate_test_report
        exit 1
    }

    # Final cleanup
    cleanup_docker

    # Generate report
    generate_test_report
    local final_status=$?

    exit $final_status
}

# Run main test
main
