# Ticket: BRANCHX-1003: Test worktree tracking schema and JSONB queries

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Verify migration 004 succeeded and test JSONB query patterns for worktree filtering. This ensures the schema foundation is solid before building git integration and incremental update logic.

## Background
This is Phase 1, Step 1.3 of BRANCHX. After implementing the schema changes in BRANCHX-1001 (worktree_ids column) and BRANCHX-1002 (worktree_index_state table), we need comprehensive tests to verify:
- The migration succeeded and schema is correct
- JSONB query operators work correctly for worktree filtering
- The GIN index is created and used for performance
- Edge cases are handled (empty arrays, single worktrees, many worktrees)

Reference: `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 1.3

## Acceptance Criteria
- [x] Migration 004 test passes (schema created correctly)
- [x] Test JSONB contains query (`worktree_ids ? '2'`)
- [x] Test JSONB overlaps query (`worktree_ids ?| ARRAY['1', '3']`)
- [x] Test worktree_ids has no duplicates after multiple upserts
- [x] Test removing worktree from array (`worktree_ids - '2'`)
- [x] Test GIN index exists and is used (EXPLAIN output verification)
- [x] All tests pass in CI

## Technical Requirements
- Tests must run against test database with migration 004 applied
- Use actual JSONB operators (not mocked)
- Verify GIN index is created and used (check EXPLAIN output)
- Test edge cases: empty array, single worktree, many worktrees
- Test migration rollback works
- Use existing test infrastructure in `packages/maproom-mcp/tests/`

## Implementation Notes

### Test Files

**Primary test file**: `packages/maproom-mcp/tests/jsonb-queries.test.ts`

Key test cases from `quality-strategy.md`:
1. `test_jsonb_contains_query` - Single worktree filter using `?` operator
2. `test_jsonb_overlaps_query` - Multiple worktree filter using `?|` operator
3. `test_worktree_ids_no_duplicates` - Idempotency (same worktree added multiple times)
4. `test_remove_worktree` - Array element removal using `-` operator

**Migration test file**: `packages/maproom-mcp/tests/migrations/004-worktree-tracking.test.ts`

Tests:
- Schema verification (column exists, type is JSONB, default value)
- GIN index exists
- worktree_index_state table structure
- Migration rollback

### Example Test Structure

```typescript
describe('JSONB worktree_ids queries', () => {
  beforeEach(async () => {
    // Setup test database with migration 004
    await runMigration('004_add_worktree_tracking.sql');
  });

  it('detects chunk in worktree', async () => {
    await createChunk({ worktree_ids: [1, 2, 3] });

    const result = await pool.query(
      "SELECT * FROM chunks WHERE worktree_ids ? '2'"
    );

    expect(result.rows).toHaveLength(1);
  });

  it('verifies GIN index is used', async () => {
    const explain = await pool.query(
      "EXPLAIN SELECT * FROM chunks WHERE worktree_ids ? '2'"
    );

    expect(explain.rows.some(r =>
      r['QUERY PLAN'].includes('Index Scan using idx_chunks_worktree_ids')
    )).toBe(true);
  });
});
```

### Key Test Scenarios

**JSONB Operators**:
- `?` (contains) - Check if array contains specific worktree ID
- `?|` (overlaps) - Check if array contains any of given worktree IDs
- `-` (remove) - Remove worktree ID from array
- No duplicates after multiple appends

**Edge Cases**:
- Empty worktree_ids array `[]`
- Single worktree `[1]`
- Many worktrees (10+ in array)
- Non-existent worktree ID queries return no results

**Performance**:
- EXPLAIN ANALYZE shows GIN index usage
- Query performance acceptable with 10k+ chunks

## Dependencies
- BRANCHX-1001 complete (worktree_ids column exists)
- BRANCHX-1002 complete (worktree_index_state table exists)
- Migration 004 applied to test database
- Test database infrastructure in `packages/maproom-mcp/tests/`

## Risk Assessment
- **Risk**: Tests pass but GIN index not used (slow queries at scale)
  - **Mitigation**: Include EXPLAIN ANALYZE tests to verify index usage in query plans
- **Risk**: JSONB queries work in tests but fail at scale (>10k chunks)
  - **Mitigation**: Include performance test with 10k+ chunks, measure query time
- **Risk**: Migration rollback not tested, could fail in production
  - **Mitigation**: Test both migration and rollback paths

## Files/Packages Affected
- `packages/maproom-mcp/tests/jsonb-queries.test.ts` (new)
- `packages/maproom-mcp/tests/migrations/004-worktree-tracking.test.ts` (new)
- `packages/maproom-mcp/tests/test-helpers.ts` (may need helper functions)
