# Ticket: DAEMIGR-3903: Regression Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- verify-ticket
- unit-test-runner
- commit-ticket

## Summary
Verify no functionality lost from spawning approach by comparing daemon-based results with process-spawning results across all search modes, filters, and edge cases.

## Background
The daemon migration must preserve 100% of existing MCP search functionality. This ticket creates regression tests comparing daemon vs. spawning results to ensure identical behavior for all search scenarios.

**Context:**
- Test file: `/workspace/packages/maproom-mcp/tests/regression.test.ts`
- Comparison: daemon.search() vs. trySpawnWithCandidates()
- Quality strategy: `.crewchief/projects/DAEMIGR_daemon-client-migration/planning/quality-strategy.md` (lines 357-403)

**Related Planning Documents:**
- Quality Strategy: `.crewchief/projects/DAEMIGR_daemon-client-migration/planning/quality-strategy.md`

## Acceptance Criteria
- [ ] Results identical between daemon and spawning approaches:
  - Same chunk IDs returned
  - Same scores (within floating point tolerance)
  - Same result ordering
  - Same metadata (file paths, line numbers, etc.)
- [ ] All existing search scenarios work:
  - All search modes (fts, vector, hybrid)
  - All filters (repo, worktree, file_type)
  - Debug mode output matches
  - Empty results handled identically
  - Large result sets handled identically
- [ ] No performance regressions:
  - Warm requests faster (10-50ms vs 160-400ms)
  - Cold start acceptable (similar or faster)
- [ ] Error messages equivalent or better:
  - Invalid repo returns same error message
  - Invalid query returns same error message
  - Daemon crash returns clear error (better than spawning)

## Technical Requirements

**Comparison test structure:**
```typescript
it('fts search returns identical results', async () => {
  // Spawn approach (old)
  const spawnResults = await searchViaSpawn({
    query: 'authentication',
    repo: 'crewchief',
    mode: 'fts'
  })

  // Daemon approach (new)
  const daemonResults = await searchViaDaemon({
    query: 'authentication',
    repo: 'crewchief',
    mode: 'fts'
  })

  expect(daemonResults.hits).toEqual(spawnResults.hits)
})
```

**Test all search modes:**
- FTS (full-text search with tsvector)
- Vector (semantic search with pgvector)
- Hybrid (combined FTS + vector)

**Test all filters:**
- repo filter (specific repository)
- worktree filter (specific worktree)
- file_type filter (code, docs, config)
- Combination filters (repo + worktree + file_type)

**Edge cases:**
- Empty query returns empty results
- Query with no matches returns empty hits array
- Large result set (limit > results) returns correct truncation
- Debug mode returns detailed output with scores
- Special characters in query are properly escaped

## Implementation Notes

1. Keep old spawning code accessible for comparison (don't delete)
2. Run both approaches on same test data for fair comparison
3. Allow small floating point differences in scores (±0.001)
4. Test on real database with indexed test repository
5. Document any intentional differences (e.g., better error messages)
6. Reference quality-strategy.md lines 357-403 for complete test list

**Test Strategy:**
- Run daemon and spawning approaches in parallel
- Compare results using deep equality checks
- Use tolerance matcher for floating point scores
- Verify metadata fields match exactly
- Test with production-like data volume

## Dependencies
- DAEMIGR-2903 (integration tests pass) - can run in parallel with DAEMIGR-3901, DAEMIGR-3902

## Risk Assessment
- **Risk**: False negatives from floating point precision
  - **Mitigation**: Use tolerance for score comparisons (±0.001)
- **Risk**: Test data changes affecting results
  - **Mitigation**: Use stable test dataset with fixed seed data
- **Risk**: Spawning code removed prematurely
  - **Mitigation**: Keep in utils/process.ts for comparison purposes

## Files/Packages Affected
- **Create**: `/workspace/packages/maproom-mcp/tests/regression.test.ts`
- **Reference**: `/workspace/packages/maproom-mcp/src/utils/process.ts` (old spawning code)
- **Reference**: `.crewchief/projects/DAEMIGR_daemon-client-migration/planning/quality-strategy.md`

## Estimated Effort
1 day (8 hours)

## Phase
3 (Validation)

## Priority
CRITICAL (validates no functionality lost)
