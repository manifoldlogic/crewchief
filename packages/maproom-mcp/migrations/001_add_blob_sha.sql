-- Migration 001: Add blob_sha column to chunks table
-- Purpose: Enable content-addressed chunk storage for embedding deduplication
-- Related: BLOBSHA-1002

-- ============================================================================
-- STEP 1: Create PostgreSQL function for blob SHA computation
-- ============================================================================
-- This function must produce identical output to the Rust implementation
-- in crates/maproom/src/content_hash.rs::compute_blob_sha()
-- Format: SHA256("blob <size>\0<content>")

CREATE OR REPLACE FUNCTION maproom.compute_git_blob_sha(content TEXT)
RETURNS TEXT AS $$
  SELECT encode(
    digest(
      'blob ' || length(content) || E'\0' || content,
      'sha256'
    ),
    'hex'
  );
$$ LANGUAGE SQL IMMUTABLE;

COMMENT ON FUNCTION maproom.compute_git_blob_sha(TEXT) IS
  'Computes Git-compatible blob SHA-256 for content. Format: SHA256("blob <size>\0<content>")';


-- ============================================================================
-- STEP 2: Add blob_sha column (nullable initially for safe migration)
-- ============================================================================

ALTER TABLE maproom.chunks
ADD COLUMN blob_sha TEXT;

COMMENT ON COLUMN maproom.chunks.blob_sha IS
  'Git-compatible SHA-256 hash of chunk content. Used as key for embedding deduplication.';


-- ============================================================================
-- STEP 3: Create index with CONCURRENTLY (non-blocking)
-- ============================================================================
-- Using CONCURRENTLY allows the index to be built without blocking writes
-- This is critical for production databases with ongoing traffic

CREATE INDEX CONCURRENTLY idx_chunks_blob_sha
ON maproom.chunks(blob_sha);


-- ============================================================================
-- STEP 4: Backfill all existing chunks in batches
-- ============================================================================
-- Process 1000 rows at a time to avoid long-running transactions
-- Each batch commits separately, allowing progress to be saved incrementally

DO $$
DECLARE
  batch_size INT := 1000;
  rows_updated INT;
  total_processed INT := 0;
  total_chunks INT;
BEGIN
  -- Get total count for progress tracking
  SELECT COUNT(*) INTO total_chunks
  FROM maproom.chunks
  WHERE blob_sha IS NULL;

  RAISE NOTICE 'Starting backfill of % chunks', total_chunks;

  -- Process in batches until no rows remain
  LOOP
    -- Update next batch of rows
    UPDATE maproom.chunks
    SET blob_sha = maproom.compute_git_blob_sha(preview)
    WHERE id IN (
      SELECT id
      FROM maproom.chunks
      WHERE blob_sha IS NULL
      LIMIT batch_size
    );

    -- Get number of rows updated in this batch
    GET DIAGNOSTICS rows_updated = ROW_COUNT;

    -- Exit if no more rows to process
    EXIT WHEN rows_updated = 0;

    -- Update progress counter
    total_processed := total_processed + rows_updated;

    -- Log progress every batch
    RAISE NOTICE 'Processed % of % chunks (%.1f%%)',
      total_processed,
      total_chunks,
      (100.0 * total_processed / NULLIF(total_chunks, 0));

    -- Commit this batch before proceeding to next
    COMMIT;
  END LOOP;

  RAISE NOTICE 'Backfill complete: % chunks processed', total_processed;
END $$;


-- ============================================================================
-- STEP 5: Make column NOT NULL after backfill completes
-- ============================================================================
-- Only enforce NOT NULL constraint after all existing data is populated
-- This ensures data integrity going forward

ALTER TABLE maproom.chunks
ALTER COLUMN blob_sha SET NOT NULL;


-- ============================================================================
-- STEP 6: Validation queries
-- ============================================================================
-- Verify migration success and analyze deduplication potential

-- Check for any NULL values (should be 0)
DO $$
DECLARE
  null_count INT;
BEGIN
  SELECT COUNT(*) INTO null_count
  FROM maproom.chunks
  WHERE blob_sha IS NULL;

  IF null_count > 0 THEN
    RAISE WARNING 'Found % chunks with NULL blob_sha', null_count;
  ELSE
    RAISE NOTICE 'Validation passed: All chunks have blob_sha';
  END IF;
END $$;

-- Display deduplication metrics
DO $$
DECLARE
  total_chunks BIGINT;
  unique_blobs BIGINT;
  duplicate_chunks BIGINT;
  dedup_pct NUMERIC;
BEGIN
  SELECT
    COUNT(*),
    COUNT(DISTINCT blob_sha),
    COUNT(*) - COUNT(DISTINCT blob_sha),
    ROUND(100.0 * (COUNT(*) - COUNT(DISTINCT blob_sha)) / NULLIF(COUNT(*), 0), 2)
  INTO total_chunks, unique_blobs, duplicate_chunks, dedup_pct
  FROM maproom.chunks;

  RAISE NOTICE '=== Deduplication Analysis ===';
  RAISE NOTICE 'Total chunks:        %', total_chunks;
  RAISE NOTICE 'Unique blob SHAs:    %', unique_blobs;
  RAISE NOTICE 'Duplicate chunks:    %', duplicate_chunks;
  RAISE NOTICE 'Deduplication rate:  %%', dedup_pct;
  RAISE NOTICE '';
  RAISE NOTICE 'Potential embedding savings: %%', dedup_pct;
END $$;


-- ============================================================================
-- Migration complete
-- ============================================================================
-- Next steps:
--   1. Verify Rust and PostgreSQL functions produce identical output
--   2. Create code_embeddings table (BLOBSHA-1003)
--   3. Migrate embeddings to new table (BLOBSHA-1004)
--   4. Update application queries to use JOINs (BLOBSHA-1005)
