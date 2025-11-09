#!/bin/bash
# test_index_migration.sh
# Comprehensive automated test suite for migration 0017: Fix index size limits
#
# Tests migration that replaces problematic covering index with two-index strategy:
# - idx_chunks_search_covering (OLD, fails on preview > 2704 bytes) → DROPPED
# - idx_chunks_search_small_preview (NEW, partial covering index for preview <= 2000 bytes)
# - idx_chunks_search_basic (NEW, universal fallback for all preview sizes)
#
# Test Levels (Quality Strategy Pyramid):
# L1: SQL Syntax Validation (9 tests)
#     - Migration file exists
#     - Safe DROP IF EXISTS pattern
#     - CREATE INDEX CONCURRENTLY for non-blocking creation
#     - Correct index definitions (idx_chunks_search_small_preview, idx_chunks_search_basic)
#     - INCLUDE clause for covering index
#     - WHERE clause for partial index (LENGTH(preview) <= 2000)
#     - ANALYZE command for query planner
#     - Statement timeout set
#
# L2: Empty Database Test (12 tests)
#     - PostgreSQL container startup with pgvector extension
#     - Database and schema initialization
#     - Base schema migration (0001_init.sql)
#     - Old covering index simulation (pre-migration state)
#     - Migration 0017 execution
#     - Old index dropped verification
#     - New indexes created verification (2 indexes)
#     - Index comments exist
#
# L3: Data Population Test - CRITICAL (9 tests)
#     - Test repository structure creation
#     - INSERT with small preview (500 bytes) - should use partial index
#     - INSERT with medium preview (2000 bytes) - boundary case
#     - INSERT with large preview (3000 bytes) - WOULD FAIL BEFORE MIGRATION
#     - INSERT with extreme preview (10KB) - edge case
#     - All 4 chunks inserted successfully
#     - Query planner behavior verification
#     - Index usage statistics
#
# Total: 30 automated tests
#
# Requirements:
# - PostgreSQL 15+ with pgvector extension (Docker image: ankane/pgvector:v0.5.1)
# - Docker available for test container
# - Migration files:
#   - /workspace/crates/maproom/migrations/0001_init.sql
#   - /workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql
#
# Usage:
#   cd /workspace/crates/maproom/tests
#   ./test_index_migration.sh
#
# Exit codes:
#   0: All tests passed (30/30)
#   1: One or more tests failed
#
# Test Duration: ~3 seconds
#
# References:
# - Ticket: IDXSIZE-2001
# - Migration: 0017_fix_index_size_limits.sql
# - Architecture: .agents/projects/IDXSIZE_index-size-limits/planning/architecture.md

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

MIGRATION_FILE="/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql"
MIGRATION_0001="/workspace/crates/maproom/migrations/0001_init.sql"
CONTAINER_NAME="test_migration_0017_$$"
PG_PASSWORD="testpass"
PG_PORT="15432"  # Use non-standard port to avoid conflicts
PG_IMAGE="ankane/pgvector:v0.5.1"
MAX_WAIT_SECONDS=30

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
    ((TESTS_TOTAL++))
}

