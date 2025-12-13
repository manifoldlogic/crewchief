# SRCHTRN-2002: Metadata Assembly in Pipeline

## Title
Assemble query understanding metadata in search pipeline

## Status
- [ ] **Implementation Complete**
- [ ] **Tests Passing**
- [ ] **Verified**
- [ ] **Committed**

## Agents
- **Primary**: rust-engineer
- **unit-test-runner**: Execute tests
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Modify `crates/maproom/src/search/pipeline.rs` to assemble `QueryUnderstanding` metadata from `ProcessedQuery`, collect timing breakdown, populate filters from `SearchOptions`, and attach to `SearchMetadata` in successful responses.

## Background
Phase 2 structures (SRCHTRN-2001) are now defined. This ticket integrates metadata assembly into the search pipeline, populating query understanding from existing data without adding new computation. The goal is <10ms overhead compared to Phase 1 baseline.

**Performance Budget**: <10ms overhead. All data already exists in memory - just expose it.

## Acceptance Criteria
- [ ] `QueryUnderstanding` assembled from `ProcessedQuery` after query processing
- [ ] `TimingBreakdown` collected from existing timing measurements
- [ ] `QueryFilters` populated from `SearchOptions`
- [ ] `understanding` field attached to `SearchMetadata` in successful responses
- [ ] Integration test validates metadata assembly end-to-end
- [ ] Performance overhead measured: <10ms vs. Phase 1 baseline
- [ ] Performance regression test passes (p95 latency <100ms)
- [ ] Manual test: Search returns metadata with tokens, mode, timing
- [ ] All tests passing

## Technical Requirements

### Modify Search Pipeline: `crates/maproom/src/search/pipeline.rs`

Locate the main search execution function and add metadata assembly:

```rust
pub async fn execute_search(
    query: &str,
    options: SearchOptions,
) -> Result<SearchResults, PipelineError> {
    // Start timing
    let start_time = Instant::now();

    // 1. Query Processing
    let query_start = Instant::now();
    let processed_query = process_query(query, &options)?;
    let query_processing_ms = query_start.elapsed().as_secs_f64() * 1000.0;

    // 2. Search Execution
    let search_start = Instant::now();
    let search_results = execute_searches(&processed_query, &options).await?;
    let search_execution_ms = search_start.elapsed().as_secs_f64() * 1000.0;

    // 3. Score Fusion
    let fusion_start = Instant::now();
    let fused_results = fuse_scores(&search_results, &processed_query)?;
    let score_fusion_ms = fusion_start.elapsed().as_secs_f64() * 1000.0;

    // 4. Result Assembly
    let assembly_start = Instant::now();
    let results = assemble_results(&fused_results, &options)?;
    let result_assembly_ms = assembly_start.elapsed().as_secs_f64() * 1000.0;

    // Assemble timing breakdown
    let timing = TimingBreakdown::new(
        query_processing_ms,
        search_execution_ms,
        score_fusion_ms,
        result_assembly_ms,
    );

    // Assemble query filters
    let filters = QueryFilters {
        repo_id: options.repo_id,
        worktree_id: options.worktree_id,
        file_types: options.file_types.clone(),
        recency_threshold: options.recency_threshold.clone(),
    };

    // Assemble query understanding
    let understanding = QueryUnderstanding::from_processed_query(
        &processed_query,
        filters,
        timing,
    );

    // Attach to metadata
    let metadata = SearchMetadata {
        // ... existing fields ...
        understanding: Some(understanding),
    };

    Ok(SearchResults {
        hits: results,
        total: results.len(),
        metadata,
    })
}
```

### Integration Test: `crates/maproom/tests/query_understanding.rs`

