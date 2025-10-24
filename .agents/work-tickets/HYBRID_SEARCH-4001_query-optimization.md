# Ticket: HYBRID_SEARCH-4001: Query Optimization

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
Optimize database query performance through materialized views, query profiling, result caching, and connection pooling to achieve sub-30ms p50 latency targets for hybrid search queries.

## Background
As the hybrid search system moves into Phase 4 (performance optimization), query performance becomes critical for production readiness. The system currently executes multiple complex queries (FTS, vector similarity, graph traversal) that need to be optimized for low-latency production use. This ticket implements the database-level optimizations required to meet the p50 latency target of <30ms.

This work is part of Phase 4, Week 4, Task 1 of the HYBRID_SEARCH project plan and directly builds upon the search query implementations completed in HYBRID_SEARCH-3003.

## Acceptance Criteria
- [ ] Materialized view created for chunk_importance with proper indexing
- [ ] All search-related queries profiled using EXPLAIN ANALYZE with results documented
- [ ] Query result cache implemented with LRU eviction strategy
- [ ] Connection pooling configured and tested for database connections
- [ ] Performance benchmarks show p50 latency <30ms for typical search queries
- [ ] Cache hit rate metrics tracked and logged
- [ ] Documentation updated with optimization strategies and results

## Technical Requirements
- Create materialized view `maproom.chunk_importance` for expensive importance score computations
- Profile all queries with EXPLAIN ANALYZE to identify bottlenecks
- Implement LRU cache for query results using Arc<RwLock<LruCache<>>>
- Set up database connection pooling (r2d2 or deadpool)
- Optimize recursive CTEs for graph traversal queries
- Add index on importance_score column in materialized view
- Track cache statistics (hits, misses, evictions)
- Document all query optimizations with before/after metrics

## Implementation Notes

### Materialized View Implementation
Based on architecture specification (lines 407-425):
```sql
CREATE MATERIALIZED VIEW maproom.chunk_importance AS
SELECT
  c.id,
  COUNT(DISTINCT e1.src_chunk_id) as in_degree,
  COUNT(DISTINCT e2.dst_chunk_id) as out_degree,
  c.recency_score,
  c.churn_score,
  (
    COUNT(DISTINCT e1.src_chunk_id) * 0.4 +
    c.recency_score * 0.3 +
    (1.0 / (1.0 + c.churn_score)) * 0.3
  ) as importance_score
FROM maproom.chunks c
LEFT JOIN maproom.chunk_edges e1 ON e1.dst_chunk_id = c.id
LEFT JOIN maproom.chunk_edges e2 ON e2.src_chunk_id = c.id
GROUP BY c.id, c.recency_score, c.churn_score;

CREATE INDEX idx_importance ON maproom.chunk_importance(importance_score);
```

### Query Result Cache Architecture
Based on architecture specification (lines 345-379):
- Use `Arc<RwLock<LruCache<String, SearchResults>>>` for thread-safe caching
- Implement `get_or_compute` pattern for cache-aside strategy
- Track cache statistics (hits, misses) with atomic counters
- Support separate caches for query results and embeddings
- Configure reasonable cache size limits (to be determined during implementation)

### Connection Pooling
From architecture performance considerations (lines 427-449):
- Use async-compatible connection pool (deadpool-postgres recommended)
- Configure max concurrent queries limit (suggested: 10)
- Set appropriate timeout values (suggested: 100ms)
- Enable connection reuse to reduce overhead

### Query Profiling Process
1. Run EXPLAIN ANALYZE on all search queries (FTS, vector, graph traversal)
2. Identify slow queries and missing indexes
3. Optimize join strategies and index usage
4. Document findings in `crates/maproom/docs/query_optimization.md`
5. Create indexes or adjust queries as needed

### Performance Targets
- p50 latency: <30ms
- Cache hit rate: >70% for common queries
- Connection pool utilization: <80% under typical load

## Dependencies
- HYBRID_SEARCH-3003 (signal integration) - All search queries must be implemented before optimization
- PostgreSQL database with pg_trgm and vector extensions installed
- Existing database schema with chunks, chunk_edges tables

## Risk Assessment
- **Risk**: Materialized view refresh strategy not defined
  - **Mitigation**: Implement incremental refresh strategy or document manual refresh requirements; consider CONCURRENTLY option for production refreshes

- **Risk**: Cache invalidation timing may serve stale results
  - **Mitigation**: Implement TTL-based cache expiration; document cache freshness guarantees; consider cache invalidation on data updates

- **Risk**: Connection pool exhaustion under high load
  - **Mitigation**: Configure appropriate pool sizes; implement connection timeout handling; add monitoring for pool metrics

- **Risk**: Query optimizations may vary across PostgreSQL versions
  - **Mitigation**: Document minimum PostgreSQL version requirements; test on target production version

## Files/Packages Affected
- `crates/maproom/migrations/XXX_create_materialized_views.sql` - New migration for materialized views
- `crates/maproom/src/db/pool.rs` - New file for connection pooling configuration
- `crates/maproom/src/search/cache.rs` - New file for query result caching implementation
- `crates/maproom/src/search/mod.rs` - Integration of caching layer into search pipeline
- `crates/maproom/src/db/mod.rs` - Integration of connection pooling
- `crates/maproom/docs/query_optimization.md` - New documentation file for EXPLAIN ANALYZE results and optimization strategies
- `Cargo.toml` - Add dependencies for LRU cache and connection pooling crates
- `crates/maproom/src/lib.rs` - Export new cache and pool modules

## Planning References
- Architecture: `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`
  - Query Optimization section (lines 404-425)
  - Caching Strategy section (lines 343-379)
  - Performance Considerations (lines 427-449)
- Project Plan: Phase 4, Week 4, Task 1
