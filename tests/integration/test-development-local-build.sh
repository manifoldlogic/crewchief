#!/usr/bin/env bash
#
# DKRHUB-2903: Test Development Configuration (Local Build)
#
# This script tests that docker-compose.override.yml allows local builds,
# preserving the development workflow for contributors building from source.
#
# Test Phases:
#   1. Override File Verification
#   2. Compose File Merge Verification
#   3. Local Build Test (from source)
#   4. Image Creation Verification
#   5. Container Startup with Local Image
#   6. Functionality Test
#   7. Production Mode Test (without override)
#   8. Cleanup and Reporting
#
# Exit Codes:
#   0 = All tests passed
#   1 = Tests failed
#   2 = Tests blocked (prerequisite not met)
#

# Note: Using || true for grep commands to prevent script exit on no-match with set -e
set -uo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
WORKSPACE_ROOT="/workspace"
CONFIG_DIR="${WORKSPACE_ROOT}/packages/maproom-mcp/config"
OVERRIDE_FILE="${CONFIG_DIR}/docker-compose.override.yml"
COMPOSE_FILE="${CONFIG_DIR}/docker-compose.yml"
MAX_BUILD_TIME_MINUTES=15
HEALTH_CHECK_TIMEOUT=${HEALTH_CHECK_TIMEOUT:-60}
TEST_START_TIME=$(date +%s)

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_BLOCKED=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $*"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $*"
    ((TESTS_FAILED++))
}

