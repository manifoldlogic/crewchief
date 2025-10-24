# Ticket: PERF_OPT-2002: Query Tuning

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- performance-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Tune database queries using EXPLAIN ANALYZE, rewrite slow queries for better performance, create materialized views for expensive joins, and update table statistics for optimal query planning.

## Background
After optimizing indices in PERF_OPT-2001, we need to optimize the queries themselves. PERF_OPT_ANALYSIS.md (lines 28-32) identifies query optimization as a key area where materialized views can cache expensive queries and proper statistics updates improve query planning.

The architecture document (PERF_OPT_ARCHITECTURE.md lines 22-40) provides a materialized view strategy for expensive joins that should significantly improve search performance.

## Acceptance Criteria
- [ ] All queries analyzed with EXPLAIN ANALYZE and documented
- [ ] Slow queries rewritten for better performance
- [ ] Materialized views created for expensive joins
- [ ] Table statistics updated with ANALYZE
- [ ] Query time reduced by 50%+ compared to baseline from PERF_OPT-1002
- [ ] No sequential scans on large tables
- [ ] Query plans verified to use indices from PERF_OPT-2001

## Technical Requirements

### EXPLAIN ANALYZE All Queries
Run EXPLAIN ANALYZE on all critical queries:
- Search queries (FTS, vector, graph)
- File lookup queries
- Chunk retrieval queries
- Edge traversal queries
- Context assembly queries
- Statistics computation queries

Document for each query:
- Current execution time
- Query plan (seq scan vs index scan)
- Rows examined vs rows returned
- Buffer hits vs reads
- Cost estimates

### Query Rewrites
Rewrite queries to:
- Use indices effectively
- Avoid subqueries where joins are better
- Eliminate redundant JOINs
- Push down filters (WHERE before JOIN)
- Use EXISTS instead of IN for large sets
- Avoid SELECT * when specific columns needed
- Use CTEs for complex queries

### Materialized Views
Implement materialized view strategy (PERF_OPT_ARCHITECTURE.md lines 24-40):

```sql
CREATE MATERIALIZED VIEW maproom.chunk_search_view AS
SELECT
  c.*,
  f.relpath,
  f.repo_id,
  f.worktree_id,
  COUNT(e1.src_chunk_id) as importance
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
LEFT JOIN maproom.chunk_edges e1 ON e1.dst_chunk_id = c.id
GROUP BY c.id, f.relpath, f.repo_id, f.worktree_id;

-- Index the view
CREATE INDEX idx_search_view_repo ON maproom.chunk_search_view(repo_id);

-- Refresh strategy
REFRESH MATERIALIZED VIEW CONCURRENTLY maproom.chunk_search_view;
```

Benefits:
- Pre-computes expensive joins
- Caches importance scores
- Enables faster search queries
- Concurrent refresh doesn't block reads

### Statistics Updates
Update table statistics (PERF_OPT_PLAN.md line 44):
```sql
ANALYZE maproom.chunks;
ANALYZE maproom.files;
ANALYZE maproom.chunk_edges;
ANALYZE maproom.repos;
ANALYZE maproom.worktrees;
```

Consider targeted statistics:
```sql
ALTER TABLE maproom.chunks ALTER COLUMN symbol_name SET STATISTICS 1000;
```

### Query Optimization Checklist
- [ ] Use prepared statements to cache query plans
- [ ] Batch similar queries together
- [ ] Avoid N+1 query patterns
- [ ] Use appropriate JOIN types (INNER vs LEFT)
- [ ] Filter early in WHERE clauses
- [ ] Use LIMIT for pagination
- [ ] Avoid DISTINCT when unnecessary
- [ ] Use COUNT(*) not COUNT(column)

## Implementation Notes

### Database Configuration
Tune PostgreSQL settings (PERF_OPT_ARCHITECTURE.md lines 195-198):
```yaml
database:
  pool_size: 20
  statement_timeout: 5000
  work_mem: "256MB"
```

Consider:
- `shared_buffers`: 25% of RAM
- `effective_cache_size`: 50% of RAM
- `work_mem`: Per-operation memory (256MB recommended)
- `maintenance_work_mem`: For VACUUM, CREATE INDEX
- `random_page_cost`: Lower for SSDs (1.1)

### Query Monitoring
Add query logging to identify slow queries:
```sql
SET log_min_duration_statement = 100;  -- Log queries > 100ms
```

Track query statistics:
```sql
SELECT query, calls, total_time, mean_time, max_time
FROM pg_stat_statements
WHERE query LIKE '%maproom%'
ORDER BY mean_time DESC
LIMIT 20;
```

### Materialized View Refresh Strategy
Create refresh function:
```sql
CREATE OR REPLACE FUNCTION maproom.refresh_search_view()
RETURNS void AS $$
BEGIN
  REFRESH MATERIALIZED VIEW CONCURRENTLY maproom.chunk_search_view;
END;
$$ LANGUAGE plpgsql;
```

Schedule refresh:
- After bulk indexing operations
- Periodically (e.g., every hour)
- On-demand via CLI command

### Testing
Compare query performance:
1. Baseline from PERF_OPT-1002
2. After index optimization (PERF_OPT-2001)
3. After query tuning (this ticket)

Verify 50%+ improvement in query time.

## Dependencies
- **PERF_OPT-1002** - Requires bottleneck analysis to identify slow queries
- **PERF_OPT-2001** - Requires optimized indices for queries to use
- PostgreSQL 9.4+ for REFRESH MATERIALIZED VIEW CONCURRENTLY
- pg_stat_statements extension for query statistics

## Risk Assessment
- **Risk**: Materialized views may become stale
  - **Mitigation**: Implement automatic refresh strategy, document refresh triggers
- **Risk**: Query rewrites may change result semantics
  - **Mitigation**: Comprehensive testing, verify results match original queries
- **Risk**: work_mem settings may cause OOM
  - **Mitigation**: Conservative settings, monitor memory usage
- **Risk**: Concurrent refresh may fail if unique index doesn't exist
  - **Mitigation**: Create unique indices on materialized views

## Files/Packages Affected
- `crates/maproom/migrations/YYYYMMDDHHMMSS_query_tuning.sql` - New migration for materialized views
- `crates/maproom/src/database/queries.rs` - Rewritten queries
- `crates/maproom/src/database/materialized_views.rs` - New module for view management
- `docs/QUERY_OPTIMIZATION.md` - New documentation of query patterns
- `scripts/refresh-views.sql` - View refresh script
- `scripts/analyze-queries.sql` - Query analysis script
