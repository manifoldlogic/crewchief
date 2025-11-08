# Ticket: BLOBSHA-3901: Execute Phase 3 Test Suite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute comprehensive test suite for Phase 3 (Application Integration) to verify cache behavior, query equivalence, metrics accuracy, and performance benchmarks. Report all test results without modifying code.

## Background
This ticket implements Step 3.4 from the BLOBSHA project plan (planning/plan.md, lines 441-454). After updating queries (BLOBSHA-3001) and implementing cache-aware upsert (BLOBSHA-3002), we must verify cache correctness, query equivalence, and performance before Phase 4 cleanup. Cache hit rate should be 70-90% for typical branch overlaps, and query performance must be within 10% of baseline.

## Acceptance Criteria
- [ ] Integration test passes: `test_cache_hit_duplicate_content`
  - Insert chunk with content X, verify embedding generated
  - Insert different chunk with same content X, verify no new embedding (cache hit)
  - Confirm only one embedding in code_embeddings for blob SHA
- [ ] Integration test passes: `test_cache_miss_unique_content`
  - Insert chunks with different content
  - Verify each generates new embedding (cache miss)
  - Confirm embedding count matches chunk count
- [ ] Integration test passes: `test_cache_metrics_accuracy`
  - Process known mix of duplicate/unique chunks
  - Verify hit/miss counts match expectations
  - Verify hit_rate() calculation correct
- [ ] E2E test passes: `test_search_query_equivalence`
  - Run same search query before/after migration
  - Verify results identical (same chunk IDs, same order)
  - Confirm both use HNSW index (via EXPLAIN ANALYZE)
- [ ] Performance benchmarks within targets:
  - Search latency within 10% of baseline
  - Cache check overhead < 5ms per chunk
  - Metrics reporting overhead < 1% of total scan time
- [ ] Test report generated with performance metrics

## Technical Requirements
- Execute via: `cd crates/maproom && cargo test cache --nocapture`
- Execute via: `cd crates/maproom && cargo test search_query_equivalence`
- Execute performance benchmarks: `cargo bench --bench search_performance`
- Generate metrics showing:
  - Cache hit rate (should be 70-90% for multi-branch tests)
  - Query latency (should be within 10% of baseline)
  - Embedding API cost savings

## Implementation Notes
The unit-test-runner agent should NOT modify any code. If tests fail:
1. Report which specific tests failed
2. Include cache metrics (hits/misses/rate)
3. Show query EXPLAIN ANALYZE output
4. Show performance benchmark results
5. Return to rust-indexer-engineer for fixes

Success criteria from planning/plan.md lines 451-454:
- All Phase 3 tests passing
- Cache metrics verified
- Query performance benchmarked (within 10% of baseline)

Expected cache behavior:
- Same content indexed twice: 100% hit rate on second
- Branch with 80% overlap: 80% hit rate
- Initial scan: 0% hit rate (cold cache)

Reference test implementations in planning/quality-strategy.md lines 329-415 (cache tests) and lines 417-437 (E2E tests).

## Dependencies
- BLOBSHA-3001 (queries updated to JOIN code_embeddings)
- BLOBSHA-3002 (cache-aware upsert implemented)
- Phase 1 & 2 tests passed
- Test database with full schema (chunks.blob_sha, code_embeddings table, foreign key)

## Risk Assessment
- **Risk**: False positives - tests pass but cache logic flawed
  - **Mitigation**: Multiple cache scenarios tested (hit, miss, concurrent)
- **Risk**: Performance benchmarks unreliable (flaky)
  - **Mitigation**: Run benchmarks 5 times, take median result

## Files/Packages Affected
- READ: `crates/maproom/tests/cache_tests.rs` (cache behavior tests)
- READ: `crates/maproom/tests/search_query_equivalence.rs` (E2E test)
- READ: `crates/maproom/benches/search_performance.rs` (performance benchmarks)
- READ: Test database (for integration tests)
