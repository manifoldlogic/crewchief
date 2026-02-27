#!/usr/bin/env bash
#
# create-test-fixtures.sh - Generate SQL test fixtures from the test corpus
#
# This script indexes the test corpus files using the maproom daemon and exports
# the resulting data as SQL fixtures for deterministic integration testing.
#
# Usage:
#   ./scripts/create-test-fixtures.sh
#
# Output:
#   tests/setup/test-fixtures.sql
#
# Requirements:
#   - PostgreSQL client tools (psql)
#   - Database accessible (see MAPROOM_DATABASE_URL)
#   - Maproom daemon running (for scan command)
#
# Environment Variables:
#   MAPROOM_DATABASE_URL - Database connection (default: auto-detected)
#   SKIP_EMBEDDINGS      - Set to "true" to skip embedding generation

set -euo pipefail

# Script directory for relative paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(dirname "$SCRIPT_DIR")"
WORKSPACE_DIR="$(dirname "$(dirname "$PACKAGE_DIR")")"

# Configuration
CORPUS_DIR="$PACKAGE_DIR/tests/corpus"
OUTPUT_FILE="$PACKAGE_DIR/tests/setup/test-fixtures.sql"

# Fixture versioning
FIXTURE_VERSION="1.0.0"
SCHEMA_COMPATIBILITY="migrations 0000-0020"

# Fixed IDs to avoid conflicts (starting at 1000)
REPO_ID_START=1000
WORKTREE_ID_START=1000
COMMIT_ID_START=1000
FILE_ID_START=1000
CHUNK_ID_START=1000

# Database connection
if [ -n "${MAPROOM_DATABASE_URL:-}" ]; then
  DB_URL="$MAPROOM_DATABASE_URL"
elif [ "${IN_DEVCONTAINER:-}" = "true" ]; then
  DB_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom"
else
  DB_URL="postgresql://maproom:maproom@localhost:5433/maproom"
fi

echo "============================================"
echo "Creating MCP Test Fixtures"
echo "============================================"
echo ""
echo "Corpus:     $CORPUS_DIR"
echo "Output:     $OUTPUT_FILE"
echo "Database:   ${DB_URL%@*}@***"
echo "Version:    $FIXTURE_VERSION"
echo ""

# Pre-flight checks
echo "Step 1: Pre-flight checks..."

if [ ! -d "$CORPUS_DIR" ]; then
  echo "ERROR: Corpus directory not found: $CORPUS_DIR"
  echo "Run TESTENV-1001 first to create the test corpus."
  exit 1
fi

# Count corpus files
CORPUS_FILES=$(find "$CORPUS_DIR" -type f \( -name "*.ts" -o -name "*.py" -o -name "*.rs" -o -name "*.md" \) ! -name "README.md" | wc -l)
echo "  Found $CORPUS_FILES corpus files"

if [ "$CORPUS_FILES" -eq 0 ]; then
  echo "ERROR: No source files found in corpus directory"
  exit 1
fi

# Check database connectivity
if ! psql "$DB_URL" -c "SELECT 1" > /dev/null 2>&1; then
  echo "ERROR: Cannot connect to database"
  echo "Make sure PostgreSQL is running and accessible at: $DB_URL"
  exit 1
fi
echo "  Database connection OK"

# Check if schema exists
if ! psql "$DB_URL" -c "SELECT 1 FROM maproom.repos LIMIT 0" > /dev/null 2>&1; then
  echo "ERROR: Database schema not initialized"
  echo "Run the schema migration first."
  exit 1
fi
echo "  Schema verified"

# Create a unique test repo name with timestamp
TEST_REPO_NAME="test-corpus"
TEST_WORKTREE_NAME="main"
CORPUS_ROOT="$CORPUS_DIR"

echo ""
echo "Step 2: Cleaning up any existing test data..."

