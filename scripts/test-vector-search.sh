#!/usr/bin/env bash
# Integration test script for vector-search command
# VECSRCH-3001: Integration Testing
#
# Requirements:
# - PostgreSQL with pgvector running
# - MAPROOM_DATABASE_URL set
# - OPENAI_API_KEY set
# - Indexed repository with embeddings

set -e

echo "=== Vector Search Integration Test ==="
echo ""

# Check environment
if [[ -z "$MAPROOM_DATABASE_URL" ]]; then
    echo "ERROR: MAPROOM_DATABASE_URL not set"
    exit 1
fi

if [[ -z "$OPENAI_API_KEY" ]]; then
    echo "ERROR: OPENAI_API_KEY not set"
    exit 1
fi

# Build the binary
echo "Building maproom binary..."
cargo build --release
MAPROOM_BIN="./target/release/crewchief-maproom"

if [[ ! -f "$MAPROOM_BIN" ]]; then
    echo "ERROR: Binary not found at $MAPROOM_BIN"
    exit 1
fi

echo "✓ Binary built successfully"
echo ""

# Test 1: Help command
echo "Test 1: Verify vector-search help"
if $MAPROOM_BIN vector-search --help | grep -q "vector-search"; then
    echo "✓ Help command works"
else
    echo "✗ Help command failed"
    exit 1
fi
echo ""

# Test 2: Basic vector search (requires test repo)
TEST_REPO="${TEST_REPO:-crewchief}"
echo "Test 2: Basic vector search (repo: $TEST_REPO)"
OUTPUT=$($MAPROOM_BIN vector-search --repo "$TEST_REPO" --query "test function" --k 5 || echo "FAILED")

if [[ "$OUTPUT" == "FAILED" ]]; then
    echo "✗ Vector search failed (may need to create test repo and embeddings)"
    echo "  Run: maproom scan --repo $TEST_REPO && maproom generate-embeddings"
    exit 1
fi

# Verify JSON output
if echo "$OUTPUT" | jq empty 2>/dev/null; then
    echo "✓ Valid JSON output"
else
    echo "✗ Invalid JSON output"
    echo "$OUTPUT"
    exit 1
fi

# Verify schema
if echo "$OUTPUT" | jq -e '.hits' > /dev/null; then
    echo "✓ JSON has 'hits' field"
else
    echo "✗ JSON missing 'hits' field"
    exit 1
fi

if echo "$OUTPUT" | jq -e '.total' > /dev/null; then
    echo "✓ JSON has 'total' field"
else
    echo "✗ JSON missing 'total' field"
    exit 1
fi

if echo "$OUTPUT" | jq -e '.mode' > /dev/null && [[ "$(echo "$OUTPUT" | jq -r '.mode')" == "vector" ]]; then
    echo "✓ JSON has correct mode"
else
    echo "✗ JSON missing or incorrect mode field"
    exit 1
fi
echo ""

# Test 3: Vector search with parameters
echo "Test 3: Vector search with threshold and k"
OUTPUT=$($MAPROOM_BIN vector-search \
    --repo "$TEST_REPO" \
    --query "authentication logic" \
    --k 10 \
    --threshold 0.7 || echo "FAILED")

if [[ "$OUTPUT" == "FAILED" ]]; then
    echo "✗ Vector search with parameters failed"
    exit 1
fi

# Verify parameters in output
if echo "$OUTPUT" | jq -e '.k == 10' > /dev/null; then
    echo "✓ Parameter k=10 in output"
else
    echo "✗ Parameter k not in output"
    exit 1
fi

if echo "$OUTPUT" | jq -e '.threshold == 0.7' > /dev/null; then
    echo "✓ Parameter threshold=0.7 in output"
else
    echo "✗ Parameter threshold not in output"
    exit 1
fi

# Verify all results meet threshold
THRESHOLD_CHECK=$(echo "$OUTPUT" | jq '.hits | all(.[].score >= 0.7)')
if [[ "$THRESHOLD_CHECK" == "true" ]] || [[ "$(echo "$OUTPUT" | jq '.hits | length')" == "0" ]]; then
    echo "✓ All results meet threshold requirement"
else
    echo "✗ Some results below threshold"
    exit 1
fi
echo ""

# Test 4: Verify hit schema
echo "Test 4: Verify hit schema"
FIRST_HIT=$(echo "$OUTPUT" | jq '.hits[0] // empty')

if [[ -n "$FIRST_HIT" ]]; then
    for field in chunk_id score file_path start_line end_line kind symbol_name; do
        if echo "$FIRST_HIT" | jq -e ".$field" > /dev/null 2>&1; then
            echo "✓ Hit has field: $field"
        else
            echo "✗ Hit missing field: $field"
            exit 1
        fi
    done
else
    echo "⚠ No hits returned (database may be empty)"
fi
echo ""

# Test 5: Error handling for missing repo
echo "Test 5: Error handling for nonexistent repo"
if $MAPROOM_BIN vector-search --repo "nonexistent-repo-xyz" --query "test" 2>&1 | grep -q "not found"; then
    echo "✓ Proper error for missing repo"
else
    echo "✗ No error for missing repo"
    exit 1
fi
echo ""

echo "==================================="
echo "✓ All tests passed!"
echo "==================================="
