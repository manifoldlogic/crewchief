-- Dimension-typed embedding storage + cosine HNSW ANN index (spec §4-§5, F04+F03).
--
-- Replaces the typeless `embedding vector` column (0002) with one typed column per
-- supported dim (768/1024/1536) in the SAME content-addressed pool: pgvector cannot
-- build an HNSW index on a typeless `vector`, so dimension-typing and the ANN index
-- must land together. Each row populates exactly the column matching its
-- `embedding_dim`; the other two are NULL (NULLs cost no storage in Postgres). A
-- partial cosine HNSW index per dim then serves the KNN scan via the `<=>` operator,
-- replacing the unbounded brute-force L2 sequential scan (0002/0003's Phase-2 note).
--
-- Runs inside the migration runner's single advisory-locked transaction, which lifts
-- statement_timeout — so the (locking, non-CONCURRENT: Postgres forbids CONCURRENTLY
-- in a tx block) HNSW build completes. Applies cleanly on a fresh pool (empty table
-- -> backfill is a no-op) and on a populated one (existing rows copy into their typed
-- column, then the typeless column is dropped). Idempotent by schema_migrations
-- version — never re-run once recorded.

-- Fail loudly if any existing row carries an unsupported dim: dropping the typeless
-- column would otherwise leave it with no typed column and silently unsearchable.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM code_embeddings WHERE embedding_dim NOT IN (768, 1024, 1536)) THEN
        RAISE EXCEPTION 'migration 0004: code_embeddings contains rows with an unsupported embedding_dim (not in 768/1024/1536); refusing to migrate to typed columns and leave them unindexable';
    END IF;
END $$;

-- One typed vector column per supported dim.
ALTER TABLE code_embeddings ADD COLUMN embedding_768  vector(768);
ALTER TABLE code_embeddings ADD COLUMN embedding_1024 vector(1024);
ALTER TABLE code_embeddings ADD COLUMN embedding_1536 vector(1536);

-- Backfill each row into the column matching its recorded dim. The cast asserts the
-- stored vector's length matches embedding_dim; a mismatch aborts the migration.
UPDATE code_embeddings SET embedding_768  = embedding::vector(768)  WHERE embedding_dim = 768;
UPDATE code_embeddings SET embedding_1024 = embedding::vector(1024) WHERE embedding_dim = 1024;
UPDATE code_embeddings SET embedding_1536 = embedding::vector(1536) WHERE embedding_dim = 1536;

-- Partial cosine HNSW index per dim. `vector_cosine_ops` matches the `<=>` cosine
-- query operator; the `IS NOT NULL` predicate matches the search WHERE clause so the
-- planner can use the partial index. Built while statement_timeout is lifted.
CREATE INDEX idx_code_embeddings_hnsw_768
    ON code_embeddings USING hnsw (embedding_768 vector_cosine_ops)
    WHERE embedding_768 IS NOT NULL;
CREATE INDEX idx_code_embeddings_hnsw_1024
    ON code_embeddings USING hnsw (embedding_1024 vector_cosine_ops)
    WHERE embedding_1024 IS NOT NULL;
CREATE INDEX idx_code_embeddings_hnsw_1536
    ON code_embeddings USING hnsw (embedding_1536 vector_cosine_ops)
    WHERE embedding_1536 IS NOT NULL;

-- The typeless column is fully superseded by the typed columns above.
ALTER TABLE code_embeddings DROP COLUMN embedding;
