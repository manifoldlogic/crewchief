-- Migration 0017: Fix index size limit errors
--
-- Problem: idx_chunks_search_covering fails when preview > 2704 bytes
-- Solution: Two-index strategy (partial covering + basic fallback)
--
-- Note: Original plan included hash-based index with MD5(preview), but PostgreSQL
-- does not support expressions in INCLUDE clauses. The two-index approach provides
-- the same coverage: partial index handles 95%+ of data, basic index handles 100%.
--
-- References:
-- - .agents/projects/IDXSIZE_index-size-limits/planning/architecture.md
-- - .agents/projects/IDXSIZE_index-size-limits/planning/analysis.md

SET statement_timeout = '10min';

BEGIN;

-- Drop the problematic covering index
DROP INDEX IF EXISTS maproom.idx_chunks_search_covering;

COMMIT;

-- Create new indexes concurrently (no lock)

-- Partial covering index for small previews (95% of data)
-- This enables index-only scans for the vast majority of chunks
CREATE INDEX CONCURRENTLY idx_chunks_search_small_preview
  ON maproom.chunks (file_id, kind, start_line)
  INCLUDE (symbol_name, preview)
  WHERE LENGTH(preview) <= 2000;

COMMENT ON INDEX maproom.idx_chunks_search_small_preview IS
  'Covering index for search queries with preview <= 2000 bytes. Enables index-only scans for 95%+ of chunks.';

-- Basic non-covering index as universal fallback (100% of data)
-- Handles large previews with heap lookup (slightly slower but works for all data)
CREATE INDEX CONCURRENTLY idx_chunks_search_basic
  ON maproom.chunks (file_id, kind, start_line);

COMMENT ON INDEX maproom.idx_chunks_search_basic IS
  'Basic index for chunks with large previews. Requires heap lookup but works for 100% of data, including chunks exceeding 2704-byte limit.';

-- Update statistics for query planner
ANALYZE maproom.chunks;

RESET statement_timeout;
