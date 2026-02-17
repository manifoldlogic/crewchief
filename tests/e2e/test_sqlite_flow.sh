#!/bin/sh
# E2E Integration Tests for SQLite Backend
# Tests CLI commands work correctly with the SQLite VectorStore backend
# Note: Don't use set -e as we handle errors within test functions

echo "=== MAPCLI SQLite E2E Tests ==="
echo ""

# Configuration
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
FIXTURE_DB="$REPO_ROOT/crates/maproom/tests/fixtures/pre-indexed-maproom.db"
TEST_DB="/tmp/mapcli-test-$$.db"
PASSED=0
FAILED=0

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up..."
    rm -f "$TEST_DB"
}
trap cleanup EXIT

# Test helper function
run_test() {
    test_name="$1"
    test_cmd="$2"
    expect_success="${3:-true}"

    printf "Test: %s... " "$test_name"

    if eval "$test_cmd" > /tmp/test_output_$$.txt 2>&1; then
        if [ "$expect_success" = "true" ]; then
            echo "PASS"
            PASSED=$((PASSED + 1))
            return 0
        else
            echo "FAIL (expected failure but got success)"
            cat /tmp/test_output_$$.txt
            FAILED=$((FAILED + 1))
            return 1
        fi
    else
        if [ "$expect_success" = "false" ]; then
            echo "PASS (expected failure)"
            PASSED=$((PASSED + 1))
            return 0
        else
            echo "FAIL"
            cat /tmp/test_output_$$.txt
            FAILED=$((FAILED + 1))
            return 1
        fi
    fi
}

# Check prerequisites
if [ ! -f "$FIXTURE_DB" ]; then
    echo "ERROR: Test fixture not found at $FIXTURE_DB"
    echo "Run: cargo test --test create_sqlite_fixture -- --ignored --nocapture"
    exit 1
fi

# Copy fixture to test location
echo "Setting up test database..."
cp "$FIXTURE_DB" "$TEST_DB"
export MAPROOM_DATABASE_URL="sqlite://$TEST_DB"

# Build with SQLite feature
echo "Building CLI with SQLite support..."
cd "$REPO_ROOT" || exit 1
cargo build --bin crewchief-maproom --release 2>/dev/null
CLI="./target/release/crewchief-maproom"

if [ ! -f "$CLI" ]; then
    echo "ERROR: CLI binary not found at $CLI"
    exit 1
fi

echo ""
echo "=== Running CLI Tests ==="
echo ""

# Test 1: Status command (JSON)
run_test "Status command (JSON)" \
    "$CLI status --json 2>/dev/null | jq -e '.repos | length > 0'"

# Test 2: Status command (text)
run_test "Status command (text)" \
    "$CLI status 2>/dev/null | grep -q 'test-repo'"

# Test 3: Search command - find 'main' function
run_test "Search command - find main" \
    "$CLI search --repo test-repo --query 'main' 2>/dev/null | jq -e '.hits | length > 0'"

# Test 4: Search command - find 'function' in content
run_test "Search command - find function" \
    "$CLI search --repo test-repo --query 'function' 2>/dev/null | jq -e '.hits | length > 0'"

# Test 5: Search command - verify result structure
run_test "Search result structure" \
    "$CLI search --repo test-repo --query 'helper' 2>/dev/null | jq -e '.hits[0] | has(\"score\", \"start_line\", \"end_line\")'"

# Test 6: Search command - nonexistent repo returns error
run_test "Search nonexistent repo (expects error)" \
    "$CLI search --repo nonexistent --query 'test' 2>/dev/null" \
    "false"

# Test 7: Cleanup-stale command (default is dry-run)
run_test "Cleanup-stale command" \
    "$CLI db cleanup-stale 2>/dev/null"

# Test 8: DB migrate for SQLite (should be no-op or informative)
run_test "DB migrate for SQLite" \
    "$CLI db migrate 2>/dev/null"

# Summary
echo ""
echo "=== Test Summary ==="
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo ""

if [ $FAILED -gt 0 ]; then
    echo "SOME TESTS FAILED"
    exit 1
else
    echo "ALL TESTS PASSED"
    exit 0
fi
