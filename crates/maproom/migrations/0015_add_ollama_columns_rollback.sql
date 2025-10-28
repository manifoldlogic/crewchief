-- Migration 0015 ROLLBACK: Remove 768-dimensional embedding columns for Ollama and Google providers
-- Estimated duration: < 1 minute for 25K chunks
-- Safety: Non-blocking (CONCURRENTLY), idempotent (IF EXISTS)
--
-- ROLLBACK SAFETY: Only run if:
--   1. No Ollama/Google embeddings generated yet (all columns are NULL), OR
--   2. You're okay losing Ollama/Google embeddings and re-embedding later
--
-- This rollback removes:
-- - code_embedding_ollama and text_embedding_ollama columns (768 dimensions)
-- - Associated IVFFlat indexes
--
-- CRITICAL: Existing OpenAI columns (code_embedding, text_embedding) remain UNCHANGED.
-- This rollback only affects the 768-dim columns added in migration 0015.

BEGIN;

-- Check for data before dropping (warning only, doesn't prevent rollback)
-- This helps operators understand data loss implications
DO $$
DECLARE
  ollama_count INTEGER;
  columns_exist BOOLEAN;
BEGIN
  -- First check if columns exist before querying them
  SELECT EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_schema = 'maproom'
      AND table_name = 'chunks'
      AND column_name IN ('code_embedding_ollama', 'text_embedding_ollama')
  ) INTO columns_exist;

  IF columns_exist THEN
    -- Columns exist, check for data
    SELECT COUNT(*) INTO ollama_count
    FROM maproom.chunks
    WHERE code_embedding_ollama IS NOT NULL OR text_embedding_ollama IS NOT NULL;

    IF ollama_count > 0 THEN
      RAISE WARNING 'Found % chunks with Ollama/Google embeddings. These will be LOST if you proceed with rollback!', ollama_count;
      RAISE WARNING 'Consider backing up the database before proceeding if you need to preserve this data.';
    ELSE
      RAISE NOTICE 'Ollama/Google columns exist but contain no data. Safe to rollback without data loss.';
    END IF;
  ELSE
    -- Columns don't exist, nothing to check
    RAISE NOTICE 'Ollama/Google columns do not exist. Rollback is idempotent and safe.';
  END IF;
END $$;

COMMIT;

-- Drop indexes OUTSIDE transaction (CONCURRENTLY requires this)
-- Estimated duration: < 1 second (dropping indexes is fast)
--
-- IF NOT EXISTS prevents errors if indexes were never created or already dropped
-- CONCURRENTLY ensures non-blocking operation:
-- - Reads and writes continue during index removal
-- - Safe for production deployment
-- - Cannot run inside a transaction block

DROP INDEX CONCURRENTLY IF EXISTS maproom.idx_chunks_code_vec_ollama;
DROP INDEX CONCURRENTLY IF EXISTS maproom.idx_chunks_text_vec_ollama;

BEGIN;

-- Drop columns (estimated duration: < 1 second for metadata change)
-- Column drop is a metadata operation in PostgreSQL, not a table rewrite
--
-- IF EXISTS prevents errors if columns were never created or already dropped
-- This makes the rollback idempotent (safe to run multiple times)
--
-- Note: Column drop will fail if foreign key dependencies exist (safety feature)
-- This is intentional - it prevents accidental data loss from dependencies

ALTER TABLE maproom.chunks
  DROP COLUMN IF EXISTS code_embedding_ollama,
  DROP COLUMN IF EXISTS text_embedding_ollama;

COMMIT;

-- VERIFICATION QUERIES (run these after rollback to verify success):
--
-- 1. Verify columns are dropped:
--    SELECT column_name FROM information_schema.columns
--    WHERE table_schema = 'maproom' AND table_name = 'chunks'
--    AND column_name LIKE '%ollama%';
--    -- Should return 0 rows
--
-- 2. Verify indexes are dropped:
--    SELECT indexname FROM pg_indexes
--    WHERE schemaname = 'maproom' AND tablename = 'chunks'
--    AND indexname LIKE '%ollama%';
--    -- Should return 0 rows
--
-- 3. Verify OpenAI columns still exist:
--    SELECT column_name FROM information_schema.columns
--    WHERE table_schema = 'maproom' AND table_name = 'chunks'
--    AND column_name IN ('code_embedding', 'text_embedding');
--    -- Should return 2 rows

-- ROLLBACK COMPLETE
-- Data loss: Any embeddings in code_embedding_ollama and text_embedding_ollama are now permanently deleted
-- Recovery: Re-run forward migration 0015 and re-embed affected chunks using Ollama or Google providers
