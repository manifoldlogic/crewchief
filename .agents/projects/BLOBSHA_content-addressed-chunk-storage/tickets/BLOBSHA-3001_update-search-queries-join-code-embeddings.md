# Ticket: BLOBSHA-3001: Update Search Queries to JOIN code_embeddings

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update all vector search queries in Rust and TypeScript to JOIN chunks with code_embeddings table instead of accessing embeddings directly from chunks. Verify query equivalence and performance.

## Background
This ticket implements Step 3.1 from the BLOBSHA project plan (planning/plan.md, lines 293-329). After Phase 2 created the code_embeddings table, application code must switch to the new schema. Queries currently access chunks.embedding directly - this must change to JOIN chunks with code_embeddings on blob_sha. The JOIN uses indexed keys (PRIMARY KEY → FOREIGN KEY) so performance should be equal or better than current queries.

## Acceptance Criteria
- [ ] All search queries in `crates/maproom/src/search.rs` updated to JOIN code_embeddings
- [ ] All search queries in `packages/maproom-mcp/src/search.ts` updated to JOIN code_embeddings
- [ ] EXPLAIN ANALYZE shows efficient query plan:
  - Index scan on code_embeddings (HNSW index)
  - Index nested loop join on chunks.blob_sha
  - Total query time within 10% of baseline
- [ ] Query results identical to old implementation (verified via integration test)
- [ ] All vector similarity searches use pattern: `e.embedding <=> $1` where e is code_embeddings alias
- [ ] No queries still reference chunks.embedding column

## Technical Requirements
- JOIN pattern from planning/architecture.md lines 272-288:
  ```rust
  SELECT c.chunk_id, c.content, e.embedding
  FROM chunks c
  JOIN code_embeddings e ON c.blob_sha = e.blob_sha
  WHERE e.embedding <=> $1 < 0.5
  ORDER BY e.embedding <=> $1
  LIMIT 10
  ```
- Use table aliases: `c` for chunks, `e` for code_embeddings
- Preserve all existing WHERE clauses and ORDER BY logic
- Maintain backward compatibility during transition (queries work with or without old embedding column)

## Implementation Notes
Files to update (planning/architecture.md line 290-294):
1. `crates/maproom/src/search.rs` - Core Rust search implementation
2. `crates/maproom/src/upsert.rs` - Will be updated in BLOBSHA-3002
3. `packages/maproom-mcp/src/search.ts` - MCP server search handler

Before/after examples in planning/architecture.md lines 253-288.

Performance considerations (planning/architecture.md lines 332-336):
- HNSW index on code_embeddings is smaller (fewer unique embeddings)
- JOIN overhead is minimal (indexed PRIMARY KEY → FOREIGN KEY)
- Overall performance equal or better than baseline

Benchmark query from planning/architecture.md lines 340-349 to verify performance.

## Dependencies
- BLOBSHA-2002 (Phase 2 tests passed, code_embeddings table exists)
- chunks.embedding column still exists (for backward compat during transition)

## Risk Assessment
- **Risk**: Query performance regression
  - **Mitigation**: EXPLAIN ANALYZE before/after comparison, rollback if >10% slower
- **Risk**: Results differ from baseline (correctness issue)
  - **Mitigation**: Integration test compares old vs new query results
- **Risk**: Miss updating a query, leaving chunks.embedding references
  - **Mitigation**: Grep for all instances of `chunks.embedding` in codebase

## Files/Packages Affected
- MODIFY: `crates/maproom/src/search.rs`
- MODIFY: `packages/maproom-mcp/src/search.ts`
- NEW: `crates/maproom/tests/search_query_equivalence.rs` (integration test)
