# Ticket: PERF_OPT-2001: Index Optimization

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Optimize PostgreSQL indices for Maproom tables by creating covering indices, partial indices, BRIN indices, and filling gaps in index coverage identified during bottleneck analysis.

## Background
PERF_OPT_ANALYSIS.md (lines 28-32) identifies database tuning as "low-hanging fruit" where proper indices can provide 10x query speedup. Current state has only basic indices, missing opportunities for covering indices, partial indices for hot paths, and BRIN indices for large tables.

The architecture document (PERF_OPT_ARCHITECTURE.md lines 7-20) provides specific index strategies that should be implemented based on common query patterns.

## Acceptance Criteria
- [ ] Covering indices created for common queries to avoid table lookups
- [ ] Partial indices created for frequently accessed subsets (e.g., high recency scores)
- [ ] BRIN indices created for large tables with natural ordering (e.g., timestamps)
- [ ] All missing indices identified in PERF_OPT-1002 are created
- [ ] Index usage verified with EXPLAIN ANALYZE showing index scans instead of sequential scans
- [ ] Query performance improved by 50%+ compared to baseline

## Technical Requirements

### Covering Indices
Implement covering indices (PERF_OPT_ARCHITECTURE.md lines 9-11):
```sql
CREATE INDEX idx_chunks_search ON maproom.chunks(file_id, kind, start_line)
  INCLUDE (symbol_name, preview);
```

Benefits: Avoids heap lookups by including commonly accessed columns

### Partial Indices
Implement partial indices for hot paths (PERF_OPT_ARCHITECTURE.md lines 13-15):
```sql
CREATE INDEX idx_recent_chunks ON maproom.chunks(recency_score)
  WHERE recency_score > 0.7;
```

Benefits: Smaller index size, faster scans for filtered queries

### BRIN Indices
Implement BRIN indices for large tables (PERF_OPT_ARCHITECTURE.md lines 17-19):
```sql
CREATE INDEX idx_files_modified_brin ON maproom.files
  USING BRIN(last_modified);
```

Benefits: Space-efficient for naturally ordered data

### Additional Indices
Based on bottleneck analysis from PERF_OPT-1002:
- Index on `chunks.symbol_name` for symbol lookups
- Composite index on `files(repo_id, worktree_id, relpath)` for file lookups
- Index on `chunk_edges(src_chunk_id)` and `chunk_edges(dst_chunk_id)` for graph traversal
- GIN index on `chunks.symbol_name` for pattern matching if needed

### Index Maintenance
- Create script to rebuild indices: `REINDEX INDEX CONCURRENTLY`
- Set up autovacuum configuration
- Update statistics: `ANALYZE maproom.chunks;`

## Implementation Notes

### Migration Structure
Create migration file: `crates/maproom/migrations/YYYYMMDDHHMMSS_optimize_indices.sql`

Include:
1. Analysis queries to validate current index usage
2. DROP existing inefficient indices
3. CREATE new optimized indices
4. ANALYZE statements to update statistics
5. Validation queries to confirm index usage

### Testing Index Effectiveness
Before/after comparison:
```sql
-- Before index creation
EXPLAIN ANALYZE SELECT ...;

-- After index creation
EXPLAIN ANALYZE SELECT ...;
```

Verify:
- "Index Scan" instead of "Seq Scan"
- Lower cost estimates
- Faster actual execution time
- Higher buffer hit ratio

### Index Monitoring
Add queries to track index usage:
```sql
SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
FROM pg_stat_user_indexes
WHERE schemaname = 'maproom'
ORDER BY idx_scan DESC;
```

### Performance Impact
Indices improve read performance but slow writes:
- Monitor insert/update performance during indexing
- Use CONCURRENTLY for index creation to avoid locks
- Consider index-only scans with VACUUM ANALYZE

## Dependencies
- **PERF_OPT-1002** - Requires bottleneck analysis to identify missing indices
- PostgreSQL 12+ for covering indices (INCLUDE clause)
- Existing database schema and tables

## Risk Assessment
- **Risk**: Index creation may lock tables in production
  - **Mitigation**: Use CREATE INDEX CONCURRENTLY to avoid locks
- **Risk**: Too many indices may slow down writes
  - **Mitigation**: Monitor insert performance, remove unused indices
- **Risk**: Indices may not be used by query planner
  - **Mitigation**: Update statistics with ANALYZE, verify with EXPLAIN ANALYZE
- **Risk**: BRIN indices may not provide enough selectivity
  - **Mitigation**: Test on representative data, fall back to B-tree if needed

## Files/Packages Affected
- `crates/maproom/migrations/YYYYMMDDHHMMSS_optimize_indices.sql` - New migration file
- `docs/DATABASE_INDICES.md` - New documentation of index strategy
- `scripts/analyze-indices.sql` - New index analysis script
- `scripts/monitor-indices.sql` - New index monitoring script
