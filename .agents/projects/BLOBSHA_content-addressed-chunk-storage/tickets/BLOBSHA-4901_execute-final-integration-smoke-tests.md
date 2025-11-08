# Ticket: BLOBSHA-4901: Execute Final Integration and Smoke Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute comprehensive final integration tests and manual smoke tests to verify the complete BLOBSHA migration is successful, all success criteria met, and system is production-ready.

## Background
This ticket validates the entire BLOBSHA project against success criteria from planning/plan.md lines 649-677. After completing all 4 phases, we must verify: zero data loss, 70-90% deduplication achieved, query performance within targets, cache metrics working, and all systems operational. This is the final gate before project completion and verify-ticket agent validation.

This ticket references the success criteria and acceptance checklist from the BLOBSHA planning/plan.md document which established the complete validation requirements for the content-addressed chunk storage migration.

## Acceptance Criteria
- [ ] Final integration test passes: `test_end_to_end_migration`
  - Index repository from scratch using new system
  - Verify all chunks have blob_sha
  - Verify embeddings deduplicated in code_embeddings
  - Verify cache hits on re-indexing
- [ ] Manual smoke test checklist complete:
  - [ ] Can index new repository successfully
  - [ ] Can search indexed code and get results
  - [ ] Can reindex same repository (cache hits expected)
  - [ ] Can drop and recreate worktree without errors
- [ ] Success metrics validated:
  - [ ] Blob SHA computed for all chunks (COUNT blob_sha = COUNT chunk_id)
  - [ ] Embeddings deduplicated (COUNT code_embeddings < COUNT chunks)
  - [ ] Cache hit rate measurable (metrics.hit_rate() works)
  - [ ] All queries return correct results
  - [ ] Foreign key integrity enforced
  - [ ] Query latency within 10% of baseline
  - [ ] Cache hit rate 70-90% for typical branch overlap test
  - [ ] Zero data loss verified (all original embeddings accessible)
- [ ] Performance targets met:
  - [ ] Query latency within 10% of baseline
  - [ ] Cache hit rate 70-90% for multi-branch tests
  - [ ] Storage savings 50%+ measured
- [ ] Documentation complete and accurate:
  - [ ] Architecture doc exists and explains blob SHA approach
  - [ ] Migration guide complete with all phases
  - [ ] Changelog updated with breaking changes

## Technical Requirements

### Integration Test Execution
- Execute final integration test: `cd crates/maproom && cargo test test_end_to_end_migration --nocapture`
- Verify test output shows:
  - All chunks indexed with blob_sha
  - Embeddings deduplicated in code_embeddings table
  - Cache hits detected on re-indexing
  - Zero test failures

### Manual Smoke Tests
Manual smoke tests using maproom CLI:
```bash
# Test 1: Index repository
maproom scan --repo /path/to/test-repo --worktree main

# Test 2: Search works
maproom search --query "authentication" --repo /path/to/test-repo

# Test 3: Re-index shows cache hits
maproom scan --repo /path/to/test-repo --worktree main
# Expected: 100% cache hit rate in logs
```

### Validation Queries
Validation queries from planning/plan.md lines 700-713:

1. **Zero NULL blob_sha values**:
   ```sql
   SELECT COUNT(*) FROM chunks WHERE blob_sha IS NULL;
   -- Expected: 0
   ```

2. **Deduplication percentage calculation**:
   ```sql
   SELECT
     COUNT(DISTINCT chunk_id) as total_chunks,
     COUNT(DISTINCT blob_sha) as unique_embeddings,
     (1.0 - COUNT(DISTINCT blob_sha)::float / COUNT(DISTINCT chunk_id)) * 100 as dedup_percentage
   FROM chunks;
   -- Expected: dedup_percentage between 10-30%
   ```

3. **Embedding accessibility check**:
   ```sql
   SELECT COUNT(*) FROM chunks c
   JOIN code_embeddings ce ON c.blob_sha = ce.blob_sha;
   -- Expected: COUNT = total chunks
   ```

4. **Foreign key constraint verification**:
   ```sql
   SELECT COUNT(*) FROM chunks c
   LEFT JOIN code_embeddings ce ON c.blob_sha = ce.blob_sha
   WHERE ce.blob_sha IS NULL;
   -- Expected: 0
   ```

## Implementation Notes

The unit-test-runner agent should execute all tests and manual procedures, then report comprehensive results. This is the final validation before the verify-ticket agent reviews the project.

### Success Criteria from planning/plan.md Acceptance Checklist (lines 700-713):
- All phases complete (1-4) ✓
- All tests passing (unit + integration + E2E) ✓
- Performance benchmarks within targets ✓
- Documentation updated ✓
- Manual smoke test successful ✓
- Cache metrics showing expected behavior ✓
- Deduplication working ✓
- Zero data loss verified ✓

### Manual Smoke Test Checklist from planning/plan.md:
1. Run full scan on test repository
2. Verify cache hit rate on re-scan
3. Check metrics: cache hit rate, chunks processed, embeddings generated, cost
4. Search operations return results
5. Query performance acceptable (<50ms)

### Test Execution Strategy:
1. Run automated integration test first
2. Execute manual smoke tests with real maproom CLI
3. Run validation SQL queries against production database
4. Verify all metrics meet success criteria
5. Document any deviations or unexpected results

### Failure Handling:
If ANY criterion fails, return to appropriate implementation agent for fixes before proceeding to verify-ticket:
- Schema issues → database-engineer
- Query performance issues → database-engineer
- Application integration issues → rust-indexer-engineer
- Test failures → rust-indexer-engineer
- Documentation gaps → technical-writer

## Dependencies
- BLOBSHA-4001 (embedding column dropped)
- BLOBSHA-4002 (documentation complete)
- All Phase 1-3 tests passed
- Test repository available for smoke testing
- Maproom CLI built and accessible
- PostgreSQL database accessible for validation queries

## Risk Assessment
- **Risk**: Integration test passes but production issues remain
  - **Mitigation**: Manual smoke tests cover real-world scenarios, validation queries check actual database state
- **Risk**: Performance degradation not caught by automated tests
  - **Mitigation**: Manual testing includes performance observation, validation queries check query execution
- **Risk**: Cache metrics reporting incorrect values
  - **Mitigation**: Cross-reference automated test results with manual smoke test observations
- **Risk**: Edge cases not covered by test suite
  - **Mitigation**: Smoke tests use diverse operations (index, search, re-index, drop/recreate)

## Files/Packages Affected
- READ: `crates/maproom/tests/end_to_end_migration.rs` (integration test)
- READ: `crates/maproom/src/cli.rs` (for manual smoke tests)
- READ: Production PostgreSQL database (final validation queries)
- READ: `.agents/projects/BLOBSHA_content-addressed-chunk-storage/planning/plan.md` (success criteria reference)
- READ: Cache metrics implementation for validation
- EXECUTE: `cargo test test_end_to_end_migration`
- EXECUTE: `maproom scan`, `maproom search` CLI commands
- EXECUTE: SQL validation queries via psql or database client
