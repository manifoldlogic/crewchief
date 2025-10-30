# Ticket: MPEMBED-1003: Create migration verification script

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- migration-safety-specialist
- test-runner
- verify-ticket
- commit-ticket

## Summary
Write automated verification script that confirms migration succeeded: columns exist, indexes exist, existing data preserved, new columns are NULL.

## Background
Manual verification is error-prone and doesn't scale for CI/CD pipelines. We need automated checks for production deployment that can run post-migration and as part of continuous integration. The verification should detect partial migration state (columns but no indexes) and confirm zero data loss of existing OpenAI embeddings.

This implements Phase 1: Database Migration verification from the MPEMBED multi-provider embeddings plan.

## Acceptance Criteria
- [x] Script checks `code_embedding_ollama` column exists
- [x] Script checks `text_embedding_ollama` column exists
- [x] Script checks `idx_chunks_code_vec_ollama` index exists
- [x] Script checks `idx_chunks_text_vec_ollama` index exists
- [x] Script verifies existing OpenAI embedding count unchanged (23,632 chunks)
- [x] Script verifies new Ollama columns are NULL (no data yet)
- [x] Script exits with status code 0 on success, non-zero on failure
- [x] Script outputs clear ✓ or ✗ for each check

## Technical Requirements
- File location: `crates/maproom/scripts/verify_migration_0015.sh`
- Use `psql` to query PostgreSQL metadata tables
- Check `pg_indexes` for index existence
- Count non-NULL values in each column
- Use `set -e` to exit on first error
- Support DATABASE_URL environment variable
- Make executable: `chmod +x`

## Implementation Notes
The verification script should follow this structure:

```bash
#!/bin/bash
# verify_migration_0015.sh
# Verifies migration 0015 completed successfully

set -e

DB_URL="${DATABASE_URL:-postgresql://postgres:postgres@postgres:5432/crewchief}"

echo "Verifying migration 0015..."

# Check code_embedding_ollama column existence
if psql "$DB_URL" -c "SELECT code_embedding_ollama FROM maproom.chunks LIMIT 1" &>/dev/null; then
  echo "✓ code_embedding_ollama column exists"
else
  echo "✗ code_embedding_ollama column missing"
  exit 1
fi

# Check text_embedding_ollama column existence
if psql "$DB_URL" -c "SELECT text_embedding_ollama FROM maproom.chunks LIMIT 1" &>/dev/null; then
  echo "✓ text_embedding_ollama column exists"
else
  echo "✗ text_embedding_ollama column missing"
  exit 1
fi

# Check code index existence
INDEX_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM pg_indexes
  WHERE tablename='chunks' AND indexname='idx_chunks_code_vec_ollama'
")

if [ "$INDEX_COUNT" -eq 1 ]; then
  echo "✓ idx_chunks_code_vec_ollama index exists"
else
  echo "✗ idx_chunks_code_vec_ollama index missing"
  exit 1
fi

# Check text index existence
INDEX_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM pg_indexes
  WHERE tablename='chunks' AND indexname='idx_chunks_text_vec_ollama'
")

if [ "$INDEX_COUNT" -eq 1 ]; then
  echo "✓ idx_chunks_text_vec_ollama index exists"
else
  echo "✗ idx_chunks_text_vec_ollama index missing"
  exit 1
fi

# Check existing embeddings preserved
OPENAI_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL
")
echo "✓ OpenAI embeddings preserved: $OPENAI_COUNT chunks"

# Check new columns are NULL
OLLAMA_COUNT=$(psql "$DB_URL" -t -c "
  SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding_ollama IS NOT NULL
")
if [ "$OLLAMA_COUNT" -eq 0 ]; then
  echo "✓ Ollama columns are empty (as expected)"
else
  echo "⚠  Warning: Ollama columns have $OLLAMA_COUNT non-NULL values"
fi

echo "✓ Migration verification complete!"
```

Key considerations:
- Exit on first error (`set -e`) ensures no false positives
- Clear ✓/✗ output for human readability
- Metadata queries use PostgreSQL system catalogs
- Count verification ensures data preservation

## Dependencies
- MPEMBED-1001 (verifies the forward migration)

## Risk Assessment
- **Risk**: Script passes but index is invalid (e.g., corrupt)
  - **Mitigation**: Add `EXPLAIN ANALYZE` query using index to verify functionality
- **Risk**: Script assumes specific chunk count (23,632) which may vary
  - **Mitigation**: Log actual count, don't hard-code expected value in assertion

## Files/Packages Affected
- crates/maproom/scripts/verify_migration_0015.sh (create)
