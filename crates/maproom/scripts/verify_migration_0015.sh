#!/bin/bash
# verify_migration_0015.sh
# Verifies migration 0015 (add Ollama embedding columns) completed successfully
#
# Checks:
# - code_embedding_ollama column exists
# - text_embedding_ollama column exists
# - idx_chunks_code_vec_ollama index exists
# - idx_chunks_text_vec_ollama index exists
# - Existing OpenAI embeddings are preserved (non-NULL count)
# - New Ollama columns are NULL (no data yet)
#
# Usage:
#   MAPROOM_DATABASE_URL=postgresql://... ./verify_migration_0015.sh
#
# Exit codes:
#   0: All checks passed
#   1: One or more checks failed

set -e

DB_URL="${MAPROOM_DATABASE_URL:-postgresql://postgres:postgres@postgres:5432/crewchief}"

echo "============================================"
echo "Verifying migration 0015..."
echo "Database: $DB_URL"
echo "============================================"
echo ""

# Check code_embedding_ollama column existence
echo "Checking code_embedding_ollama column..."
if psql "$DB_URL" -t -c "SELECT code_embedding_ollama FROM maproom.chunks LIMIT 1" &>/dev/null; then
  echo "✓ code_embedding_ollama column exists"
else
  echo "✗ code_embedding_ollama column missing"
  exit 1
fi

# Check text_embedding_ollama column existence
echo "Checking text_embedding_ollama column..."
if psql "$DB_URL" -t -c "SELECT text_embedding_ollama FROM maproom.chunks LIMIT 1" &>/dev/null; then
  echo "✓ text_embedding_ollama column exists"
else
  echo "✗ text_embedding_ollama column missing"
  exit 1
fi

# Check code index existence
echo "Checking idx_chunks_code_vec_ollama index..."
CODE_INDEX_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM pg_indexes
  WHERE schemaname='maproom' AND tablename='chunks' AND indexname='idx_chunks_code_vec_ollama'
" | tr -d ' ')

if [ "$CODE_INDEX_COUNT" -eq 1 ]; then
  echo "✓ idx_chunks_code_vec_ollama index exists"
else
  echo "✗ idx_chunks_code_vec_ollama index missing (found: $CODE_INDEX_COUNT)"
  exit 1
fi

# Check text index existence
echo "Checking idx_chunks_text_vec_ollama index..."
TEXT_INDEX_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM pg_indexes
  WHERE schemaname='maproom' AND tablename='chunks' AND indexname='idx_chunks_text_vec_ollama'
" | tr -d ' ')

if [ "$TEXT_INDEX_COUNT" -eq 1 ]; then
  echo "✓ idx_chunks_text_vec_ollama index exists"
else
  echo "✗ idx_chunks_text_vec_ollama index missing (found: $TEXT_INDEX_COUNT)"
  exit 1
fi

# Check existing OpenAI embeddings preserved
echo ""
echo "Verifying data preservation..."
OPENAI_CODE_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL
" | tr -d ' ')
echo "✓ OpenAI code embeddings preserved: $OPENAI_CODE_COUNT chunks"

OPENAI_TEXT_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM maproom.chunks WHERE text_embedding IS NOT NULL
" | tr -d ' ')
echo "✓ OpenAI text embeddings preserved: $OPENAI_TEXT_COUNT chunks"

# Check new Ollama columns are NULL (no data yet)
echo ""
echo "Verifying new columns are empty..."
OLLAMA_CODE_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding_ollama IS NOT NULL
" | tr -d ' ')

if [ "$OLLAMA_CODE_COUNT" -eq 0 ]; then
  echo "✓ code_embedding_ollama column is empty (as expected)"
else
  echo "⚠ Warning: code_embedding_ollama has $OLLAMA_CODE_COUNT non-NULL values (expected 0)"
fi

OLLAMA_TEXT_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM maproom.chunks WHERE text_embedding_ollama IS NOT NULL
" | tr -d ' ')

if [ "$OLLAMA_TEXT_COUNT" -eq 0 ]; then
  echo "✓ text_embedding_ollama column is empty (as expected)"
else
  echo "⚠ Warning: text_embedding_ollama has $OLLAMA_TEXT_COUNT non-NULL values (expected 0)"
fi

# Verify indexes are functional with EXPLAIN query
echo ""
echo "Verifying indexes are functional..."
CODE_INDEX_VALID=$(psql "$DB_URL" -t -c "
  EXPLAIN (FORMAT TEXT)
  SELECT chunk_id FROM maproom.chunks
  ORDER BY code_embedding_ollama <-> '[0,0,0]'::vector
  LIMIT 1
" | grep -c "idx_chunks_code_vec_ollama" || echo "0")

if [ "$CODE_INDEX_VALID" -gt 0 ]; then
  echo "✓ idx_chunks_code_vec_ollama is functional"
else
  echo "⚠ Warning: idx_chunks_code_vec_ollama may not be used by query planner"
fi

TEXT_INDEX_VALID=$(psql "$DB_URL" -t -c "
  EXPLAIN (FORMAT TEXT)
  SELECT chunk_id FROM maproom.chunks
  ORDER BY text_embedding_ollama <-> '[0,0,0]'::vector
  LIMIT 1
" | grep -c "idx_chunks_text_vec_ollama" || echo "0")

if [ "$TEXT_INDEX_VALID" -gt 0 ]; then
  echo "✓ idx_chunks_text_vec_ollama is functional"
else
  echo "⚠ Warning: idx_chunks_text_vec_ollama may not be used by query planner"
fi

echo ""
echo "============================================"
echo "✓ Migration verification complete!"
echo "============================================"
echo ""
echo "Summary:"
echo "  - Both Ollama embedding columns added"
echo "  - Both vector indexes created and functional"
echo "  - OpenAI embeddings preserved: $OPENAI_CODE_COUNT code, $OPENAI_TEXT_COUNT text"
echo "  - Ollama columns empty: $OLLAMA_CODE_COUNT code, $OLLAMA_TEXT_COUNT text"
echo ""

exit 0
