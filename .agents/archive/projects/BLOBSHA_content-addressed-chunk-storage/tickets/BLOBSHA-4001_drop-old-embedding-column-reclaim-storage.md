# Ticket: BLOBSHA-4001: Drop Old Embedding Column and Reclaim Storage

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Drop the deprecated `embedding` column from chunks table and run VACUUM FULL to reclaim disk space after verifying all queries work with the new JOIN-based approach.

## Background
This ticket implements Step 4.1 from the BLOBSHA project plan (planning/plan.md, lines 468-491). After Phase 3 successfully migrated all queries to use code_embeddings (BLOBSHA-3001, BLOBSHA-3002, BLOBSHA-3901), the old embedding column in chunks is no longer needed. Dropping it reclaims significant storage - embeddings are 6KB each, so for 100k chunks that's 600MB of wasted space. This is the final irreversible schema change.

## Acceptance Criteria
- [ ] All Phase 3 tests passing (prerequisite verification)
- [ ] Manual verification completed: queries work without chunks.embedding column
- [ ] Database backup taken with timestamp: `backup_before_drop_embedding_YYYYMMDD.sql`
- [ ] Migration file created: `packages/maproom-mcp/migrations/003_drop_old_embedding.sql`
- [ ] Column dropped successfully: `ALTER TABLE chunks DROP COLUMN embedding`
- [ ] VACUUM FULL executed: `VACUUM FULL chunks` completes successfully
- [ ] Disk space reclaimed (measured before/after with pg_total_relation_size)
- [ ] All existing queries still work after column drop
- [ ] No references to chunks.embedding remain in codebase (verified via grep)

## Technical Requirements
- Prerequisites before execution:
  - All Phase 3 tests must pass
  - Manual testing of search/upsert operations
  - Full database backup taken
- Migration SQL from planning/architecture.md lines 480-483:
  ```sql
  ALTER TABLE chunks DROP COLUMN embedding;
  VACUUM FULL chunks;
  ```
- Storage recalculation:
  - Before: chunks table size includes 6KB per chunk for embeddings
  - After: chunks table reduced by embedding column size
  - Expected savings: 50%+ of chunks table size if 50% dedup

## Implementation Notes
**CRITICAL**: This is an irreversible operation. Cannot easily restore the embedding column after drop.

Before execution checklist:
1. Verify all Phase 3 tests pass
2. Manually test search operations
3. Take full database backup: `pg_dump maproom > backup_before_drop_embedding_$(date +%Y%m%d).sql`
4. Verify backup can be restored

Measure savings:
```sql
-- Before drop
SELECT pg_size_pretty(pg_total_relation_size('chunks')) AS before_size;

-- After VACUUM
SELECT pg_size_pretty(pg_total_relation_size('chunks')) AS after_size;
```

Expected savings from planning/architecture.md line 310-312:
- Embedding size: 1536 floats × 4 bytes = 6KB per embedding
- If 100k chunks: 600MB savings
- If 50% dedup: actual savings varies based on dedup rate

VACUUM FULL requires exclusive lock on table - run during maintenance window.

## Dependencies
- BLOBSHA-3901 (all Phase 3 tests passed)
- All search queries using JOIN with code_embeddings (verified)
- Foreign key constraint in place (prevents orphaned chunks)

## Risk Assessment
- **Risk**: Queries fail after column drop (missed a reference)
  - **Mitigation**: Grep for all `chunks.embedding` before execution, test queries manually
- **Risk**: Cannot restore if issues discovered later
  - **Mitigation**: Full backup before operation, tested restore process
- **Risk**: VACUUM FULL takes too long, blocks production
  - **Mitigation**: Run during maintenance window, monitor duration

## Rollback Plan
If column drop causes issues:
1. Restore from backup: `psql maproom < backup_before_drop_embedding_YYYYMMDD.sql`
2. Fix any missed query references
3. Re-run Phase 4 after fixes

## Files/Packages Affected
- NEW: `packages/maproom-mcp/migrations/003_drop_old_embedding.sql`
- MODIFY: Database table `chunks` (drop embedding column)
- VERIFY: All files in crates/maproom/src/ and packages/maproom-mcp/src/ have no chunks.embedding references
