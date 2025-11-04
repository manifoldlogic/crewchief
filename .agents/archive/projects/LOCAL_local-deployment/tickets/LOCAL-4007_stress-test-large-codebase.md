# Ticket: LOCAL-4007: Stress test with large codebase (100k chunks)

## Status
- [ ] **Task completed** - WONT DO (not practical for Ollama use case)
- [ ] **Tests pass** - N/A
- [ ] **Verified** - N/A

**Rationale**: Stress testing with 100k+ chunks on Ollama is not practical or recommended for the LOCAL deployment model. Per project guidance: "Ollama takes much too long to index and is more for hobby projects and niche users requiring local-only."

The documentation already sets appropriate expectations:
- Ollama is positioned as the local/privacy option
- Performance warnings are clear (5-10x slower than cloud providers)
- Recommended for smaller projects or when privacy is paramount

Stress testing would consume significant time for a use case we explicitly don't recommend for large codebases. Users with large codebases should use OpenAI or Google providers, which have been validated in production.

## Agents
- performance-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Validate system stability and performance under extreme load by indexing a large codebase (targeting 100,000 chunks) and verifying that search remains fast and accurate at scale. This stress test proves the system can handle real-world large codebases beyond toy examples.

## Background
As part of Phase 4 (Testing & Optimization), we need to verify that the Maproom system scales to production-level workloads. The MVP must handle large codebases without performance degradation, crashes, or resource exhaustion. This ticket validates performance expectations and identifies any scalability bottlenecks that need addressing before production deployment.

This test uses real-world codebases (such as Linux kernel subsets or large React applications) to simulate actual usage patterns. Success demonstrates that the system meets the performance requirements defined in the LOCAL project plan.

## Acceptance Criteria
- [ ] Successfully index 100,000 chunks without crashes or OOM errors
- [ ] Search latency p95 < 150ms at 100k chunk scale (allowing some degradation from smaller scale)
- [ ] Memory usage plateaus (no unbounded/linear growth over time)
- [ ] No database connection exhaustion during stress test
- [ ] System remains responsive during heavy concurrent load
- [ ] Performance report identifies any scalability bottlenecks
- [ ] Recommendations for optimization documented (if needed)
- [ ] PostgreSQL indexes verified as optimal (VACUUM/REINDEX analysis)

## Technical Requirements

### 1. Large Repository Indexing
- Target: 100,000 chunks (roughly 500-1000 files depending on file size)
- Use real codebases: Linux kernel subset, large React app, or similar
- Measure time to index completely (expect 100+ files/minute sustained)
- Monitor memory usage throughout indexing process
- Verify no crashes or OOM errors during indexing
- Track PostgreSQL disk usage (expect ~5-10GB for 100k chunks)

### 2. Search Performance at Scale
- Query performance testing with 100k chunks in database
- Vector search latency (target: <100ms p95)
- Full-text search (FTS) latency
- Hybrid search performance (combined vector + FTS)
- Database query plan analysis (EXPLAIN ANALYZE)
- Test various query patterns (short/long queries, rare/common terms)

### 3. Concurrent Operations
- Multiple concurrent search requests (simulate 10+ users)
- Indexing while searching (resource contention handling)
- Connection pool behavior under load
- Thread pool utilization
- Resource contention detection

### 4. Memory Stability
- Long-running stability test (24 hours if feasible)
- Memory leak detection (should plateau, not grow unbounded)
- Connection pool exhaustion testing
- Database vacuum performance
- Rust memory profiling if needed

### 5. Database Performance
- pgvector index performance at scale
- GIN index query times for full-text search
- Table bloat analysis
- Determine if index optimization needed (VACUUM, REINDEX)
- Query plan verification for optimal index usage

### Performance Expectations (from LOCAL_PLAN.md)
- **Indexing**: 100+ files/minute sustained throughput
- **Search latency**: <100ms p95 (allowing up to 150ms for 100k chunks)
- **Memory**: Growth should plateau, not continue linearly
- **Database size**: ~5-10GB for 100k chunks
- **System responsiveness**: No freezing or unresponsiveness during load