# Clean up existing test corpus data (if any)
psql "$DB_URL" -q -c "
  DELETE FROM maproom.chunks WHERE file_id IN (
    SELECT f.id FROM maproom.files f
    JOIN maproom.repos r ON f.repo_id = r.id
    WHERE r.name = '$TEST_REPO_NAME'
  );
  DELETE FROM maproom.files WHERE repo_id IN (
    SELECT id FROM maproom.repos WHERE name = '$TEST_REPO_NAME'
  );
  DELETE FROM maproom.commits WHERE repo_id IN (
    SELECT id FROM maproom.repos WHERE name = '$TEST_REPO_NAME'
  );
  DELETE FROM maproom.worktrees WHERE repo_id IN (
    SELECT id FROM maproom.repos WHERE name = '$TEST_REPO_NAME'
  );
  DELETE FROM maproom.repos WHERE name = '$TEST_REPO_NAME';
" 2>/dev/null || true
echo "  Cleanup complete"

echo ""
echo "Step 3: Indexing corpus files..."

# Check if daemon binary exists
DAEMON_BIN="$WORKSPACE_DIR/packages/cli/bin/linux-x64/maproom"
if [ ! -x "$DAEMON_BIN" ]; then
  # Try darwin-arm64 if running on Mac
  DAEMON_BIN="$WORKSPACE_DIR/packages/cli/bin/darwin-arm64/maproom"
fi
if [ ! -x "$DAEMON_BIN" ]; then
  echo "WARNING: maproom binary not found"
  echo "Attempting to use cargo run..."

  # Use cargo to run the indexer
  cd "$WORKSPACE_DIR/crates/maproom"

  # Create repo and worktree manually
  psql "$DB_URL" -q -c "
    INSERT INTO maproom.repos (id, name, root_path)
    VALUES ($REPO_ID_START, '$TEST_REPO_NAME', '$CORPUS_ROOT')
    ON CONFLICT (name) DO UPDATE SET root_path = EXCLUDED.root_path
    RETURNING id;
  "

  psql "$DB_URL" -q -c "
    INSERT INTO maproom.worktrees (id, repo_id, name, root_path, is_main)
    VALUES ($WORKTREE_ID_START, $REPO_ID_START, '$TEST_WORKTREE_NAME', '$CORPUS_ROOT', true)
    ON CONFLICT (repo_id, name) DO UPDATE SET root_path = EXCLUDED.root_path
    RETURNING id;
  "

  # Create a placeholder commit
  psql "$DB_URL" -q -c "
    INSERT INTO maproom.commits (id, repo_id, sha, message, committed_at)
    VALUES ($COMMIT_ID_START, $REPO_ID_START, 'fixture-commit', 'Test corpus fixture', NOW())
    ON CONFLICT DO NOTHING;
  "

  # Run indexer with scan command
  MAPROOM_DATABASE_URL="$DB_URL" cargo run --bin maproom -- scan "$CORPUS_ROOT" \
    --repo "$TEST_REPO_NAME" \
    --worktree "$TEST_WORKTREE_NAME" \
    ${SKIP_EMBEDDINGS:+--skip-embeddings} 2>&1 | head -50 || true

  cd "$SCRIPT_DIR"
else
  # Use the binary directly
  MAPROOM_DATABASE_URL="$DB_URL" "$DAEMON_BIN" scan "$CORPUS_ROOT" \
    --repo "$TEST_REPO_NAME" \
    --worktree "$TEST_WORKTREE_NAME" \
    ${SKIP_EMBEDDINGS:+--skip-embeddings} 2>&1 | head -50 || true
fi

echo "  Indexing complete"

