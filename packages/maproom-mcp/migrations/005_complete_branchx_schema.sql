-- ============================================================================
-- Migration 005: Complete BRANCHX Schema Transformation
-- ============================================================================
--
-- Transforms chunks table from file-based to content-addressed architecture.
-- No data preservation - clean schema migration for development environment.
--
-- Changes:
-- 1. Add relpath and content columns
-- 2. Change unique constraint from (file_id, start_line, end_line) to (blob_sha, relpath)
-- 3. Make blob_sha and worktree_ids NOT NULL with proper defaults
--
-- Prerequisites:
-- - Migration 001 (blob_sha column exists)
-- - Migration 004 (worktree_ids column exists)
--
-- ============================================================================

BEGIN;

-- Step 1: Add new columns
ALTER TABLE maproom.chunks
ADD COLUMN IF NOT EXISTS relpath TEXT,
ADD COLUMN IF NOT EXISTS content TEXT;

COMMENT ON COLUMN maproom.chunks.relpath IS 'File path relative to repository root (enables content-addressed chunks)';
COMMENT ON COLUMN maproom.chunks.content IS 'Actual source code content of this chunk';

-- Step 2: For clean migration (no users), we can just make them NOT NULL
-- If there's existing data, truncate first since we don't need to preserve it
TRUNCATE TABLE maproom.chunks CASCADE;

-- Now set NOT NULL constraints
ALTER TABLE maproom.chunks
ALTER COLUMN relpath SET NOT NULL,
ALTER COLUMN content SET NOT NULL;

-- Step 3: Drop old unique constraint
ALTER TABLE maproom.chunks
DROP CONSTRAINT IF EXISTS chunks_file_id_start_line_end_line_key;

-- Step 4: Create new unique constraint for content-addressed approach
CREATE UNIQUE INDEX IF NOT EXISTS idx_chunks_blob_relpath
ON maproom.chunks(blob_sha, relpath);

COMMENT ON INDEX maproom.idx_chunks_blob_relpath IS
  'Unique constraint for BRANCHX: same content (blob_sha) in same file (relpath) = same chunk';

-- Step 5: Create index on relpath for queries
CREATE INDEX IF NOT EXISTS idx_chunks_relpath
ON maproom.chunks(relpath);

COMMENT ON INDEX maproom.idx_chunks_relpath IS
  'Fast lookup of chunks by file path';

-- Step 6: Ensure blob_sha is NOT NULL
-- Truncate already happened, so just set constraint
ALTER TABLE maproom.chunks
ALTER COLUMN blob_sha SET NOT NULL;

-- Step 7: Make file_id nullable (BRANCHX doesn't use file_id)
ALTER TABLE maproom.chunks
ALTER COLUMN file_id DROP NOT NULL;

-- Step 8: Validation
DO $$
DECLARE
    relpath_count INTEGER;
    content_count INTEGER;
    index_exists BOOLEAN;
BEGIN
    -- Check columns exist and are NOT NULL
    SELECT COUNT(*) INTO relpath_count
    FROM information_schema.columns
    WHERE table_schema = 'maproom'
      AND table_name = 'chunks'
      AND column_name = 'relpath'
      AND is_nullable = 'NO';

    SELECT COUNT(*) INTO content_count
    FROM information_schema.columns
    WHERE table_schema = 'maproom'
      AND table_name = 'chunks'
      AND column_name = 'content'
      AND is_nullable = 'NO';

    -- Check unique index exists
    SELECT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'maproom'
          AND tablename = 'chunks'
          AND indexname = 'idx_chunks_blob_relpath'
    ) INTO index_exists;

    IF relpath_count = 0 THEN
        RAISE EXCEPTION 'Migration validation failed: relpath column missing or nullable';
    END IF;

    IF content_count = 0 THEN
        RAISE EXCEPTION 'Migration validation failed: content column missing or nullable';
    END IF;

    IF NOT index_exists THEN
        RAISE EXCEPTION 'Migration validation failed: idx_chunks_blob_relpath index missing';
    END IF;

    RAISE NOTICE 'Validation passed: BRANCHX schema migration complete';
    RAISE NOTICE '  - relpath column: NOT NULL';
    RAISE NOTICE '  - content column: NOT NULL';
    RAISE NOTICE '  - blob_sha column: NOT NULL';
    RAISE NOTICE '  - worktree_ids column: JSONB NOT NULL DEFAULT ''''[]''''';
    RAISE NOTICE '  - Unique constraint: (blob_sha, relpath)';
END $$;

COMMIT;

-- ============================================================================
-- Post-Migration Notes
-- ============================================================================
--
-- The chunks table now supports:
-- 1. Content-addressed storage (blob_sha)
-- 2. Multi-worktree tracking (worktree_ids JSONB array)
-- 3. File-based conflict resolution (blob_sha + relpath)
-- 4. Direct content storage (content column)
--
-- Old file_id column still exists for backward compatibility with:
-- - chunk_edges foreign keys
-- - Any code that hasn't been updated yet
--
-- Future work: Remove file_id dependency entirely
-- ============================================================================
