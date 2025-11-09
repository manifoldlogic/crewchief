-- SCHMAFIX-1001: Add blob_sha column to chunks table
-- Purpose: Enable content-addressed chunk storage for embedding deduplication
-- Source: packages/maproom-mcp/migrations/001_add_blob_sha.sql
-- Changes: Simplified for transaction safety - removed CONCURRENTLY, batched backfill, and validation blocks

-- ============================================================================
-- STEP 1: Enable pgcrypto extension for digest() function
-- ============================================================================
-- Required for SHA-256 hash computation in compute_git_blob_sha function

CREATE EXTENSION IF NOT EXISTS pgcrypto;


-- ============================================================================
-- STEP 2: Create PostgreSQL function for blob SHA computation
-- ============================================================================
-- This function must produce identical output to the Rust implementation
-- in crates/maproom/src/content_hash.rs::compute_blob_sha()
-- Format: SHA256("blob <size>\0<content>")

CREATE OR REPLACE FUNCTION maproom.compute_git_blob_sha(content TEXT)
RETURNS TEXT AS $$
  SELECT encode(
    digest(
      convert_to('blob ' || length(content), 'UTF8') || '\x00'::bytea || convert_to(content, 'UTF8'),
      'sha256'::text
    ),
    'hex'
  );
$$ LANGUAGE SQL IMMUTABLE;

COMMENT ON FUNCTION maproom.compute_git_blob_sha(TEXT) IS
  'Computes Git-compatible blob SHA-256 for content. Format: SHA256("blob <size>\0<content>")';


-- ============================================================================
-- STEP 3: Add blob_sha column (nullable initially for safe migration)
-- ============================================================================

ALTER TABLE maproom.chunks
ADD COLUMN IF NOT EXISTS blob_sha TEXT;

COMMENT ON COLUMN maproom.chunks.blob_sha IS
  'Git-compatible SHA-256 hash of chunk content. Used as key for embedding deduplication.';


-- ============================================================================
-- STEP 4: Create index with IF NOT EXISTS (transaction-safe)
-- ============================================================================
-- Using IF NOT EXISTS ensures idempotency without requiring CONCURRENTLY
-- This is safe for migration runner which runs in transactions

CREATE INDEX IF NOT EXISTS idx_chunks_blob_sha
ON maproom.chunks(blob_sha);


-- ============================================================================
-- STEP 5: Backfill all existing chunks with blob SHA
-- ============================================================================
-- Simplified backfill - single UPDATE statement
-- Migration runner handles transaction management and error reporting
-- Handle NULL previews by using empty string (produces valid SHA for empty content)

UPDATE maproom.chunks
SET blob_sha = maproom.compute_git_blob_sha(COALESCE(preview, ''))
WHERE blob_sha IS NULL;


-- ============================================================================
-- STEP 6: Make column NOT NULL after backfill completes
-- ============================================================================
-- Only enforce NOT NULL constraint after all existing data is populated
-- This ensures data integrity going forward

ALTER TABLE maproom.chunks
ALTER COLUMN blob_sha SET NOT NULL;


-- ============================================================================
-- Migration complete
-- ============================================================================
-- Next steps:
--   1. Verify Rust and PostgreSQL functions produce identical output
--   2. Create code_embeddings table (migration 0019)
--   3. Update application queries to use JOINs