# Verify data was indexed
INDEXED_CHUNKS=$(psql "$DB_URL" -t -A -c "
  SELECT COUNT(*) FROM maproom.chunks c
  JOIN maproom.files f ON c.file_id = f.id
  JOIN maproom.repos r ON f.repo_id = r.id
  WHERE r.name = '$TEST_REPO_NAME';
")

echo "  Indexed $INDEXED_CHUNKS chunks"

if [ "$INDEXED_CHUNKS" -eq 0 ]; then
  echo "ERROR: No chunks were indexed"
  echo "Check that the maproom daemon/indexer is working correctly"
  exit 1
fi

echo ""
echo "Step 4: Exporting fixture data..."

# Get IDs of our test data
TEMP_FILE_IDS=$(mktemp)
TEMP_CHUNK_IDS=$(mktemp)

psql "$DB_URL" -t -A -c "
  SELECT f.id FROM maproom.files f
  JOIN maproom.repos r ON f.repo_id = r.id
  WHERE r.name = '$TEST_REPO_NAME'
  ORDER BY f.id;
" > "$TEMP_FILE_IDS"

psql "$DB_URL" -t -A -c "
  SELECT c.id FROM maproom.chunks c
  JOIN maproom.files f ON c.file_id = f.id
  JOIN maproom.repos r ON f.repo_id = r.id
  WHERE r.name = '$TEST_REPO_NAME'
  ORDER BY c.id;
" > "$TEMP_CHUNK_IDS"

FILE_COUNT=$(wc -l < "$TEMP_FILE_IDS")
CHUNK_COUNT=$(wc -l < "$TEMP_CHUNK_IDS")

echo "  Files: $FILE_COUNT"
echo "  Chunks: $CHUNK_COUNT"

# Create output directory if needed
mkdir -p "$(dirname "$OUTPUT_FILE")"

# Generate fixture file header
GENERATED_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
cat > "$OUTPUT_FILE" << EOF
-- MCP Test Fixtures
-- Fixture Version: $FIXTURE_VERSION
-- Compatible Schema: $SCHEMA_COMPATIBILITY
-- Generated: $GENERATED_DATE
-- Generator: packages/maproom-mcp/scripts/create-test-fixtures.sh
--
-- Test corpus: packages/maproom-mcp/tests/corpus/
-- Files: $FILE_COUNT
-- Chunks: $CHUNK_COUNT
--
-- This fixture provides pre-indexed test data for deterministic integration tests.
-- It includes TypeScript, Python, Rust, and Markdown samples with known query results.
--
-- Usage:
--   psql \$MAPROOM_DATABASE_URL < tests/setup/test-fixtures.sql
--
-- Query→Result Expectations:
--   See tests/corpus/README.md for the full 12 query→result matrix

BEGIN;

-- Temporarily disable triggers for faster loading
SET session_replication_role = replica;

-- Clean up any existing test data
DELETE FROM maproom.chunks WHERE file_id IN (
  SELECT f.id FROM maproom.files f
  JOIN maproom.repos r ON f.repo_id = r.id
  WHERE r.name = '$TEST_REPO_NAME'
);
DELETE FROM maproom.files WHERE repo_id IN (
  SELECT id FROM maproom.repos WHERE name = '$TEST_REPO_NAME'
);
DELETE FROM maproom.commits WHERE repo_id IN (
  SELECT id FROM maproom.repos WHERE name = '$TEST_REPO_NAME'
);
DELETE FROM maproom.worktrees WHERE repo_id IN (
  SELECT id FROM maproom.repos WHERE name = '$TEST_REPO_NAME'
);
DELETE FROM maproom.repos WHERE name = '$TEST_REPO_NAME';

EOF

# Export repos
echo "  Exporting repos..."
{
  echo "-- Repository"
  echo "\\COPY maproom.repos FROM stdin;"
  psql "$DB_URL" -c "
    COPY (
      SELECT r.*
      FROM maproom.repos r
      WHERE r.name = '$TEST_REPO_NAME'
      ORDER BY r.id
    ) TO STDOUT;
  "
  echo "\\."
  echo ""
} >> "$OUTPUT_FILE"

# Export worktrees
echo "  Exporting worktrees..."
{
  echo "-- Worktrees"
  echo "\\COPY maproom.worktrees FROM stdin;"
  psql "$DB_URL" -c "
    COPY (
      SELECT w.*
      FROM maproom.worktrees w
      JOIN maproom.repos r ON w.repo_id = r.id
      WHERE r.name = '$TEST_REPO_NAME'
      ORDER BY w.id
    ) TO STDOUT;
  "
  echo "\\."
  echo ""
} >> "$OUTPUT_FILE"

# Export commits
echo "  Exporting commits..."
{
  echo "-- Commits"
  echo "\\COPY maproom.commits FROM stdin;"
  psql "$DB_URL" -c "
    COPY (
      SELECT c.*
      FROM maproom.commits c
      JOIN maproom.repos r ON c.repo_id = r.id
      WHERE r.name = '$TEST_REPO_NAME'
      ORDER BY c.id
    ) TO STDOUT;
  "
  echo "\\."
  echo ""
} >> "$OUTPUT_FILE"

# Export files
echo "  Exporting files..."
{
  echo "-- Files"
  echo "\\COPY maproom.files FROM stdin;"
  psql "$DB_URL" -c "
    COPY (
      SELECT f.*
      FROM maproom.files f
      WHERE f.id IN ($(cat "$TEMP_FILE_IDS" | paste -sd,))
      ORDER BY f.id
    ) TO STDOUT;
  "
  echo "\\."
  echo ""
} >> "$OUTPUT_FILE"

# Export chunks
echo "  Exporting chunks..."
{
  echo "-- Chunks"
  echo "\\COPY maproom.chunks FROM stdin;"
  psql "$DB_URL" -c "
    COPY (
      SELECT c.*
      FROM maproom.chunks c
      WHERE c.id IN ($(cat "$TEMP_CHUNK_IDS" | paste -sd,))
      ORDER BY c.id
    ) TO STDOUT;
  "
  echo "\\."
  echo ""
} >> "$OUTPUT_FILE"

# Add footer with sequence updates and verification
cat >> "$OUTPUT_FILE" << 'EOF'
-- Re-enable triggers
SET session_replication_role = DEFAULT;

-- Update sequences to avoid conflicts with future inserts
SELECT setval('maproom.repos_id_seq', GREATEST((SELECT MAX(id) FROM maproom.repos), 1100));
SELECT setval('maproom.worktrees_id_seq', GREATEST((SELECT MAX(id) FROM maproom.worktrees), 1100));
SELECT setval('maproom.commits_id_seq', GREATEST((SELECT MAX(id) FROM maproom.commits), 1100));
SELECT setval('maproom.files_id_seq', GREATEST((SELECT MAX(id) FROM maproom.files), 1100));
SELECT setval('maproom.chunks_id_seq', GREATEST((SELECT MAX(id) FROM maproom.chunks), 1100));

COMMIT;

-- Verification queries
\echo ''
\echo '=== Test Fixture Statistics ==='
\echo ''

SELECT
  f.language,
  COUNT(*) as chunk_count
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
JOIN maproom.repos r ON f.repo_id = r.id
WHERE r.name = 'test-corpus'
GROUP BY f.language
ORDER BY chunk_count DESC;

\echo ''

SELECT
  c.kind::text,
  COUNT(*) as count
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
JOIN maproom.repos r ON f.repo_id = r.id
WHERE r.name = 'test-corpus'
GROUP BY c.kind
ORDER BY count DESC
LIMIT 10;

\echo ''

SELECT
  COUNT(*) as total_chunks,
  COUNT(CASE WHEN code_embedding IS NOT NULL THEN 1 END) as with_code_emb,
  COUNT(CASE WHEN text_embedding IS NOT NULL THEN 1 END) as with_text_emb
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
JOIN maproom.repos r ON f.repo_id = r.id
WHERE r.name = 'test-corpus';

\echo ''
\echo 'Test fixtures loaded successfully!'
EOF

# Cleanup
rm -f "$TEMP_FILE_IDS" "$TEMP_CHUNK_IDS"

# Get file size
FILE_SIZE=$(du -h "$OUTPUT_FILE" | cut -f1)

echo ""
echo "============================================"
echo "Fixture created successfully!"
echo "============================================"
echo ""
echo "Location: $OUTPUT_FILE"
echo "Size:     $FILE_SIZE"
echo ""
echo "Contents:"
echo "  - Repository: $TEST_REPO_NAME"
echo "  - Files: $FILE_COUNT"
echo "  - Chunks: $CHUNK_COUNT"
echo ""
echo "To load fixture:"
echo "  psql \$MAPROOM_DATABASE_URL < $OUTPUT_FILE"
echo ""
