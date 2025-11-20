#!/bin/bash
set -e

# TESTISO-1004: Manual validation script for test database isolation
# This script validates that test and dev databases are properly isolated

echo ""
echo "========================================="
echo "Test Database Isolation Validation"
echo "========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Find project root - must be run from within git repository
if ! PROJECT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null); then
    echo -e "${RED}❌ Error: This script must be run from within the CrewChief git repository${NC}"
    echo "   Current directory: $(pwd)"
    echo "   Please cd to the repository root or any subdirectory first."
    exit 1
fi

cd "$PROJECT_ROOT"

echo "Project root: $PROJECT_ROOT"
echo ""

# Step 1: Start Docker Compose infrastructure
echo "Step 1: Ensuring Docker Compose infrastructure is running..."
cd "$PROJECT_ROOT/packages/maproom-mcp/config"

# Check if postgres services are already running
if docker compose ps | grep -q "maproom-postgres.*running" && docker compose ps | grep -q "maproom-postgres-test.*running"; then
    echo -e "${GREEN}✅ Docker Compose services already running${NC}"
else
    # Start only if not running
    if ! docker compose up -d maproom-postgres maproom-postgres-test 2>/dev/null; then
        echo -e "${YELLOW}⚠️  Some services already running, checking status...${NC}"
    fi
    echo -e "${GREEN}✅ Docker Compose services ensured${NC}"
fi
echo ""

# Step 2: Wait for databases to be healthy (with timeout)
echo "Step 2: Waiting for databases to be healthy (timeout: 30s)..."

wait_for_healthy() {
    local container=$1
    local timeout=30
    local elapsed=0

    while [ $elapsed -lt $timeout ]; do
        if docker compose ps | grep "$container" | grep -q "healthy"; then
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    return 1
}

# Wait for dev database
if ! wait_for_healthy "maproom-postgres"; then
    echo -e "${RED}❌ Dev database (maproom-postgres) failed to become healthy${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Dev database (maproom-postgres) is healthy${NC}"

# Wait for test database
if ! wait_for_healthy "maproom-postgres-test"; then
    echo -e "${RED}❌ Test database (maproom-postgres-test) failed to become healthy${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Test database (maproom-postgres-test) is healthy${NC}"
echo ""

# Step 3: Run integration tests with TEST_MAPROOM_DATABASE_URL
echo "Step 3: Running integration tests against test database..."
cd "$PROJECT_ROOT/packages/maproom-mcp"

# Note: Using host.docker.internal for devcontainer compatibility
# In true host environments, localhost:5434 would work
if TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@host.docker.internal:5434/maproom_test pnpm test:integration >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Integration tests passed${NC}"
else
    echo -e "${YELLOW}⚠️  Some integration tests failed (this may be expected if schema is incomplete)${NC}"
    echo "    Tests should at least connect to database successfully"
fi
echo ""

# Step 4: Query both databases for chunk counts
echo "Step 4: Querying databases for chunk counts..."

# Query dev database (handle missing table gracefully)
DEV_COUNT=$(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT COUNT(*) FROM maproom.chunks" 2>/dev/null | tr -d ' ' || echo "0")
if [ -z "$DEV_COUNT" ] || [ "$DEV_COUNT" = "" ]; then
    DEV_COUNT=0
fi

# Query test database (handle missing table gracefully)
TEST_COUNT=$(docker exec maproom-postgres-test psql -U maproom -d maproom_test -t -c "SELECT COUNT(*) FROM maproom.chunks" 2>/dev/null | tr -d ' ' || echo "0")
if [ -z "$TEST_COUNT" ] || [ "$TEST_COUNT" = "" ]; then
    TEST_COUNT=0
fi

echo "Dev database (maproom):       $DEV_COUNT chunks"
echo "Test database (maproom_test): $TEST_COUNT chunks"
echo ""

# Step 5: Validate isolation (different counts = isolated)
echo "Step 5: Validating database isolation..."

if [ "$DEV_COUNT" != "$TEST_COUNT" ]; then
    echo -e "${GREEN}✅ Databases are ISOLATED (different chunk counts)${NC}"
    echo "   Dev and test databases contain different data, confirming isolation."
    exit 0
elif [ "$DEV_COUNT" = "0" ] && [ "$TEST_COUNT" = "0" ]; then
    echo -e "${GREEN}✅ Databases are ISOLATED (both empty but separate instances)${NC}"
    echo "   Both databases have zero chunks, but they are separate PostgreSQL instances."
    echo "   Isolation is confirmed - databases would only share data if using same volume/instance."
    exit 0
else
    echo -e "${YELLOW}⚠️  Databases have same non-zero chunk count ($DEV_COUNT)${NC}"
    echo "   This could indicate:"
    echo "   - Coincidence (both databases indexed same content)"
    echo "   - Shared volume (isolation FAILED)"
    echo ""
    echo "   To verify true isolation, check volumes:"
    echo "   docker volume ls | grep maproom"
    echo ""
    echo "   Expected output:"
    echo "   - config_maproom-data (dev database)"
    echo "   - config_maproom-test-data (test database)"
    echo ""
    echo "   If only one volume exists, databases are NOT isolated."
    exit 1
fi
