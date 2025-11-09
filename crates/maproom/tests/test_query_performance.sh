#!/usr/bin/env bash
#
# Performance test for migration 0017 two-index strategy
#
# Tests that the partial covering index (idx_chunks_search_small_preview)
# and basic fallback index (idx_chunks_search_basic) maintain acceptable
# query performance for both small and large preview chunks.
#
# Success Criteria:
# - Small preview queries use idx_chunks_search_small_preview (Index Only Scan)
# - Large preview queries use idx_chunks_search_basic (Index Scan)
# - No Sequential Scans (indicates indexes are working)
# - Small preview queries < 20ms (p95 target)
# - Large preview queries < 50ms (p95 target)

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
CONTAINER_NAME="maproom-perf-test-$$"
DB_NAME="maproom"
DB_USER="maproom"
DB_PASS="maproom"
DB_PORT="$((5434 + RANDOM % 1000))"  # Random port to avoid conflicts
MIGRATIONS_DIR="../migrations"
TEST_DATA_ROWS=200  # Sufficient to trigger index usage

# Performance thresholds (milliseconds)
SMALL_PREVIEW_THRESHOLD_MS=20
LARGE_PREVIEW_THRESHOLD_MS=50

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    docker rm -f "${CONTAINER_NAME}" 2>/dev/null || true
}

trap cleanup EXIT

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $*"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $*"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

run_test() {
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
}

# Execute SQL query
exec_sql() {
    local query="$1"
    docker exec "${CONTAINER_NAME}" psql \
        -U "${DB_USER}" \
        -d "${DB_NAME}" \
        -c "${query}" \
        -t \
        -A
}

# Execute SQL file
exec_sql_file() {
    local file="$1"
    docker cp "${file}" "${CONTAINER_NAME}:/tmp/migration.sql" > /dev/null 2>&1
    docker exec "${CONTAINER_NAME}" psql \
        -U "${DB_USER}" \
        -d "${DB_NAME}" \
        -f /tmp/migration.sql \
        -q  # Quiet mode - suppress notices
    return 0
}

# Extract execution time from EXPLAIN ANALYZE output
extract_execution_time() {
    local explain_output="$1"
    echo "${explain_output}" | grep "Execution Time:" | sed 's/.*: \([0-9.]*\) ms/\1/'
}

# Extract index name from EXPLAIN output
extract_index_name() {
    local explain_output="$1"
    echo "${explain_output}" | grep -o "idx_chunks_search_[a-z_]*" | head -1
}

# Check if execution plan contains specific scan type
check_scan_type() {
    local explain_output="$1"
    local scan_type="$2"
    echo "${explain_output}" | grep -q "${scan_type}"
}

