# Ticket: BLOBSHA-2002: Execute Phase 2 Test Suite

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - integration tests created and validated
- [x] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute comprehensive test suite for Phase 2 (Code Embeddings Table) to verify migration success, deduplication, foreign key integrity, and vector index functionality. Report all test results without modifying code.

## Background
This ticket implements Step 2.5 from the BLOBSHA project plan (planning/plan.md, lines 255-279). After creating code_embeddings table and migrating data (BLOBSHA-2001), we must verify zero data loss, deduplication correctness, and index performance before proceeding to Phase 3 application integration.

This is a critical validation checkpoint that ensures the Phase 2 migration has successfully:
- Extracted unique embeddings from chunks table
- Maintained referential integrity through foreign keys
- Achieved expected deduplication rates (70-90% for typical codebases)
- Enabled performant vector search via HNSW index

## Acceptance Criteria
- [x] Integration test passes: `test_migration_002_deduplication`
  - Verifies unique embeddings extracted correctly
  - Confirms deduplication percentage matches expectations (70-90% for typical codebases)
- [x] Integration test passes: `test_no_embedding_loss`
  - All blob_sha values from chunks have corresponding embeddings
  - Zero orphaned chunks (LEFT JOIN returns no NULLs)
- [x] Integration test passes: `test_foreign_key_constraint`
  - Foreign key prevents deletion of embeddings still referenced
  - Constraint enforces referential integrity
- [x] Validation queries executed successfully:
  - Storage savings measured (chunks size vs embeddings size)
  - Cache efficiency calculated (unique embeddings / total chunks)
- [x] EXPLAIN ANALYZE confirms HNSW index used for vector queries
- [x] Test report generated with pass/fail status and performance metrics

## Technical Requirements
- Execute via: `cd packages/maproom-mcp && npm test migration-002.test.ts`
- Validation queries from planning/plan.md lines 263-274:
  ```sql
  -- Verify storage savings
  SELECT
    pg_size_pretty(pg_total_relation_size('chunks')) AS chunks_size,
    pg_size_pretty(pg_total_relation_size('code_embeddings')) AS embeddings_size;

  -- Verify all embeddings accessible
  SELECT
    (SELECT COUNT(*) FROM chunks WHERE embedding IS NOT NULL) AS chunks_with_embeddings,
    (SELECT COUNT(*) FROM code_embeddings) AS unique_embeddings;
  ```
- Run EXPLAIN ANALYZE on vector search query to verify index usage
- Generate metrics: dedup percentage, storage savings, query performance

## Implementation Notes
The unit-test-runner agent should NOT modify any code. If tests fail:
1. Report which specific tests failed
2. Include validation query results
3. Show EXPLAIN ANALYZE output if index not used
4. Return to general-purpose agent for fixes

Success criteria from planning/plan.md lines 276-279:
- All Phase 2 tests passing
- Migration verified on test database
- Storage savings measured (should see 50%+ reduction)

Expected deduplication rates:
- Initial scan of single branch: 0-10% (few duplicates)
- Multi-branch codebase: 70-90% (high overlap)
- After refactoring: 20-40% (moved functions)

Reference test implementations in planning/quality-strategy.md lines 270-327 (migration tests).

## Dependencies
- BLOBSHA-2001 (code_embeddings table created and migrated)
- BLOBSHA-1901 (Phase 1 tests passed)
- Test database with Phase 2 migration applied

## Risk Assessment
- **Risk**: Tests pass but embeddings are incorrect (silent data corruption)
  - **Mitigation**: Spot-check embeddings match original chunks using blob_sha hashing verification
- **Risk**: Index not used despite creation
  - **Mitigation**: EXPLAIN ANALYZE test fails if index scan not present, forcing investigation before Phase 3
- **Risk**: Deduplication rate lower than expected indicates migration logic error
  - **Mitigation**: Test includes bounds checking on dedup percentage with clear failure messages

## Files/Packages Affected
- READ: `packages/maproom-mcp/tests/migration-002.test.ts` (integration tests)
- READ: `packages/maproom-mcp/migrations/002_create_code_embeddings.sql` (migration to test)
- READ: Test database (for validation queries)
