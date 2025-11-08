-- Migration 004: Add worktree tracking to chunks table
-- Purpose: Enable worktree-specific indexing and incremental updates using git tree SHA
-- Related: BRANCHX-1001, BRANCHX-1002
-- Depends on: 002_create_code_embeddings.sql (chunks table with blob_sha column)

-- ============================================================================
-- STEP 1: Add worktree_ids JSONB column (BRANCHX-1001)
-- ============================================================================
-- This column tracks which worktrees contain each chunk, enabling:
-- - Branch-specific code search
-- - Incremental updates per worktree
-- - Efficient branch switching workflows

-- Add column with default empty array (idempotent)
ALTER TABLE maproom.chunks ADD COLUMN IF NOT EXISTS worktree_ids JSONB DEFAULT '[]';

COMMENT ON COLUMN maproom.chunks.worktree_ids IS
  'JSONB array of worktree IDs containing this chunk. Enables branch-specific search and incremental updates.';


-- ============================================================================
-- STEP 2: Backfill existing chunks with their worktree ID
-- ============================================================================
-- Join through files table to identify each chunk''s worktree
-- Orphan chunks (file_id NULL or invalid) remain with empty array

DO $$
DECLARE
  total_chunks BIGINT;
  chunks_to_backfill BIGINT;
  rows_updated BIGINT;
BEGIN
  SELECT COUNT(*) INTO total_chunks FROM maproom.chunks;
  SELECT COUNT(*) INTO chunks_to_backfill
  FROM maproom.chunks
  WHERE file_id IS NOT NULL;

  RAISE NOTICE 'Starting worktree_ids backfill...';
  RAISE NOTICE 'Total chunks: %', total_chunks;
  RAISE NOTICE 'Chunks with valid file_id: %', chunks_to_backfill;

  -- Backfill chunks with their worktree ID
  UPDATE maproom.chunks c
  SET worktree_ids = jsonb_build_array(
    (SELECT w.id
     FROM maproom.worktrees w
     JOIN maproom.files f ON f.worktree_id = w.id
     WHERE f.id = c.file_id)
  )
  WHERE c.file_id IS NOT NULL
    AND EXISTS (
      SELECT 1
      FROM maproom.files f
      JOIN maproom.worktrees w ON f.worktree_id = w.id
      WHERE f.id = c.file_id
    );

  GET DIAGNOSTICS rows_updated = ROW_COUNT;

  RAISE NOTICE 'Backfill complete: % chunks updated', rows_updated;

  IF rows_updated < chunks_to_backfill THEN
    RAISE NOTICE 'Note: % chunks remain with empty worktree_ids (orphaned or missing worktree reference)',
      chunks_to_backfill - rows_updated;
  END IF;
END $$;


-- ============================================================================
-- STEP 3: Make worktree_ids NOT NULL after backfill
-- ============================================================================
-- All chunks now have worktree_ids (either populated or empty array)
-- NEW chunks will use DEFAULT '[]' for safety

ALTER TABLE maproom.chunks ALTER COLUMN worktree_ids SET NOT NULL;

DO $$ BEGIN
  RAISE NOTICE 'worktree_ids column is now NOT NULL with DEFAULT ''[]''';
END $$;


-- ============================================================================
-- STEP 4: Create GIN index for efficient JSONB queries
-- ============================================================================
-- Enables fast worktree filtering using JSONB operators:
-- - worktree_ids ? '2'            (contains specific worktree)
-- - worktree_ids ?| ARRAY['2','5'] (contains any of multiple worktrees)
-- - worktree_ids ?& ARRAY['2','5'] (contains all worktrees)

CREATE INDEX IF NOT EXISTS idx_chunks_worktree_ids ON maproom.chunks USING gin(worktree_ids);

COMMENT ON INDEX maproom.idx_chunks_worktree_ids IS
  'GIN index for efficient JSONB queries on worktree_ids. Enables fast branch-specific search.';


-- ============================================================================
-- STEP 5: Create worktree_index_state table (BRANCHX-1002)
-- ============================================================================
-- Tracks the last indexed git tree SHA for each worktree
-- Enables incremental update optimization via tree SHA comparison

CREATE TABLE IF NOT EXISTS maproom.worktree_index_state (
  worktree_id BIGINT PRIMARY KEY REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
  last_tree_sha TEXT NOT NULL,
  last_indexed TIMESTAMP DEFAULT NOW(),
  chunks_processed INT DEFAULT 0,
  embeddings_generated INT DEFAULT 0
);

COMMENT ON TABLE maproom.worktree_index_state IS
  'Tracks indexed state per worktree using git tree SHA. Enables incremental update optimization.';

COMMENT ON COLUMN maproom.worktree_index_state.worktree_id IS
  'Foreign key to worktrees table. One row per worktree.';

COMMENT ON COLUMN maproom.worktree_index_state.last_tree_sha IS
  'Git tree SHA from last successful index. Output of: git rev-parse HEAD^{tree}. Used to detect changes.';

COMMENT ON COLUMN maproom.worktree_index_state.last_indexed IS
  'Timestamp of last successful index operation.';

COMMENT ON COLUMN maproom.worktree_index_state.chunks_processed IS
  'Total chunks processed in last index operation. Used for metrics and cost tracking.';

COMMENT ON COLUMN maproom.worktree_index_state.embeddings_generated IS
  'New embeddings generated in last index operation. Used for cost calculation.';


