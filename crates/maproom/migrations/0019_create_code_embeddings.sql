-- SCHMAFIX-1001: Create code_embeddings table and migrate deduplicated embeddings
-- Purpose: Separate embeddings into content-addressed storage for massive deduplication
-- Source: packages/maproom-mcp/migrations/002_create_code_embeddings.sql
-- Depends on: 0018_add_blob_sha.sql (chunks must have blob_sha populated)

-- ============================================================================
-- STEP 1: Create code_embeddings table
-- ============================================================================
-- This table stores ONE embedding per unique blob SHA, enabling deduplication
-- Multiple chunks with identical content (same blob_sha) share the same embedding
-- Expected storage reduction: 70-90% based on typical code duplication rates

CREATE TABLE IF NOT EXISTS maproom.code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding vector(1536) NOT NULL,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TIMESTAMP DEFAULT NOW()
);

COMMENT ON TABLE maproom.code_embeddings IS
  'Deduplicated embedding storage using content-addressed keys (blob SHA). Each unique code blob has exactly one embedding.';

COMMENT ON COLUMN maproom.code_embeddings.blob_sha IS
  'Git-compatible SHA-256 hash of chunk content. PRIMARY KEY ensures natural deduplication.';

COMMENT ON COLUMN maproom.code_embeddings.embedding IS
  'OpenAI text-embedding-3-small vector (1536 dimensions). Shared by all chunks with identical content.';

COMMENT ON COLUMN maproom.code_embeddings.model_version IS
  'Embedding model identifier. Tracks which model generated this embedding for future migrations.';

COMMENT ON COLUMN maproom.code_embeddings.created_at IS
  'Timestamp when embedding was first computed. Used for tracking and debugging.';


-- ============================================================================
-- STEP 2: Migrate existing embeddings with deduplication
-- ============================================================================
-- Uses DISTINCT ON (blob_sha) to select ONE embedding per unique blob SHA
-- When multiple chunks share the same blob_sha, we keep the oldest (created_at ASC)
-- This preserves data lineage and ensures deterministic migration

DO $$
DECLARE
  total_chunks BIGINT;
  chunks_with_embeddings BIGINT;
  rows_inserted BIGINT;
BEGIN
  -- Get counts before migration
  SELECT COUNT(*) INTO total_chunks FROM maproom.chunks;
  SELECT COUNT(*) INTO chunks_with_embeddings FROM maproom.chunks WHERE embedding IS NOT NULL;

  RAISE NOTICE 'Starting embedding migration...';
  RAISE NOTICE 'Total chunks: %', total_chunks;
  RAISE NOTICE 'Chunks with embeddings: %', chunks_with_embeddings;

  -- Perform migration with deduplication
  INSERT INTO maproom.code_embeddings (blob_sha, embedding, model_version)
  SELECT DISTINCT ON (blob_sha)
    blob_sha,
    embedding,
    'text-embedding-3-small' AS model_version
  FROM maproom.chunks
  WHERE embedding IS NOT NULL
  ORDER BY blob_sha, created_at ASC;

  -- Report results
  GET DIAGNOSTICS rows_inserted = ROW_COUNT;

  RAISE NOTICE 'Migration complete: % unique embeddings inserted', rows_inserted;
  RAISE NOTICE 'Deduplication achieved: % embeddings eliminated (%.1f%% reduction)',
    chunks_with_embeddings - rows_inserted,
    100.0 * (chunks_with_embeddings - rows_inserted) / NULLIF(chunks_with_embeddings, 0);
END $$;


-- ============================================================================
-- STEP 3: Create HNSW vector index for similarity search
-- ============================================================================
-- HNSW (Hierarchical Navigable Small World) index enables fast approximate nearest neighbor search
-- Parameters: default m=16, ef_construction=64 (sufficient for MVP, can tune later)
-- vector_cosine_ops: optimized for cosine similarity (standard for embeddings)
-- This index may take several minutes for large datasets (progress shown in logs)

CREATE INDEX IF NOT EXISTS idx_embeddings_vector
ON maproom.code_embeddings
USING hnsw (embedding vector_cosine_ops);

COMMENT ON INDEX maproom.idx_embeddings_vector IS
  'HNSW index for fast cosine similarity search on embeddings. Enables efficient semantic code search.';


-- ============================================================================
-- STEP 4: Add foreign key constraint to chunks table
-- ============================================================================
-- Ensures referential integrity: every chunk's blob_sha must have a corresponding embedding
-- Prevents orphaned chunks (chunks without embeddings in code_embeddings table)
-- ON DELETE RESTRICT: cannot delete embedding if any chunk still references it

ALTER TABLE maproom.chunks
ADD CONSTRAINT IF NOT EXISTS fk_chunks_embedding
FOREIGN KEY (blob_sha) REFERENCES maproom.code_embeddings(blob_sha)
ON DELETE RESTRICT;

COMMENT ON CONSTRAINT fk_chunks_embedding ON maproom.chunks IS
  'Ensures all chunks reference valid embeddings in code_embeddings table. Prevents orphaned chunks.';