log_blocked() {
    echo -e "${YELLOW}[BLOCKED]${NC} $*"
    ((TESTS_BLOCKED++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_section() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$*${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Cleanup function
cleanup() {
    log_section "Phase 8: Cleanup"

    cd "${CONFIG_DIR}" || true

    # Restore override if it was backed up
    if [ -f "docker-compose.override.yml.bak" ]; then
        log_info "Restoring docker-compose.override.yml from backup..."
        mv docker-compose.override.yml.bak docker-compose.override.yml
    fi

    # Stop and remove all test containers
    log_info "Stopping and removing test containers..."
    docker-compose down -v 2>/dev/null || true

    # Remove locally built images (but not pulled images)
    log_info "Removing locally built test images..."
    docker images | grep -E "config[_-]maproom-mcp|maproom-mcp" | grep -v "crewchief/maproom-mcp" | awk '{print $3}' | xargs -r docker rmi -f 2>/dev/null || true

    local test_end_time=$(date +%s)
    local test_duration=$((test_end_time - TEST_START_TIME))

    log_section "Test Summary"
    echo -e "Tests Passed:  ${GREEN}${TESTS_PASSED}${NC}"
    echo -e "Tests Failed:  ${RED}${TESTS_FAILED}${NC}"
    echo -e "Tests Blocked: ${YELLOW}${TESTS_BLOCKED}${NC}"
    echo -e "Duration:      ${test_duration} seconds"

    if [ ${TESTS_BLOCKED} -gt 0 ]; then
        log_blocked "Tests blocked - prerequisite not met"
        exit 2
    elif [ ${TESTS_FAILED} -gt 0 ]; then
        log_error "Tests failed"
        exit 1
    else
        log_success "All tests passed!"
        exit 0
    fi
}

trap cleanup EXIT INT TERM

# ============================================================================
# Phase 1: Override File Verification
# ============================================================================
log_section "Phase 1: Override File Verification"

cd "${CONFIG_DIR}"

# Test 1.1: Override file exists
log_info "Checking if docker-compose.override.yml exists..."
if [ -f "${OVERRIDE_FILE}" ]; then
    log_success "docker-compose.override.yml exists"
else
    log_blocked "docker-compose.override.yml not found at ${OVERRIDE_FILE}"
    log_blocked "This file is required for development builds"
    exit 2
fi

# Test 1.2: Override file contains build configuration
log_info "Verifying override file contains build configuration..."
if grep -q "build:" "${OVERRIDE_FILE}" 2>/dev/null || [ $? -eq 1 ]; then
    if grep -q "build:" "${OVERRIDE_FILE}" 2>/dev/null; then
        log_success "Override file contains 'build:' directive"
    else
        log_error "Override file does not contain 'build:' directive"
    fi
fi

# Test 1.3: Build context path is correct
log_info "Verifying build context path..."
if grep "context:" "${OVERRIDE_FILE}" 2>/dev/null | grep -q "\.\./\.\./\.\."; then
    log_success "Build context path is correct (../../..)"
else
    log_error "Build context path is incorrect"
fi

# Test 1.4: Dockerfile path is correct
log_info "Verifying Dockerfile path..."
if grep "dockerfile:" "${OVERRIDE_FILE}" 2>/dev/null | grep -q "Dockerfile.mcp-server"; then
    log_success "Dockerfile path is correct"
else
    log_error "Dockerfile path is incorrect"
fi

# ============================================================================
# Phase 2: Compose File Merge Verification
# ============================================================================
log_section "Phase 2: Compose File Merge Verification"

log_info "Testing docker-compose config merge..."
docker-compose config > /tmp/merged-config.yml 2>&1 || {
    log_error "docker-compose config failed"
    cat /tmp/merged-config.yml
    exit 1
}

# Test 2.1: Merged config contains build directive
log_info "Checking if merged config contains build directive..."
if grep -A5 "maproom-mcp:" /tmp/merged-config.yml 2>/dev/null | grep -q "build:"; then
    log_success "Merged config contains build directive (override takes precedence)"
else
    log_error "Merged config does not contain build directive"
fi

# Test 2.2: Base compose file has image directive
log_info "Checking base compose file has image directive..."
if grep -A5 "maproom-mcp:" "${COMPOSE_FILE}" 2>/dev/null | grep -q "image: crewchief/maproom-mcp"; then
    log_success "Base compose file has image directive"
else
    log_error "Base compose file does not have image directive"
fi

# Test 2.3: Merged config shows context path
log_info "Verifying merged config shows build context..."
if grep -A10 "maproom-mcp:" /tmp/merged-config.yml 2>/dev/null | grep -q "context:"; then
    log_success "Merged config shows build context"
else
    log_error "Merged config missing build context"
fi

# ============================================================================
# Phase 3: Local Build Test (from source)
# ============================================================================
log_section "Phase 3: Local Build Test (from source)"

# Clean up any existing containers first
log_info "Cleaning up any existing containers..."
docker-compose down -v 2>/dev/null || true

log_info "Starting local build from source..."
log_warning "This may take up to ${MAX_BUILD_TIME_MINUTES} minutes..."

BUILD_START_TIME=$(date +%s)

# Build with verbose output
if docker-compose build maproom-mcp 2>&1 | tee /tmp/build-output.log; then
    BUILD_END_TIME=$(date +%s)
    BUILD_DURATION=$((BUILD_END_TIME - BUILD_START_TIME))
    BUILD_MINUTES=$((BUILD_DURATION / 60))
    BUILD_SECONDS=$((BUILD_DURATION % 60))

    log_success "Local build completed in ${BUILD_MINUTES}m ${BUILD_SECONDS}s"

    # Test 3.1: Build time is reasonable
    if [ ${BUILD_MINUTES} -lt ${MAX_BUILD_TIME_MINUTES} ]; then
        log_success "Build time is reasonable (< ${MAX_BUILD_TIME_MINUTES} minutes)"
    else
        log_warning "Build time exceeded ${MAX_BUILD_TIME_MINUTES} minutes"
    fi

    # Test 3.2: Check build output for key stages
    log_info "Verifying build stages..."
    if grep -i "builder" /tmp/build-output.log >/dev/null 2>&1; then
        log_success "Multi-stage build detected (builder stage)"
    fi

    if grep -E "pnpm install|npm install" /tmp/build-output.log >/dev/null 2>&1; then
        log_success "Dependencies installation stage present"
    fi

    if grep -E "pnpm build|npm run build" /tmp/build-output.log >/dev/null 2>&1; then
        log_success "Build stage present"
    fi
else
    log_error "Local build failed"
    log_error "Build output:"
    cat /tmp/build-output.log
    exit 1
fi

# ============================================================================
# Phase 4: Image Creation Verification
# ============================================================================
log_section "Phase 4: Image Creation Verification"

log_info "Checking for locally built image..."
docker images | tee /tmp/docker-images.log

# Test 4.1: Local image exists
log_info "Verifying local image was created..."
if docker images | grep -E "config[_-]maproom-mcp" >/dev/null 2>&1; then
    log_success "Locally built image found"

    # Get image details
    LOCAL_IMAGE=$(docker images | grep -E "config[_-]maproom-mcp" | head -1 | awk '{print $1":"$2}')
    IMAGE_ID=$(docker images | grep -E "config[_-]maproom-mcp" | head -1 | awk '{print $3}')
    IMAGE_SIZE=$(docker images | grep -E "config[_-]maproom-mcp" | head -1 | awk '{print $7$8}')

    log_info "Local image: ${LOCAL_IMAGE}"
    log_info "Image ID: ${IMAGE_ID}"
    log_info "Image size: ${IMAGE_SIZE}"
else
    log_error "Locally built image not found"
fi

# Test 4.2: Image is NOT from Docker Hub
log_info "Verifying image is not from Docker Hub..."
if ! docker images | grep "crewchief/maproom-mcp" >/dev/null 2>&1; then
    log_success "No Docker Hub images present (using local build)"
else
    log_warning "Docker Hub image present (may conflict with local build)"
fi

# ============================================================================
# Phase 5: Container Startup with Local Image
# ============================================================================
log_section "Phase 5: Container Startup with Local Image"

log_info "Starting services with locally built image..."
if docker-compose up -d 2>&1 | tee /tmp/compose-up.log; then
    log_success "docker-compose up succeeded"
else
    log_error "docker-compose up failed"
    cat /tmp/compose-up.log
    exit 1
fi

# Test 5.1: No pull attempts detected
log_info "Verifying no Docker Hub pulls occurred..."
if ! grep -i "pull" /tmp/compose-up.log >/dev/null 2>&1; then
    log_success "No Docker Hub pulls detected (using local image)"
else
    log_warning "Pull messages detected in output"
fi

# Test 5.2: All containers started
log_info "Waiting for containers to start..."
sleep 5

docker ps -a | tee /tmp/docker-ps.log

EXPECTED_CONTAINERS=3
RUNNING_CONTAINERS=$(docker ps | grep -c "maproom-" 2>/dev/null || echo "0")

if [ "${RUNNING_CONTAINERS}" -ge "${EXPECTED_CONTAINERS}" ]; then
    log_success "All ${EXPECTED_CONTAINERS} containers are running"
else
    log_error "Expected ${EXPECTED_CONTAINERS} containers, found ${RUNNING_CONTAINERS}"
fi

# Test 5.3: MCP container using local image
log_info "Verifying MCP container is using local image..."
CONTAINER_IMAGE=$(docker inspect maproom-mcp --format='{{.Config.Image}}' 2>/dev/null || echo "")
log_info "Container image: ${CONTAINER_IMAGE}"

if echo "${CONTAINER_IMAGE}" | grep -E "config[_-]maproom-mcp" >/dev/null 2>&1; then
    if ! echo "${CONTAINER_IMAGE}" | grep "crewchief/maproom-mcp" >/dev/null 2>&1; then
        log_success "Container using local image (not Docker Hub)"
    else
        log_error "Container using Docker Hub image instead of local build"
    fi
else
    log_error "Container image does not match expected pattern"
fi

# ============================================================================
# Phase 6: Functionality Test
# ============================================================================
log_section "Phase 6: Functionality Test"

log_info "Waiting for health checks (max ${HEALTH_CHECK_TIMEOUT}s)..."
HEALTH_CHECK_START=$(date +%s)
HEALTH_CHECKS_PASSED=false

while [ $(($(date +%s) - HEALTH_CHECK_START)) -lt ${HEALTH_CHECK_TIMEOUT} ]; do
    HEALTHY_CONTAINERS=$(docker ps | grep "maproom-" | grep -c "(healthy)" 2>/dev/null || echo "0")

    if [ "${HEALTHY_CONTAINERS}" -ge 2 ]; then
        log_success "Health checks passed (${HEALTHY_CONTAINERS} healthy containers)"
        HEALTH_CHECKS_PASSED=true
        break
    fi

    sleep 5
done

if [ "${HEALTH_CHECKS_PASSED}" = false ]; then
    log_warning "Health checks did not pass within ${HEALTH_CHECK_TIMEOUT}s"
    docker ps -a
fi

# Test 6.1: Check container logs for errors
log_info "Checking MCP container logs for errors..."
docker logs maproom-mcp 2>&1 | tee /tmp/mcp-logs.log

if grep -iE "error|fatal|exception" /tmp/mcp-logs.log 2>/dev/null | grep -v "Test" >/dev/null 2>&1; then
    log_warning "Errors found in MCP logs (see above)"
else
    log_success "No critical errors in MCP logs"
fi

# Test 6.2: Verify Node.js is present
log_info "Verifying Node.js is present in container..."
if docker exec maproom-mcp node --version >/dev/null 2>&1; then
    log_success "Node.js is available in container"
else
    log_error "Node.js not found in container"
fi

# Test 6.3: Verify PostgreSQL connectivity
log_info "Verifying PostgreSQL connectivity..."
if docker exec maproom-postgres pg_isready -U maproom -d maproom >/dev/null 2>&1; then
    log_success "PostgreSQL is ready"
else
    log_error "PostgreSQL is not ready"
fi

# ============================================================================
# Phase 7: Production Mode Test (without override)
# ============================================================================
log_section "Phase 7: Production Mode Test (without override)"

log_info "Stopping development containers..."
docker-compose down -v

log_info "Backing up override file..."
cp "${OVERRIDE_FILE}" "${OVERRIDE_FILE}.bak"

log_info "Removing override to test production mode..."
rm "${OVERRIDE_FILE}"

log_info "Verifying docker-compose config without override..."
if docker-compose config > /tmp/prod-config.yml 2>&1; then
    log_success "docker-compose config works without override"
else
    log_error "docker-compose config failed without override"
    cat /tmp/prod-config.yml
fi

# Test 7.1: Production config uses image directive
log_info "Checking production config uses image directive..."
if grep -A5 "maproom-mcp:" /tmp/prod-config.yml 2>/dev/null | grep -q "image: crewchief/maproom-mcp"; then
    log_success "Production config uses Docker Hub image"
else
    log_error "Production config does not use Docker Hub image"
fi

# Test 7.2: Production config does NOT have build directive
log_info "Checking production config does not have build directive..."
if ! grep -A10 "maproom-mcp:" /tmp/prod-config.yml 2>/dev/null | grep -q "build:"; then
    log_success "Production config has no build directive (pull-only)"
else
    log_error "Production config has build directive (should use image only)"
fi

# Test 7.3: Attempt to pull would occur (don't actually pull)
log_info "Verifying production mode would attempt pull..."
log_info "(Not actually pulling to avoid Docker Hub dependency)"
if docker-compose config 2>/dev/null | grep -q "image: crewchief/maproom-mcp"; then
    log_success "Production mode configured to pull from Docker Hub"
else
    log_error "Production mode not configured correctly"
fi

log_info "Restoring override file for development..."
mv "${OVERRIDE_FILE}.bak" "${OVERRIDE_FILE}"

# ============================================================================
# Test Complete
# ============================================================================
log_section "DKRHUB-2903: Test Complete"

# Generate summary report
TEST_END_TIME=$(date +%s)
TOTAL_DURATION=$((TEST_END_TIME - TEST_START_TIME))
TOTAL_MINUTES=$((TOTAL_DURATION / 60))
TOTAL_SECONDS=$((TOTAL_DURATION % 60))

log_info "Total test duration: ${TOTAL_MINUTES}m ${TOTAL_SECONDS}s"
log_info "Build duration: ${BUILD_MINUTES}m ${BUILD_SECONDS}s"

# Show final statistics
log_section "Test Statistics"
echo "Tests Passed:  ${TESTS_PASSED}"
echo "Tests Failed:  ${TESTS_FAILED}"
echo "Tests Blocked: ${TESTS_BLOCKED}"
echo ""
echo "Build Time: ${BUILD_MINUTES}m ${BUILD_SECONDS}s"
echo "Total Time: ${TOTAL_MINUTES}m ${TOTAL_SECONDS}s"

# Exit is handled by trap
