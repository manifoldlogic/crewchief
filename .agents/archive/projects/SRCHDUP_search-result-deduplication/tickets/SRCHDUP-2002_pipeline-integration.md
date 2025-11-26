# Ticket: SRCHDUP-2002: Integrate dedup into SearchPipeline

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (135 search tests pass)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Integrate the deduplication module into the SearchPipeline so that search results are automatically deduplicated after fusion and before the limit is applied. This is the core integration that makes deduplication active in the search flow.

## Background

The search pipeline processes queries through multiple stages: parsing, execution (FTS/vector/graph), fusion (RRF), and result assembly. Deduplication should occur after fusion (to benefit from full scoring) and before limit (to ensure limit applies to unique results).

**Reference:** plan.md Phase 2, architecture.md Sections 4 "Pipeline Integration" and "Limit Interaction"

## Acceptance Criteria

- [x] `SearchPipeline::search()` calls `dedup::deduplicate()` after fusion
- [x] Deduplication is conditional on `options.deduplicate`
- [x] Deduplication happens BEFORE limit is applied
- [x] Pipeline fetches extra results (3x limit) to compensate for dedup reduction
- [x] Default search behavior returns deduplicated results
- [x] `cargo test` shows no regression in existing search tests

## Technical Requirements

### Pipeline Modification
```rust
pub async fn search(
    &self,
    query: &str,
    options: &SearchOptions,
) -> Result<FinalSearchResults, PipelineError> {
    // Fetch extra results to ensure limit can be satisfied post-dedup
    let fetch_limit = if options.deduplicate {
        options.limit * 3
    } else {
        options.limit
    };

    // ... existing query processing ...
    let raw_results = self.execute_search(query, fetch_limit).await?;
    let fused_results = self.fusion.fuse(raw_results);

    // Apply deduplication if enabled
    let deduped_results = if options.deduplicate {
        let config = DeduplicationConfig::default();
        dedup::deduplicate(fused_results, &config)
    } else {
        fused_results
    };

    // Apply limit after deduplication
    let limited_results: Vec<_> = deduped_results
        .into_iter()
        .take(options.limit)
        .collect();

    Ok(FinalSearchResults::new(query.to_string(), limited_results, metadata))
}
```

### Import Statement
```rust
use crate::search::dedup::{self, DeduplicationConfig};
```

## Implementation Notes

1. **Identify insertion point** - Find where fusion results become final results
2. **Preserve metadata** - Ensure search metadata (timing, counts) is still accurate
3. **Handle empty results** - Dedup handles empty gracefully, but verify
4. **Consider debug mode** - If debug=true, may want to log dedup statistics

### Performance Consideration
The 3x fetch multiplier ensures we have enough unique results after dedup. For a typical `limit=10`, we fetch 30 and dedup down. This is a reasonable overhead for the benefit.

### Cache Key Update
If the search pipeline uses caching (check for `cache.rs`), ensure the cache key includes the `deduplicate` flag to prevent incorrect cache hits.

## Dependencies

- SRCHDUP-1001 (dedup module)
- SRCHDUP-2001 (SearchOptions.deduplicate field)

## Risk Assessment

- **Risk**: Pipeline structure has changed since design
  - **Mitigation**: Read current pipeline.rs before implementing
- **Risk**: Performance regression from 3x fetch
  - **Mitigation**: Benchmarks in Phase 4 will verify; can tune multiplier
- **Risk**: Breaking existing tests that expect specific result counts
  - **Mitigation**: Run full test suite, fix any affected tests

## Files/Packages Affected

- `crates/maproom/src/search/pipeline.rs` (modify search method)
- `crates/maproom/src/search/mod.rs` (verify dedup is exported)