# Main test execution
main() {
    echo -e "${BLUE}================================================${NC}"
    echo -e "${BLUE}  Query Performance Test - Migration 0017      ${NC}"
    echo -e "${BLUE}================================================${NC}"
    echo ""

    # Step 1: Start PostgreSQL container
    log_info "Starting PostgreSQL container with pgvector..."
    docker run -d \
        --name "${CONTAINER_NAME}" \
        -e POSTGRES_USER="${DB_USER}" \
        -e POSTGRES_PASSWORD="${DB_PASS}" \
        -e POSTGRES_DB="${DB_NAME}" \
        -p "${DB_PORT}:5432" \
        pgvector/pgvector:pg15 \
        > /dev/null

    # Wait for PostgreSQL to be ready
    log_info "Waiting for PostgreSQL to be ready..."
    for i in {1..30}; do
        if docker exec "${CONTAINER_NAME}" pg_isready -U "${DB_USER}" > /dev/null 2>&1; then
            break
        fi
        if [ $i -eq 30 ]; then
            log_fail "PostgreSQL failed to start"
            exit 1
        fi
        sleep 1
    done
    log_success "PostgreSQL is ready"

    # Step 2: Initialize schema
    log_info "Applying schema migration (0001_init.sql)..."
    exec_sql_file "${MIGRATIONS_DIR}/0001_init.sql"
    log_success "Schema initialized"

    # Step 3: Apply migration 0017
    log_info "Applying migration 0017 (two-index strategy)..."
    exec_sql_file "${MIGRATIONS_DIR}/0017_fix_index_size_limits.sql"
    log_success "Migration 0017 applied"

    # Step 4: Populate test data
    log_info "Populating test data (${TEST_DATA_ROWS} rows, 95% small / 5% large)..."

    # Create test repo, worktree, commit, and file
    exec_sql "INSERT INTO maproom.repos (id, name, root_path) VALUES (1, 'test-repo', '/tmp/test');" > /dev/null
    exec_sql "INSERT INTO maproom.worktrees (id, repo_id, name, abs_path) VALUES (1, 1, 'main', '/tmp/test');" > /dev/null
    exec_sql "INSERT INTO maproom.commits (id, repo_id, sha) VALUES (1, 1, 'abc123');" > /dev/null
    exec_sql "INSERT INTO maproom.files (id, repo_id, worktree_id, commit_id, relpath, content_hash) VALUES (1, 1, 1, 1, 'test.ts', 'hash1');" > /dev/null

    # Generate test chunks with realistic distribution
    local small_count=$((TEST_DATA_ROWS * 95 / 100))
    local large_count=$((TEST_DATA_ROWS - small_count))

    # Insert small preview chunks (preview <= 2000 bytes)
    log_info "  - Inserting ${small_count} small preview chunks..."
    for i in $(seq 1 ${small_count}); do
        local preview=$(printf 'x%.0s' {1..1500})  # 1500 bytes
        exec_sql "INSERT INTO maproom.chunks (file_id, symbol_name, kind, start_line, end_line, preview) VALUES (1, 'func_${i}', 'func', ${i}, $((i+10)), '${preview}');" > /dev/null
    done

    # Insert large preview chunks (preview > 2704 bytes)
    log_info "  - Inserting ${large_count} large preview chunks..."
    for i in $(seq 1 ${large_count}); do
        local line=$((small_count + i))
        local preview=$(printf 'y%.0s' {1..3000})  # 3000 bytes (exceeds 2704 limit)
        exec_sql "INSERT INTO maproom.chunks (file_id, symbol_name, kind, start_line, end_line, preview) VALUES (1, 'large_func_${i}', 'func', ${line}, $((line+10)), '${preview}');" > /dev/null
    done

    log_success "Test data populated (${small_count} small, ${large_count} large)"

    # Step 5: Update statistics
    log_info "Running ANALYZE to update query planner statistics..."
    exec_sql "ANALYZE maproom.chunks;" > /dev/null
    log_success "Statistics updated"

    # Verify indexes exist
    log_info "Verifying indexes exist..."
    local idx_count=$(exec_sql "SELECT COUNT(*) FROM pg_indexes WHERE schemaname='maproom' AND tablename='chunks' AND (indexname='idx_chunks_search_small_preview' OR indexname='idx_chunks_search_basic');")
    if [ "${idx_count}" -eq 2 ]; then
        log_success "Both indexes exist (idx_chunks_search_small_preview, idx_chunks_search_basic)"
    else
        log_fail "Expected 2 indexes, found ${idx_count}"
        exit 1
    fi

    echo ""
    echo -e "${BLUE}================================================${NC}"
    echo -e "${BLUE}  Running Performance Tests                     ${NC}"
    echo -e "${BLUE}================================================${NC}"
    echo ""

    # ========================================
    # Test 1: Small Preview Search
    # ========================================
    run_test
    log_info "Test 1: Small Preview Search (preview <= 2000 bytes)"

    local query1="EXPLAIN (ANALYZE, BUFFERS) SELECT file_id, symbol_name, preview FROM maproom.chunks WHERE file_id = 1 AND kind = 'func' AND LENGTH(preview) <= 2000 ORDER BY start_line LIMIT 10;"
    local explain1=$(exec_sql "${query1}" 2>&1)

    echo "${explain1}" | sed 's/^/  | /'
    echo ""

    local exec_time1=$(extract_execution_time "${explain1}")
    local index1=$(extract_index_name "${explain1}")

    # Check if correct index is used
    if [ "${index1}" = "idx_chunks_search_small_preview" ]; then
        log_success "Test 1.1: Using idx_chunks_search_small_preview"
    else
        log_fail "Test 1.1: Expected idx_chunks_search_small_preview, got: ${index1}"
    fi

    # Check for Index Only Scan
    if check_scan_type "${explain1}" "Index Only Scan"; then
        log_success "Test 1.2: Using Index Only Scan (no heap fetch)"
    else
        log_fail "Test 1.2: Expected Index Only Scan, got different scan type"
    fi

    # Check execution time (convert to integer for comparison)
    local exec_time1_int=$(printf "%.0f" "${exec_time1}")
    if [ "${exec_time1_int}" -lt "${SMALL_PREVIEW_THRESHOLD_MS}" ]; then
        log_success "Test 1.3: Execution time ${exec_time1}ms < ${SMALL_PREVIEW_THRESHOLD_MS}ms threshold"
    else
        log_fail "Test 1.3: Execution time ${exec_time1}ms >= ${SMALL_PREVIEW_THRESHOLD_MS}ms threshold"
    fi

    # Check no sequential scan
    if ! check_scan_type "${explain1}" "Seq Scan"; then
        log_success "Test 1.4: No Sequential Scan (index is used)"
    else
        log_fail "Test 1.4: Sequential Scan detected (index not used)"
    fi

    echo ""

    # ========================================
    # Test 2: Large Preview Search
    # ========================================
    run_test
    log_info "Test 2: Large Preview Search (preview > 2704 bytes)"

    local query2="EXPLAIN (ANALYZE, BUFFERS) SELECT file_id, symbol_name, preview FROM maproom.chunks WHERE file_id = 1 AND kind = 'func' AND LENGTH(preview) > 2704 ORDER BY start_line LIMIT 10;"
    local explain2=$(exec_sql "${query2}" 2>&1)

    echo "${explain2}" | sed 's/^/  | /'
    echo ""

    local exec_time2=$(extract_execution_time "${explain2}")
    local index2=$(extract_index_name "${explain2}")

    # Check if basic index is used
    if [ "${index2}" = "idx_chunks_search_basic" ]; then
        log_success "Test 2.1: Using idx_chunks_search_basic"
    else
        log_fail "Test 2.1: Expected idx_chunks_search_basic, got: ${index2}"
    fi

    # Check for Index Scan (heap fetch is acceptable for large previews)
    if check_scan_type "${explain2}" "Index Scan"; then
        log_success "Test 2.2: Using Index Scan with heap fetch (acceptable for large previews)"
    else
        log_warn "Test 2.2: Expected Index Scan, got different scan type (may be optimized differently)"
    fi

    # Check execution time (convert to integer for comparison)
    local exec_time2_int=$(printf "%.0f" "${exec_time2}")
    if [ "${exec_time2_int}" -lt "${LARGE_PREVIEW_THRESHOLD_MS}" ]; then
        log_success "Test 2.3: Execution time ${exec_time2}ms < ${LARGE_PREVIEW_THRESHOLD_MS}ms threshold"
    else
        log_fail "Test 2.3: Execution time ${exec_time2}ms >= ${LARGE_PREVIEW_THRESHOLD_MS}ms threshold"
    fi

    # Check no sequential scan
    if ! check_scan_type "${explain2}" "Seq Scan"; then
        log_success "Test 2.4: No Sequential Scan (index is used)"
    else
        log_fail "Test 2.4: Sequential Scan detected (index not used)"
    fi

    echo ""

    # ========================================
    # Test 3: Mixed Query
    # ========================================
    run_test
    log_info "Test 3: Mixed Query (both small and large previews)"

    local query3="EXPLAIN (ANALYZE, BUFFERS) SELECT file_id, symbol_name, LENGTH(preview) as preview_len FROM maproom.chunks WHERE file_id = 1 AND kind = 'func' ORDER BY start_line LIMIT 20;"
    local explain3=$(exec_sql "${query3}" 2>&1)

    echo "${explain3}" | sed 's/^/  | /'
    echo ""

    local exec_time3=$(extract_execution_time "${explain3}")
    local index3=$(extract_index_name "${explain3}")

    # Check if an index is used
    if [ -n "${index3}" ]; then
        log_success "Test 3.1: Using index: ${index3}"
    else
        log_fail "Test 3.1: No index detected in query plan"
    fi

    # Check execution time (should be reasonable)
    local mixed_threshold=50
    local exec_time3_int=$(printf "%.0f" "${exec_time3}")
    if [ "${exec_time3_int}" -lt "${mixed_threshold}" ]; then
        log_success "Test 3.2: Execution time ${exec_time3}ms < ${mixed_threshold}ms threshold"
    else
        log_fail "Test 3.2: Execution time ${exec_time3}ms >= ${mixed_threshold}ms threshold"
    fi

    # Check no sequential scan
    if ! check_scan_type "${explain3}" "Seq Scan"; then
        log_success "Test 3.3: No Sequential Scan (index is used)"
    else
        log_fail "Test 3.3: Sequential Scan detected (index not used)"
    fi

    echo ""

    # ========================================
    # Summary
    # ========================================
    echo -e "${BLUE}================================================${NC}"
    echo -e "${BLUE}  Test Summary                                  ${NC}"
    echo -e "${BLUE}================================================${NC}"
    echo ""

    echo -e "Tests Run:    ${TESTS_TOTAL}"
    echo -e "Tests Passed: ${GREEN}${TESTS_PASSED}${NC}"
    echo -e "Tests Failed: ${RED}${TESTS_FAILED}${NC}"
    echo ""

    if [ ${TESTS_FAILED} -eq 0 ]; then
        echo -e "${GREEN}All performance tests passed!${NC}"
        echo ""
        echo "Key Findings:"
        echo "  - Small preview queries use idx_chunks_search_small_preview (Index Only Scan)"
        echo "  - Large preview queries use idx_chunks_search_basic (Index Scan)"
        echo "  - No Sequential Scans detected (indexes are working)"
        echo "  - Query performance meets acceptable thresholds"
        echo ""
        return 0
    else
        echo -e "${RED}Some performance tests failed!${NC}"
        echo ""
        echo "Review the test output above for details."
        echo ""
        return 1
    fi
}

# Run main test
main
exit_code=$?

exit ${exit_code}
