# Ticket: CONTEXT_ASM-3001: Query Optimization

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Optimize database query performance for Context Assembly Engine (CONTEXT_ASM) by creating materialized views, optimizing recursive CTEs, adding strategic indices, and profiling slow queries.

## Background
The Context Assembly Engine uses recursive CTEs for graph traversal to find related code chunks. As the codebase grows and relationship graphs become more complex, these queries can become expensive. Phase 3 focuses on performance optimization to ensure the system scales efficiently. This ticket implements database-level optimizations including materialized views for precomputed relationships, strategic indexing, and query profiling to identify bottlenecks.

## Acceptance Criteria
- [ ] Materialized views created for precomputed test links
- [ ] Recursive CTEs optimized for graph traversal performance
- [ ] Strategic indices added (at minimum: test_links(target_chunk_id))
- [ ] Slow queries profiled using EXPLAIN ANALYZE with documented results
- [ ] Bidirectional edge traversal optimized in recursive CTE
- [ ] Query performance improvement measured and documented (target: 50%+ reduction in execution time)

## Technical Requirements
- Optimize the recursive CTE for graph traversal (see CONTEXT_ASM_ARCHITECTURE.md lines 35-62)
- Create materialized view for precomputed test links with proper refresh strategy
- Add index on `test_links(target_chunk_id)` as specified in architecture (line 209)
- Profile queries with `EXPLAIN ANALYZE` and document results
- Optimize bidirectional edge traversal in the recursive CTE join condition
- Ensure migrations are reversible with proper DOWN migrations
- Add query performance tests to verify improvements
- Consider additional indices for common access patterns (chunk_edges, relationship types)

## Implementation Notes

### Recursive CTE Optimization
The current recursive CTE (lines 35-62 of CONTEXT_ASM_ARCHITECTURE.md) performs bidirectional graph traversal:
```sql
JOIN maproom.chunk_edges e ON (
  e.src_chunk_id = r.id OR e.dst_chunk_id = r.id
)
```

Optimization strategies:
- Split OR condition into UNION ALL for better index usage
- Add work table size limits to prevent runaway recursion
- Consider separate CTEs for forward/backward traversal
- Add CTE materialization hints if needed

### Materialized View Design
Create materialized view for test links:
```sql
CREATE MATERIALIZED VIEW maproom.test_links_mv AS
SELECT target_chunk_id, test_chunk_id, relationship_type
FROM maproom.test_links;
```

Include refresh strategy:
- CONCURRENTLY for non-blocking refreshes
- Trigger-based or scheduled refresh
- Consider partial refresh for incremental updates

### Index Strategy
Minimum required indices:
- `CREATE INDEX idx_test_links_target ON maproom.test_links(target_chunk_id);`

Additional strategic indices to consider:
- `chunk_edges(src_chunk_id, dst_chunk_id)` - for graph traversal
- `chunk_edges(dst_chunk_id, src_chunk_id)` - for bidirectional lookup
- `chunks(importance_score)` - for priority ranking
- Composite indices based on common query patterns

### Profiling Approach
1. Identify slow queries from production/test usage
2. Run `EXPLAIN ANALYZE` on each query
3. Document baseline performance metrics
4. Apply optimizations incrementally
5. Measure improvement after each change
6. Document findings in `/workspace/crates/maproom/docs/context_query_optimization.md`

### File Locations
- Migration: `crates/maproom/migrations/XXX_context_optimizations.sql`
- Query updates: `crates/maproom/src/context/graph.rs`
- Documentation: `crates/maproom/docs/context_query_optimization.md`

## Dependencies
- **CONTEXT_ASM-1002** (relationship-queries) - Must be completed first to have queries to optimize
- PostgreSQL 12+ with support for recursive CTEs and materialized views
- Database write access for creating indices and materialized views

## Risk Assessment
- **Risk**: Index creation may lock tables temporarily on large databases
  - **Mitigation**: Use `CREATE INDEX CONCURRENTLY` for non-blocking index creation

- **Risk**: Materialized view refresh may consume significant resources
  - **Mitigation**: Schedule refreshes during low-traffic periods, use CONCURRENTLY option

- **Risk**: Query plan changes may regress performance in edge cases
  - **Mitigation**: Comprehensive benchmark suite covering common and edge cases; keep baseline metrics for comparison

- **Risk**: Recursive CTE optimization may change result ordering or completeness
  - **Mitigation**: Thorough testing with known graph structures; validate result sets match pre-optimization behavior

## Files/Packages Affected
- `crates/maproom/migrations/XXX_context_optimizations.sql` - New migration file for indices and materialized views
- `crates/maproom/src/context/graph.rs` - Updated to use optimized queries and materialized views
- `crates/maproom/docs/context_query_optimization.md` - New documentation file with EXPLAIN ANALYZE results
- `crates/maproom/src/context/mod.rs` - May need updates for materialized view refresh logic
- `crates/maproom/tests/context_performance.rs` - New or updated performance test suite