-- ============================================================================
-- STEP 5: Validation queries
-- ============================================================================
-- Comprehensive validation to ensure migration success and data integrity

-- Validation 1: Check for orphaned chunks (should be 0)
DO $$
DECLARE
  orphaned_count BIGINT;
BEGIN
  SELECT COUNT(*) INTO orphaned_count
  FROM maproom.chunks c
  LEFT JOIN maproom.code_embeddings e ON c.blob_sha = e.blob_sha
  WHERE e.blob_sha IS NULL AND c.embedding IS NOT NULL;

  IF orphaned_count > 0 THEN
    RAISE EXCEPTION 'Validation failed: Found % orphaned chunks (chunks without embeddings)', orphaned_count;
  ELSE
    RAISE NOTICE 'Validation 1 passed: No orphaned chunks';
  END IF;
END $$;

-- Validation 2: Verify deduplication achieved
DO $$
DECLARE
  total_chunks BIGINT;
  total_embeddings BIGINT;
  cache_efficiency NUMERIC;
BEGIN
  SELECT COUNT(*) INTO total_chunks FROM maproom.chunks;
  SELECT COUNT(*) INTO total_embeddings FROM maproom.code_embeddings;

  cache_efficiency := ROUND(100.0 * total_embeddings / NULLIF(total_chunks, 0), 2);

  IF total_embeddings >= total_chunks THEN
    RAISE WARNING 'No deduplication achieved: embeddings (%) >= chunks (%)', total_embeddings, total_chunks;
  ELSE
    RAISE NOTICE 'Validation 2 passed: Deduplication successful';
    RAISE NOTICE '  Total chunks:        %', total_chunks;
    RAISE NOTICE '  Unique embeddings:   %', total_embeddings;
    RAISE NOTICE '  Cache efficiency:    %%', cache_efficiency;
    RAISE NOTICE '  Duplicates removed:  % (%.1f%%)',
      total_chunks - total_embeddings,
      100.0 * (total_chunks - total_embeddings) / NULLIF(total_chunks, 0);
  END IF;
END $$;

-- Validation 3: Calculate storage savings
DO $$
DECLARE
  total_chunks BIGINT;
  unique_embeddings BIGINT;
  duplicate_chunks BIGINT;
  embedding_size_kb NUMERIC := 6.0; -- 1536 floats × 4 bytes = 6144 bytes ≈ 6KB
  storage_saved_mb NUMERIC;
  storage_saved_gb NUMERIC;
BEGIN
  SELECT COUNT(*) INTO total_chunks FROM maproom.chunks;
  SELECT COUNT(*) INTO unique_embeddings FROM maproom.code_embeddings;
  duplicate_chunks := total_chunks - unique_embeddings;

  -- Calculate storage savings (duplicate_chunks × 6KB)
  storage_saved_mb := ROUND((duplicate_chunks * embedding_size_kb) / 1024.0, 2);
  storage_saved_gb := ROUND(storage_saved_mb / 1024.0, 3);

  RAISE NOTICE '';
  RAISE NOTICE '=== Storage Savings Analysis ===';
  RAISE NOTICE 'Embedding size per chunk:    % KB', embedding_size_kb;
  RAISE NOTICE 'Duplicate chunks eliminated: %', duplicate_chunks;
  RAISE NOTICE 'Storage saved:               % MB (% GB)', storage_saved_mb, storage_saved_gb;
  RAISE NOTICE '';
  RAISE NOTICE 'Note: This only accounts for embedding vectors. Additional savings from index deduplication not included.';
END $$;

-- Validation 4: Verify HNSW index exists and is usable
DO $$
DECLARE
  index_exists BOOLEAN;
BEGIN
  SELECT EXISTS (
    SELECT 1
    FROM pg_indexes
    WHERE schemaname = 'maproom'
      AND tablename = 'code_embeddings'
      AND indexname = 'idx_embeddings_vector'
  ) INTO index_exists;

  IF index_exists THEN
    RAISE NOTICE 'Validation 4 passed: HNSW index created successfully';
  ELSE
    RAISE EXCEPTION 'Validation failed: HNSW index not found';
  END IF;
END $$;


-- ============================================================================
-- Migration complete
-- ============================================================================
-- Next steps:
--   1. Update application queries to JOIN chunks with code_embeddings
--   2. Remove embedding column from chunks table (future migration)
--   3. Update Rust indexer to insert into code_embeddings
--
-- Performance notes:
--   - Query planner will automatically use HNSW index for vector similarity searches
--   - Foreign key constraint adds minimal overhead (indexed lookups)
--   - Expected query performance: same or better (smaller working set, better cache hits)
--
-- Rollback procedure (if needed):
--   1. ALTER TABLE chunks DROP CONSTRAINT fk_chunks_embedding;
--   2. DROP INDEX idx_embeddings_vector;
--   3. DROP TABLE code_embeddings;
--   Note: Original embeddings still in chunks.embedding column until future migration
