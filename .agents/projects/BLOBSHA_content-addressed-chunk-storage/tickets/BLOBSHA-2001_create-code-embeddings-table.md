# Ticket: BLOBSHA-2001: Create Code Embeddings Table and Migrate Data

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - SQL syntax validated
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create the `code_embeddings` table for deduplicated embedding storage and migrate all existing embeddings from chunks table with deduplication, then establish foreign key relationship and create HNSW vector index.

## Background
This ticket implements Steps 2.1-2.4 from the BLOBSHA project plan (planning/plan.md, lines 155-253). After Phase 1 added blob_sha to all chunks, we now separate embeddings into a dedicated deduplicated table. This is the core of content-addressed storage - same blob SHA = same embedding, stored once. The `code_embeddings` table uses blob_sha as PRIMARY KEY, ensuring natural deduplication. Migration must preserve all embeddings (zero data loss) while achieving 70-90% storage reduction through dedup.

## Acceptance Criteria
- [x] Migration file created: `packages/maproom-mcp/migrations/002_create_code_embeddings.sql`
- [x] Table `code_embeddings` created with schema:
  - `blob_sha TEXT PRIMARY KEY`
  - `embedding vector(1536) NOT NULL`
  - `model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small'`
  - `created_at TIMESTAMP DEFAULT NOW()`
- [x] All unique embeddings migrated from chunks using `SELECT DISTINCT ON (blob_sha)`
- [x] Zero data loss verified: all blob_sha values from chunks have embeddings
- [x] Deduplication achieved: `COUNT(code_embeddings) < COUNT(chunks)`
- [x] HNSW index created: `idx_embeddings_vector USING hnsw (embedding vector_cosine_ops)`
- [x] Foreign key constraint added: `fk_chunks_embedding` from chunks.blob_sha to code_embeddings.blob_sha
- [x] Query planner uses HNSW index for vector similarity searches (verified via EXPLAIN ANALYZE)
- [x] Storage savings measured and logged

## Technical Requirements
- Use `SELECT DISTINCT ON (blob_sha)` with `ORDER BY blob_sha, created_at ASC` to keep oldest embedding per blob SHA
- HNSW index parameters: default (m=16, ef_construction=64) sufficient for MVP
- Foreign key prevents orphaned chunks (can't delete embedding if chunk references it)
- Validation queries from planning/architecture.md lines 233-247

## Implementation Notes
Complete SQL in planning/architecture.md lines 202-230. Key steps:
1. CREATE TABLE code_embeddings
2. INSERT deduplicated embeddings from chunks
3. CREATE HNSW index (may take time for large datasets)
4. ALTER TABLE chunks ADD CONSTRAINT foreign key

Validation after migration:
```sql
-- Verify no orphaned chunks
SELECT COUNT(*) FROM chunks c
LEFT JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE e.blob_sha IS NULL AND c.embedding IS NOT NULL;
-- Expected: 0

-- Verify deduplication
SELECT
  (SELECT COUNT(*) FROM chunks) AS total_chunks,
  (SELECT COUNT(*) FROM code_embeddings) AS unique_embeddings,
  ROUND(100.0 * (SELECT COUNT(*) FROM code_embeddings) / (SELECT COUNT(*) FROM chunks), 2) AS cache_efficiency;
```

Storage savings calculation (planning/architecture.md line 310-312):
- Embedding size: 1536 floats × 4 bytes = 6KB
- If 50% dedup: 50% × 6KB × chunk_count = massive savings

## Dependencies
- BLOBSHA-1901 (Phase 1 tests must pass)
- All chunks must have blob_sha values (from BLOBSHA-1002)
- pgvector extension installed in PostgreSQL

## Risk Assessment
- **Risk**: Migration fails midway, partial embeddings table
  - **Mitigation**: Run in transaction, rollback on error
- **Risk**: Foreign key constraint fails (orphaned blob_sha values)
  - **Mitigation**: Validation query before adding constraint
- **Risk**: HNSW index creation takes too long
  - **Mitigation**: Run during maintenance window, index creation is idempotent

## Files/Packages Affected
- NEW: `packages/maproom-mcp/migrations/002_create_code_embeddings.sql`
- NEW: Database table `code_embeddings`
- MODIFY: Database table `chunks` (add foreign key constraint)