```rust
#[tokio::test]
async fn test_query_understanding_in_response() {
    let pipeline = create_test_pipeline().await;

    let result = pipeline
        .search(
            "authenticate user",
            SearchOptions {
                repo_id: 1,
                mode: SearchMode::Auto,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Verify metadata includes understanding
    assert!(result.metadata.understanding.is_some());
    let understanding = result.metadata.understanding.unwrap();

    // Verify tokens extracted
    assert_eq!(understanding.tokens, vec!["authenticate", "user"]);

    // Verify mode detected
    assert_eq!(understanding.mode, SearchMode::Auto);

    // Verify timing data present
    assert!(understanding.timing.total_ms > 0.0);
    assert!(understanding.timing.query_processing_ms > 0.0);

    // Verify timing breakdown is accurate (sum equals total)
    let sum = understanding.timing.query_processing_ms
        + understanding.timing.search_execution_ms
        + understanding.timing.score_fusion_ms
        + understanding.timing.result_assembly_ms;

    // Allow small floating point variance
    assert!((sum - understanding.timing.total_ms).abs() < 0.1);

    // Verify filters populated
    assert_eq!(understanding.filters.repo_id, 1);
}
```

### Performance Regression Test: `crates/maproom/tests/performance_regression.rs`

```rust
#[tokio::test]
async fn test_metadata_assembly_overhead() {
    // Load Phase 1 baseline from file
    let baseline = load_performance_baseline();

    // Run same search workload
    let pipeline = create_test_pipeline().await;
    let mut latencies = Vec::new();

    for query in test_queries() {
        let start = Instant::now();
        let _ = pipeline.search(query, test_options()).await;
        latencies.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    // Calculate p95 latency
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_idx = (latencies.len() as f64 * 0.95) as usize;
    let p95_latency = latencies[p95_idx];

    // Verify overhead is acceptable
    let overhead = p95_latency - baseline.p95;
    assert!(
        overhead < 10.0,
        "Metadata assembly overhead {}ms exceeds 10ms budget",
        overhead
    );

    // Verify absolute p95 target
    assert!(
        p95_latency < 100.0,
        "p95 latency {}ms exceeds 100ms target",
        p95_latency
    );
}
```

## Implementation Notes
1. Add timing measurements at each pipeline stage
2. Assemble `QueryUnderstanding` after query processing completes
3. Use existing data from `ProcessedQuery` and `SearchOptions`
4. Attach metadata only to successful responses (not errors)
5. Measure performance overhead vs. Phase 1 baseline

**Performance Optimization**:
- Use `Instant::now()` for timing (nanosecond precision, low overhead)
- Clone only what's needed (tokens, expanded_terms are already Vecs)
- Timing calculation is ~1μs (negligible)

**Critical**: If performance regression detected (>10ms overhead), investigate and optimize before merging.

## Dependencies
- **SRCHTRN-1000**: Performance baseline measured (provides comparison point)
- **SRCHTRN-2001**: Query understanding structures (must complete first)

## Risk Assessment
**Risk Level**: Medium

**Risks**:
- Performance overhead exceeds 10ms budget
- Timing measurements add latency
- Memory allocation for metadata

**Mitigations**:
- Performance regression test blocks merge if >10ms
- Baseline comparison validates overhead
- Use existing data (no new computation)
- Timing measurements are nanosecond precision (negligible overhead)

**BLOCK Criteria**: If p95 latency increases >10ms vs. baseline, investigate and optimize before merging.

## Files/Packages Affected
- **Modified**: `crates/maproom/src/search/pipeline.rs` (~40 lines added)
- **New file**: `crates/maproom/tests/query_understanding.rs` (integration test)
- **New file**: `crates/maproom/tests/performance_regression.rs` (performance test)
- **Reference**: `.crewchief/projects/SRCHTRN_search-transparency/planning/performance-baseline.md` (from SRCHTRN-1000)

## Estimated Effort
6-8 hours

**Breakdown**:
- Pipeline integration: 2-3 hours
- Timing collection: 1-2 hours
- Integration tests: 2 hours
- Performance testing: 1-2 hours

**Note**: May exceed 8 hours if performance optimization needed. Flag if approaching limit.

## Planning References
- [plan.md](../planning/plan.md) - Phase 2 ticket breakdown, performance requirements
- [architecture.md](../planning/architecture.md) - Metadata assembly design, performance budget
- [quality-strategy.md](../planning/quality-strategy.md) - Performance regression testing
- [performance-baseline.md](../planning/performance-baseline.md) - Phase 1 baseline (from SRCHTRN-1000)
