# Ticket: BLOBSHA-1002: Database Migration - Add Blob SHA Column

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create database migration 001 that adds the `blob_sha` column to the chunks table, creates PostgreSQL function for blob SHA computation, and backfills all existing chunks with their blob SHA values.

## Background
This ticket implements Steps 1.2-1.4 from the BLOBSHA project plan (planning/plan.md, lines 53-136). After Rust implements blob SHA computation (BLOBSHA-1001), we need the database schema to store these values. The migration must be non-blocking (use CONCURRENTLY for indexes), handle large tables efficiently (batch updates), and validate results. The PostgreSQL function must produce identical output to the Rust implementation to ensure consistency.

## Acceptance Criteria
- [ ] Migration file created: `packages/maproom-mcp/migrations/001_add_blob_sha.sql`
- [ ] PostgreSQL function `compute_git_blob_sha(content TEXT)` implemented and returns TEXT
- [ ] Function marked as IMMUTABLE for query optimization
- [ ] Function output matches Rust `compute_blob_sha()` for identical input (verified via integration test)
- [ ] `blob_sha` column added to chunks table (nullable initially)
- [ ] Index `idx_chunks_blob_sha` created using CREATE INDEX CONCURRENTLY (non-blocking)
- [ ] All existing chunks backfilled with blob SHA values using batched updates
- [ ] Column made NOT NULL after backfill completes
- [ ] Validation query confirms zero NULL values
- [ ] Deduplication metrics calculated and logged (total chunks vs unique blob_sha count)

## Technical Requirements
- PostgreSQL function format: `encode(digest('blob ' || length(content) || E'\0' || content, 'sha256'), 'hex')`
- Batch size for backfill: 1000 chunks per transaction
- Use DO $$ block for batched processing with progress logging
- Index creation with CONCURRENTLY flag to avoid table locks
- Validation queries from planning/architecture.md lines 184-196

## Implementation Notes
Complete SQL code available in planning/architecture.md lines 148-181 (migration) and lines 122-133 (PostgreSQL function).

The batched backfill prevents long-running transactions that could block the table. Progress should be logged with RAISE NOTICE every 1000 rows.

After backfill, run validation:
```sql
SELECT COUNT(*) AS total_chunks,
  COUNT(DISTINCT blob_sha) AS unique_blobs,
  ROUND(100.0 * (COUNT(*) - COUNT(DISTINCT blob_sha)) / COUNT(*), 2) AS dedup_pct
FROM chunks;
```

This shows deduplication potential before creating the code_embeddings table.

**PostgreSQL Function Structure** (from planning/architecture.md lines 122-133):
```sql
CREATE OR REPLACE FUNCTION compute_git_blob_sha(content TEXT)
RETURNS TEXT AS $$
  SELECT encode(
    digest(
      'blob ' || length(content) || E'\0' || content,
      'sha256'
    ),
    'hex'
  );
$$ LANGUAGE SQL IMMUTABLE;
```

**Migration Structure** (from planning/architecture.md lines 148-181):
1. Add nullable column
2. Create index CONCURRENTLY
3. Backfill in batches with progress logging
4. Make NOT NULL after completion
5. Run validation queries

## Dependencies
- BLOBSHA-1001 (Rust blob SHA function must exist for comparison testing)
- Database backup taken before migration execution
- PostgreSQL pgcrypto extension (for digest function)

## Risk Assessment
- **Risk**: Long-running migration blocks production queries
  - **Mitigation**: CREATE INDEX CONCURRENTLY, batched updates with commits, run during maintenance window
- **Risk**: Backfill fails midway, partial state
  - **Mitigation**: Batched with explicit commits allows resume from last successful batch
- **Risk**: PostgreSQL function differs from Rust implementation
  - **Mitigation**: Integration test verifies byte-for-byte identical output

## Files/Packages Affected
- NEW: `packages/maproom-mcp/migrations/001_add_blob_sha.sql`
- NEW: `packages/maproom-mcp/tests/blob-sha-migration.test.ts` (integration test for Rust/SQL compatibility)
- MODIFY: Database schema - chunks table gets blob_sha column and index