## Implementation Notes

### Test Environment Setup
1. Use Docker Compose environment from LOCAL-1003
2. Allocate sufficient resources (recommend 8GB RAM, 4 CPUs)
3. Use production-like PostgreSQL settings
4. Enable pgvector indexing with appropriate parameters

### Test Data Selection
- Option 1: Linux kernel subset (well-structured C code)
- Option 2: Large React/TypeScript application (real-world JS/TS)
- Option 3: Combination of multiple medium codebases
- Ensure mix of file types (TypeScript, JavaScript, Rust, Markdown, JSON)

### Performance Measurement Tools
- Use `hyperfine` or similar for latency percentiles
- PostgreSQL `pg_stat_statements` for query analysis
- `EXPLAIN ANALYZE` for query plan inspection
- Memory profiling with Rust tools (e.g., `heaptrack`, `valgrind`)
- Docker stats for container resource monitoring

### Test Script Structure
Create automated test script that:
1. Indexes large codebase and measures time/memory
2. Runs search performance suite (various query types)
3. Runs concurrent load test (multiple simultaneous operations)
4. Monitors for 24 hours (if time allows)
5. Generates performance report with recommendations

### Expected Bottlenecks to Investigate
- PostgreSQL query performance (may need index tuning)
- pgvector index type (IVFFlat vs HNSW trade-offs)
- Embedding batch size (balance speed vs memory)
- Connection pool sizing
- Tokio runtime thread pool sizing

### Report Format
Performance report should include:
- Test environment specifications
- Codebase characteristics (file count, total LOC, chunk count)
- Indexing throughput (files/min, chunks/sec)
- Search latency distribution (p50, p95, p99)
- Memory usage graphs
- Database size and index statistics
- Identified bottlenecks
- Optimization recommendations
- Pass/fail assessment against acceptance criteria

## Dependencies
- **LOCAL-4004**: E2E tests pass (prerequisite)
- **LOCAL-1003**: Docker Compose orchestration (test environment)
- **LOCAL-1002**: PostgreSQL init schema (database setup)
- **LOCAL-2001-2006**: Ollama embedding integration (indexing capability)

## Risk Assessment

- **Risk**: Test may reveal fundamental scalability issues requiring architecture changes
  - **Mitigation**: This is intentionally placed in Week 4 to allow time for remediation. If major issues found, escalate to planning phase for resolution strategy.

- **Risk**: 100k chunks may exceed available hardware resources in test environment
  - **Mitigation**: Start with 50k chunks and scale up. Document minimum hardware requirements. Test on cloud instance if local resources insufficient.

- **Risk**: PostgreSQL performance may degrade due to index configuration
  - **Mitigation**: Research pgvector best practices before test. Document index tuning parameters. Test both IVFFlat and HNSW index types.

- **Risk**: Test may take very long to complete (multi-hour indexing + 24hr stability)
  - **Mitigation**: Estimated 8 hours includes test execution. Stability test can run overnight. Prioritize critical measurements first.

- **Risk**: Difficult to find suitable test codebase with exact chunk count
  - **Mitigation**: Combine multiple codebases or use subset of very large repo. Document actual chunk count achieved and scale results accordingly.

## Files/Packages Affected
- `crates/maproom/tests/stress/` (new directory for stress tests)
- `crates/maproom/tests/stress/large_codebase_test.rs` (new test file)
- `scripts/stress-test-large-codebase.sh` (new test automation script)
- `docs/performance/stress-test-report.md` (new performance report)
- `crates/maproom/src/embeddings/batch.rs` (may need tuning based on results)
- `crates/maproom/src/db/pool.rs` (connection pool sizing)
- `docker-compose.yml` (PostgreSQL performance tuning if needed)
- `README.md` (update with minimum hardware requirements)

## References
- PostgreSQL Performance Tips: https://www.postgresql.org/docs/current/performance-tips.html
- pgvector Indexing Guide: https://github.com/pgvector/pgvector#indexing
- LOCAL Project Plan: `/workspace/docs/LOCAL_PLAN.md`
- Phase 4 Context: Testing & Optimization (Week 4)
