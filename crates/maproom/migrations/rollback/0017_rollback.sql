-- Rollback 0017: Restore original covering index
--
-- ⚠️  WARNING: This rollback will FAIL if the database contains chunks with preview > 2704 bytes
--
-- After running migration 0017 (multi-index strategy), if large-preview chunks were indexed,
-- this rollback cannot restore the original idx_chunks_search_covering because that index
-- fails on rows exceeding the PostgreSQL B-tree 2704-byte limit.
--
-- RECOMMENDATION: Use forward-fix instead of rollback if issues occur.
-- Only use this rollback on databases that have NOT indexed any large-preview chunks.
--
-- Migration 0017 created 2 indexes (not 3 - hash-based index was invalid PostgreSQL syntax):
-- - idx_chunks_search_small_preview (partial covering index)
-- - idx_chunks_search_basic (universal fallback)

BEGIN;

-- Drop new indexes created by migration 0017
DROP INDEX IF EXISTS maproom.idx_chunks_search_small_preview;
DROP INDEX IF EXISTS maproom.idx_chunks_search_basic;

-- Note: idx_chunks_search_hash was NOT created in migration 0017 due to PostgreSQL
-- limitation (expressions not allowed in INCLUDE clauses), so no need to drop it

COMMIT;

-- Attempt to restore original covering index
-- This will FAIL with "index row size exceeds btree maximum" if any chunk has preview > 2704 bytes
CREATE INDEX CONCURRENTLY idx_chunks_search_covering
  ON maproom.chunks (file_id, kind, start_line)
  INCLUDE (symbol_name, preview);

COMMENT ON INDEX maproom.idx_chunks_search_covering IS
  'Original covering index restored by rollback. WARNING: Will fail if large previews exist.';