-- ============================================================================
-- STEP 6: Create index on last_tree_sha for fast lookups
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_worktree_index_state_tree_sha
ON maproom.worktree_index_state(last_tree_sha);


-- ============================================================================
-- STEP 7: Initialize index state for existing worktrees
-- ============================================================================
-- Use 'init' as placeholder tree SHA for first-time indexing

DO $$
DECLARE
  worktrees_initialized BIGINT;
BEGIN
  INSERT INTO maproom.worktree_index_state (worktree_id, last_tree_sha)
  SELECT id, 'init'
  FROM maproom.worktrees
  ON CONFLICT (worktree_id) DO NOTHING;

  GET DIAGNOSTICS worktrees_initialized = ROW_COUNT;

  RAISE NOTICE 'Initialized index state for % existing worktrees', worktrees_initialized;
END $$;


-- ============================================================================
-- STEP 8: Validation queries
-- ============================================================================

-- Validation 1: Verify all chunks have worktree_ids
DO $$
DECLARE
  chunks_without_worktree_ids BIGINT;
BEGIN
  SELECT COUNT(*) INTO chunks_without_worktree_ids
  FROM maproom.chunks
  WHERE worktree_ids IS NULL;

  IF chunks_without_worktree_ids > 0 THEN
    RAISE EXCEPTION 'Validation failed: Found % chunks with NULL worktree_ids', chunks_without_worktree_ids;
  ELSE
    RAISE NOTICE 'Validation 1 passed: All chunks have worktree_ids (% total)',
      (SELECT COUNT(*) FROM maproom.chunks);
  END IF;
END $$;

-- Validation 2: Verify GIN index exists and is usable
DO $$
DECLARE
  index_exists BOOLEAN;
BEGIN
  SELECT EXISTS (
    SELECT 1
    FROM pg_indexes
    WHERE schemaname = 'maproom'
      AND tablename = 'chunks'
      AND indexname = 'idx_chunks_worktree_ids'
  ) INTO index_exists;

  IF index_exists THEN
    RAISE NOTICE 'Validation 2 passed: GIN index on worktree_ids created successfully';
  ELSE
    RAISE EXCEPTION 'Validation failed: GIN index idx_chunks_worktree_ids not found';
  END IF;
END $$;

-- Validation 3: Verify worktree_index_state table created
DO $$
DECLARE
  table_exists BOOLEAN;
  initialized_worktrees BIGINT;
BEGIN
  SELECT EXISTS (
    SELECT 1
    FROM information_schema.tables
    WHERE table_schema = 'maproom'
      AND table_name = 'worktree_index_state'
  ) INTO table_exists;

  IF table_exists THEN
    SELECT COUNT(*) INTO initialized_worktrees FROM maproom.worktree_index_state;
    RAISE NOTICE 'Validation 3 passed: worktree_index_state table created with % rows',
      initialized_worktrees;
  ELSE
    RAISE EXCEPTION 'Validation failed: worktree_index_state table not found';
  END IF;
END $$;

-- Validation 4: Display worktree tracking statistics
DO $$
DECLARE
  total_chunks BIGINT;
  populated_chunks BIGINT;
  empty_chunks BIGINT;
  unique_worktrees_in_chunks BIGINT;
BEGIN
  SELECT COUNT(*) INTO total_chunks FROM maproom.chunks;
  SELECT COUNT(*) INTO populated_chunks
  FROM maproom.chunks
  WHERE jsonb_array_length(worktree_ids) > 0;

  SELECT COUNT(*) INTO empty_chunks
  FROM maproom.chunks
  WHERE jsonb_array_length(worktree_ids) = 0;

  SELECT COUNT(DISTINCT elem) INTO unique_worktrees_in_chunks
  FROM maproom.chunks,
       jsonb_array_elements_text(worktree_ids) AS elem;

  RAISE NOTICE '';
  RAISE NOTICE '=== Worktree Tracking Statistics ===';
  RAISE NOTICE 'Total chunks:                  %', total_chunks;
  RAISE NOTICE 'Chunks with worktree_ids:      % (%.1f%%)',
    populated_chunks,
    100.0 * populated_chunks / NULLIF(total_chunks, 0);
  RAISE NOTICE 'Chunks with empty worktree_ids: % (%.1f%%)',
    empty_chunks,
    100.0 * empty_chunks / NULLIF(total_chunks, 0);
  RAISE NOTICE 'Unique worktrees referenced:   %', unique_worktrees_in_chunks;
  RAISE NOTICE '';
END $$;


-- ============================================================================
-- Migration complete
-- ============================================================================
-- Next steps:
--   1. Implement git integration functions (BRANCHX-1004)
--   2. Create incremental update algorithm (BRANCHX-1007)
--   3. Update MCP search to filter by worktree (BRANCHX-1012)
--
-- Query examples:
--   -- Find chunks in specific worktree
--   SELECT * FROM chunks WHERE worktree_ids ? '2';
--
--   -- Find chunks in multiple worktrees (OR)
--   SELECT * FROM chunks WHERE worktree_ids ?| ARRAY['2', '5'];
--
--   -- Get current tree SHA
--   SELECT last_tree_sha FROM worktree_index_state WHERE worktree_id = 1;
--
-- Rollback procedure (if needed):
--   1. DROP TABLE worktree_index_state;
--   2. DROP INDEX idx_chunks_worktree_ids;
--   3. ALTER TABLE chunks DROP COLUMN worktree_ids;