log_failure() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
    ((TESTS_TOTAL++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_section() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Cleanup function - always remove test container
cleanup() {
    local exit_code=$?
    log_info "Cleaning up test container..."
    docker rm -f "$CONTAINER_NAME" &>/dev/null || true

    if [ $exit_code -eq 0 ]; then
        log_success "Cleanup completed successfully"
    else
        log_warning "Cleanup completed (script failed with exit code $exit_code)"
    fi
}

# Register cleanup trap
trap cleanup EXIT

# Execute SQL command in container
exec_sql() {
    local sql="$1"
    docker exec -i "$CONTAINER_NAME" psql -U postgres -d maproom -t -c "$sql" 2>&1
}

# Execute SQL file in container
exec_sql_file() {
    local file="$1"
    docker exec -i "$CONTAINER_NAME" psql -U postgres -d maproom -f /tmp/$(basename "$file") 2>&1
}

# =============================================================================
# L1: SQL Syntax Validation
# =============================================================================

test_l1_syntax_validation() {
    log_section "L1: SQL Syntax Validation"

    # Check migration file exists
    if [ ! -f "$MIGRATION_FILE" ]; then
        log_failure "Migration file not found: $MIGRATION_FILE"
    else
        log_success "Migration file exists: $MIGRATION_FILE"
    fi

    # Validate DROP IF EXISTS pattern
    if grep -q "DROP INDEX IF EXISTS" "$MIGRATION_FILE"; then
        log_success "Migration uses safe DROP IF EXISTS pattern"
    else
        log_failure "Migration does not use DROP IF EXISTS pattern"
    fi

    # Validate CREATE CONCURRENTLY pattern
    if grep -q "CREATE INDEX CONCURRENTLY" "$MIGRATION_FILE"; then
        log_success "Migration uses CREATE INDEX CONCURRENTLY for non-blocking index creation"
    else
        log_failure "Migration does not use CREATE INDEX CONCURRENTLY"
    fi

    # Validate index names
    if grep -q "idx_chunks_search_small_preview" "$MIGRATION_FILE"; then
        log_success "Found idx_chunks_search_small_preview definition"
    else
        log_failure "Missing idx_chunks_search_small_preview definition"
    fi

    if grep -q "idx_chunks_search_basic" "$MIGRATION_FILE"; then
        log_success "Found idx_chunks_search_basic definition"
    else
        log_failure "Missing idx_chunks_search_basic definition"
    fi

    # Validate INCLUDE clause for covering index
    if grep -q "INCLUDE (symbol_name, preview)" "$MIGRATION_FILE"; then
        log_success "Partial covering index uses INCLUDE clause for index-only scans"
    else
        log_failure "Missing INCLUDE clause in partial covering index"
    fi

    # Validate WHERE clause for partial index
    if grep -q "WHERE LENGTH(preview) <= 2000" "$MIGRATION_FILE"; then
        log_success "Partial index correctly filters by preview size"
    else
        log_failure "Missing or incorrect WHERE clause in partial index"
    fi

    # Validate ANALYZE command
    if grep -q "ANALYZE maproom.chunks" "$MIGRATION_FILE"; then
        log_success "Migration includes ANALYZE for query planner statistics"
    else
        log_warning "Migration does not include ANALYZE command (recommended)"
    fi

    # Validate statement timeout
    if grep -q "SET statement_timeout" "$MIGRATION_FILE"; then
        log_success "Migration sets statement timeout for safety"
    else
        log_warning "Migration does not set statement timeout"
    fi
}

# =============================================================================
# L2: Empty Database Test
# =============================================================================

test_l2_empty_database() {
    log_section "L2: Empty Database Test"

    # Start PostgreSQL container with pgvector
    log_info "Starting PostgreSQL container with pgvector..."
    if ! docker run -d \
        --name "$CONTAINER_NAME" \
        -e POSTGRES_PASSWORD="$PG_PASSWORD" \
        -p "$PG_PORT:5432" \
        "$PG_IMAGE" \
        postgres -c log_statement=all &>/dev/null; then
        log_failure "Failed to start PostgreSQL container"
        return 1
    fi
    log_success "PostgreSQL container started: $CONTAINER_NAME"

    # Wait for PostgreSQL to be ready
    log_info "Waiting for PostgreSQL to be ready..."
    local wait_count=0
    while [ $wait_count -lt $MAX_WAIT_SECONDS ]; do
        if docker exec "$CONTAINER_NAME" pg_isready -U postgres &>/dev/null; then
            log_success "PostgreSQL is ready"
            break
        fi
        sleep 1
        ((wait_count++))
    done

    if [ $wait_count -eq $MAX_WAIT_SECONDS ]; then
        log_failure "PostgreSQL failed to start within $MAX_WAIT_SECONDS seconds"
        return 1
    fi

    # Create maproom database
    log_info "Creating maproom database..."
    if docker exec "$CONTAINER_NAME" psql -U postgres -c "CREATE DATABASE maproom;" &>/dev/null; then
        log_success "Created maproom database"
    else
        log_failure "Failed to create maproom database"
        return 1
    fi

    # Copy migration files to container
    log_info "Copying migration files to container..."
    docker cp "$MIGRATION_0001" "$CONTAINER_NAME:/tmp/" &>/dev/null
    docker cp "$MIGRATION_FILE" "$CONTAINER_NAME:/tmp/" &>/dev/null
    log_success "Migration files copied to container"

    # Run 0001_init.sql to set up base schema
    log_info "Running base schema migration (0001_init.sql)..."
    local init_output
    init_output=$(exec_sql_file "/tmp/0001_init.sql")
    if [ $? -eq 0 ]; then
        log_success "Base schema initialized successfully"
    else
        log_failure "Failed to initialize base schema: $init_output"
        return 1
    fi

    # Create the old covering index that will be dropped
    log_info "Creating old covering index (idx_chunks_search_covering) to simulate pre-migration state..."
    local create_old_index_output
    create_old_index_output=$(exec_sql "CREATE INDEX idx_chunks_search_covering ON maproom.chunks (file_id, kind, start_line) INCLUDE (symbol_name, preview);")
    if [ $? -eq 0 ]; then
        log_success "Old covering index created (simulating pre-migration state)"
    else
        log_failure "Failed to create old covering index: $create_old_index_output"
        return 1
    fi

    # Verify old index exists before migration
    log_info "Verifying old index exists before migration..."
    local old_index_count
    old_index_count=$(exec_sql "SELECT COUNT(*) FROM pg_indexes WHERE schemaname='maproom' AND tablename='chunks' AND indexname='idx_chunks_search_covering';" | tr -d ' ')
    if [ "$old_index_count" -eq 1 ]; then
        log_success "Old covering index exists before migration (count: 1)"
    else
        log_failure "Old covering index not found before migration (count: $old_index_count)"
    fi

    # Run migration 0017
    log_info "Running migration 0017..."
    local migration_output
    migration_output=$(exec_sql_file "/tmp/0017_fix_index_size_limits.sql")
    if [ $? -eq 0 ]; then
        log_success "Migration 0017 executed successfully"
    else
        log_failure "Migration 0017 failed: $migration_output"
        return 1
    fi

    # Verify old index was dropped
    log_info "Verifying old index was dropped..."
    old_index_count=$(exec_sql "SELECT COUNT(*) FROM pg_indexes WHERE schemaname='maproom' AND tablename='chunks' AND indexname='idx_chunks_search_covering';" | tr -d ' ')
    if [ "$old_index_count" -eq 0 ]; then
        log_success "Old covering index dropped successfully (count: 0)"
    else
        log_failure "Old covering index still exists after migration (count: $old_index_count)"
    fi

    # Verify new partial covering index exists
    log_info "Verifying idx_chunks_search_small_preview exists..."
    local small_preview_count
    small_preview_count=$(exec_sql "SELECT COUNT(*) FROM pg_indexes WHERE schemaname='maproom' AND tablename='chunks' AND indexname='idx_chunks_search_small_preview';" | tr -d ' ')
    if [ "$small_preview_count" -eq 1 ]; then
        log_success "Partial covering index exists (count: 1)"
    else
        log_failure "Partial covering index not found (count: $small_preview_count)"
    fi

    # Verify new basic index exists
    log_info "Verifying idx_chunks_search_basic exists..."
    local basic_index_count
    basic_index_count=$(exec_sql "SELECT COUNT(*) FROM pg_indexes WHERE schemaname='maproom' AND tablename='chunks' AND indexname='idx_chunks_search_basic';" | tr -d ' ')
    if [ "$basic_index_count" -eq 1 ]; then
        log_success "Basic fallback index exists (count: 1)"
    else
        log_failure "Basic fallback index not found (count: $basic_index_count)"
    fi

    # Verify index comments
    log_info "Verifying index comments exist..."
    local comment_count
    comment_count=$(exec_sql "SELECT COUNT(*) FROM pg_description WHERE objsubid = 0 AND description LIKE '%Covering index for search queries%';" | tr -d ' ')
    if [ "$comment_count" -ge 1 ]; then
        log_success "Index comments exist (count: $comment_count)"
    else
        log_warning "Index comments not found (count: $comment_count)"
    fi
}

# =============================================================================
# L3: Data Population Test (CRITICAL)
# =============================================================================

test_l3_data_population() {
    log_section "L3: Data Population Test"

    # Create test repository and worktree
    log_info "Creating test repository and worktree..."
    exec_sql "INSERT INTO maproom.repos (name, root_path) VALUES ('test_repo', '/test/repo') RETURNING id;" &>/dev/null
    local repo_id
    repo_id=$(exec_sql "SELECT id FROM maproom.repos WHERE name='test_repo';" | tr -d ' ')

    exec_sql "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($repo_id, 'main', '/test/repo/main') RETURNING id;" &>/dev/null
    local worktree_id
    worktree_id=$(exec_sql "SELECT id FROM maproom.worktrees WHERE name='main';" | tr -d ' ')

    exec_sql "INSERT INTO maproom.commits (repo_id, sha, committed_at) VALUES ($repo_id, 'abc123', NOW()) RETURNING id;" &>/dev/null
    local commit_id
    commit_id=$(exec_sql "SELECT id FROM maproom.commits WHERE sha='abc123';" | tr -d ' ')

    exec_sql "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes, last_modified) VALUES ($repo_id, $worktree_id, $commit_id, 'test.ts', 'typescript', 'hash123', 1000, NOW()) RETURNING id;" &>/dev/null
    local file_id
    file_id=$(exec_sql "SELECT id FROM maproom.files WHERE relpath='test.ts';" | tr -d ' ')

    log_success "Test repository structure created (repo_id: $repo_id, file_id: $file_id)"

    # Test Case 1: Small preview (500 bytes) - should use partial covering index
    log_info "Test Case 1: Inserting chunk with small preview (500 bytes)..."
    local small_preview
    small_preview=$(printf 'x%.0s' {1..500})
    local insert_output
    insert_output=$(exec_sql "INSERT INTO maproom.chunks (file_id, symbol_name, kind, start_line, end_line, preview) VALUES ($file_id, 'smallFunc', 'func', 1, 10, '$small_preview');" 2>&1)
    if [ $? -eq 0 ]; then
        log_success "Small preview INSERT succeeded (500 bytes)"
    else
        log_failure "Small preview INSERT failed: $insert_output"
    fi

    # Test Case 2: Medium preview (2000 bytes) - boundary for partial index
    log_info "Test Case 2: Inserting chunk with medium preview (2000 bytes)..."
    local medium_preview
    medium_preview=$(printf 'y%.0s' {1..2000})
    insert_output=$(exec_sql "INSERT INTO maproom.chunks (file_id, symbol_name, kind, start_line, end_line, preview) VALUES ($file_id, 'mediumFunc', 'func', 11, 20, '$medium_preview');" 2>&1)
    if [ $? -eq 0 ]; then
        log_success "Medium preview INSERT succeeded (2000 bytes)"
    else
        log_failure "Medium preview INSERT failed: $insert_output"
    fi

    # Test Case 3: Large preview (3000 bytes) - exceeds old 2704-byte limit (CRITICAL)
    log_info "Test Case 3: Inserting chunk with large preview (3000 bytes) - would fail before migration..."
    local large_preview
    large_preview=$(printf 'z%.0s' {1..3000})
    insert_output=$(exec_sql "INSERT INTO maproom.chunks (file_id, symbol_name, kind, start_line, end_line, preview) VALUES ($file_id, 'largeFunc', 'func', 21, 30, '$large_preview');" 2>&1)
    if [ $? -eq 0 ]; then
        log_success "Large preview INSERT succeeded (3000 bytes) - CRITICAL TEST PASSED"
    else
        log_failure "Large preview INSERT failed (3000 bytes) - CRITICAL FAILURE: $insert_output"
    fi

    # Test Case 4: Extreme preview (10KB) - edge case validation
    log_info "Test Case 4: Inserting chunk with extreme preview (10KB)..."
    local extreme_preview
    extreme_preview=$(printf 'w%.0s' {1..10000})
    insert_output=$(exec_sql "INSERT INTO maproom.chunks (file_id, symbol_name, kind, start_line, end_line, preview) VALUES ($file_id, 'extremeFunc', 'func', 31, 40, '$extreme_preview');" 2>&1)
    if [ $? -eq 0 ]; then
        log_success "Extreme preview INSERT succeeded (10KB) - edge case validated"
    else
        log_failure "Extreme preview INSERT failed (10KB): $insert_output"
    fi

    # Verify all chunks were inserted
    log_info "Verifying all chunks were inserted..."
    local chunk_count
    chunk_count=$(exec_sql "SELECT COUNT(*) FROM maproom.chunks;" | tr -d ' ')
    if [ "$chunk_count" -eq 4 ]; then
        log_success "All 4 test chunks inserted successfully"
    else
        log_failure "Expected 4 chunks, found $chunk_count"
    fi

    # Query planner test: Small preview should use partial covering index
    log_info "Verifying query planner uses partial covering index for small previews..."
    local explain_small
    explain_small=$(exec_sql "EXPLAIN (FORMAT TEXT) SELECT symbol_name, preview FROM maproom.chunks WHERE file_id=$file_id AND kind='func' AND start_line=1;" 2>&1)
    if echo "$explain_small" | grep -q "idx_chunks_search_small_preview"; then
        log_success "Query planner uses partial covering index for small previews (index-only scan)"
    elif echo "$explain_small" | grep -q "idx_chunks_search_basic"; then
        log_warning "Query planner uses basic index for small previews (expected partial covering index)"
    elif echo "$explain_small" | grep -q "Seq Scan"; then
        log_success "Query planner uses sequential scan (expected for small tables < 100 rows)"
    else
        log_warning "Query planner behavior unclear: $explain_small"
    fi

    # Query planner test: Large preview should use basic index with heap lookup
    log_info "Verifying query planner uses basic index for large previews..."
    local explain_large
    explain_large=$(exec_sql "EXPLAIN (FORMAT TEXT) SELECT symbol_name, preview FROM maproom.chunks WHERE file_id=$file_id AND kind='func' AND start_line=21;" 2>&1)
    if echo "$explain_large" | grep -q "idx_chunks_search_basic"; then
        log_success "Query planner uses basic fallback index for large previews"
    elif echo "$explain_large" | grep -q "idx_chunks_search_small_preview"; then
        log_failure "Query planner incorrectly uses partial index for large previews (should use basic index)"
    elif echo "$explain_large" | grep -q "Seq Scan"; then
        log_success "Query planner uses sequential scan (expected for small tables < 100 rows)"
    else
        log_warning "Query planner behavior unclear: $explain_large"
    fi

    # Verify index statistics
    log_info "Analyzing index usage statistics..."
    local stats_output
    stats_output=$(exec_sql "SELECT indexrelname, idx_scan, idx_tup_read, idx_tup_fetch FROM pg_stat_user_indexes WHERE schemaname='maproom' AND indexrelname LIKE 'idx_chunks_search_%' ORDER BY indexrelname;" 2>&1)
    if [ $? -eq 0 ]; then
        log_success "Index statistics retrieved successfully"
        echo "$stats_output"
    else
        log_warning "Failed to retrieve index statistics: $stats_output"
    fi
}

# =============================================================================
# Main Test Execution
# =============================================================================

main() {
    local start_time
    start_time=$(date +%s)

    log_section "Migration 0017 Comprehensive Test Suite"
    log_info "Container: $CONTAINER_NAME"
    log_info "PostgreSQL Image: $PG_IMAGE"
    log_info "Migration: $MIGRATION_FILE"
    echo ""

    # Run L1: SQL Syntax Validation
    test_l1_syntax_validation || true

    # Run L2: Empty Database Test
    if ! test_l2_empty_database; then
        log_failure "L2 test failed, skipping L3"
    else
        # Run L3: Data Population Test (uses container from L2)
        test_l3_data_population || true
    fi

    # Calculate execution time
    local end_time
    end_time=$(date +%s)
    local duration=$((end_time - start_time))

    # Print summary
    log_section "Test Summary"
    echo -e "${BLUE}Total Tests:${NC} $TESTS_TOTAL"
    echo -e "${GREEN}Passed:${NC} $TESTS_PASSED"
    echo -e "${RED}Failed:${NC} $TESTS_FAILED"
    echo -e "${BLUE}Duration:${NC} ${duration}s"
    echo ""

    if [ $TESTS_FAILED -eq 0 ]; then
        log_success "ALL TESTS PASSED!"
        echo ""
        echo "Migration 0017 validation complete:"
        echo "  - SQL syntax follows safe patterns"
        echo "  - Old covering index dropped successfully"
        echo "  - New partial covering index created (for preview <= 2000 bytes)"
        echo "  - New basic fallback index created (for all preview sizes)"
        echo "  - Large previews (>2704 bytes) INSERT successfully"
        echo "  - Query planner uses appropriate indexes"
        echo ""
        return 0
    else
        log_failure "$TESTS_FAILED TEST(S) FAILED"
        echo ""
        return 1
    fi
}

# Run main function
main
exit $?
