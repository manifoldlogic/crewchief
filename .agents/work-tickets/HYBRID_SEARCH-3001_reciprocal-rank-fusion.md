# Ticket: HYBRID_SEARCH-3001: Reciprocal Rank Fusion (RRF) Implementation

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
Implement Reciprocal Rank Fusion (RRF) algorithm to replace the basic weighted average fusion from Phase 2. RRF provides a rank-based fusion method that is more robust to score distribution differences across different search methods (FTS, vector, graph, signals).

## Background
Phase 2 implemented basic search integration using a simple weighted average of normalized scores. While functional, this approach has limitations when combining results from different search methods that have different score distributions and ranges.

Reciprocal Rank Fusion (RRF) is a proven algorithm that addresses these limitations by:
1. Using rank position instead of raw scores, making it robust to score distribution differences
2. Using a simple, interpretable formula: `1.0 / (k + rank + 1.0)` where k is typically 60
3. Summing RRF scores across all result sets to produce final rankings
4. Eliminating the need for score normalization and manual weight tuning

This implementation is part of Phase 3, Week 3 of the HYBRID_SEARCH project plan and represents a significant improvement in result quality over the baseline weighted fusion approach.

## Acceptance Criteria
- [ ] RRFFusion struct implemented with configurable k parameter (default 60)
- [ ] RRF score formula correctly implemented: `1.0 / (k + rank + 1.0)`
- [ ] Fusion algorithm handles multiple result sets (FTS, vector, graph, signals)
- [ ] Results are properly sorted by combined RRF score in descending order
- [ ] RRF integrated into search pipeline as an alternative fusion method
- [ ] Fusion benchmarks created comparing RRF vs baseline weighted fusion
- [ ] Unit tests verify RRF calculation correctness
- [ ] Integration tests verify end-to-end RRF fusion in search pipeline
- [ ] Performance benchmarks show RRF performance characteristics
- [ ] Documentation includes comparison results vs baseline weighted fusion

## Technical Requirements
- Implement `RRFFusion` struct in new module `crates/maproom/src/search/fusion/rrf.rs`
- k parameter must be configurable (default: 60.0)
- Handle empty result sets gracefully
- Support fusion of 2-4 result sets (FTS, vector, graph, signals)
- Use HashMap for efficient score accumulation
- Properly handle chunk_id as the fusion key
- Sort final results by RRF score (highest first)
- Integration with existing fusion infrastructure from HYBRID_SEARCH-2003
- Benchmark suite measuring:
  - RRF computation time vs number of results
  - RRF computation time vs number of result sets
  - Memory usage during fusion
  - Comparison with weighted fusion baseline
- Unit tests covering:
  - Basic RRF score calculation
  - Multiple result set fusion
  - Edge cases (empty sets, single item, duplicate items)
  - k parameter variations

## Implementation Notes

### RRF Algorithm Structure
From architecture document (lines 209-234):

```rust
pub struct RRFFusion {
    k: f32,  // Typically 60
}

impl RRFFusion {
    pub fn fuse(&self, result_sets: Vec<RankedResults>) -> Vec<FusedResult> {
        let mut scores: HashMap<ChunkId, f32> = HashMap::new();

        for results in result_sets {
            for (rank, result) in results.iter().enumerate() {
                let rrf_score = 1.0 / (self.k + rank as f32 + 1.0);
                *scores.entry(result.chunk_id).or_insert(0.0) += rrf_score;
            }
        }

        let mut fused: Vec<_> = scores.into_iter()
            .map(|(id, score)| FusedResult { chunk_id: id, score })
            .collect();

        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        fused
    }
}
```

### Key Implementation Considerations
1. **k Parameter**: Default to 60 as per literature (Cormack et al., 2009), but allow configuration for tuning
2. **Rank Indexing**: Use 0-based enumerate() output, formula accounts for this with +1
3. **Score Accumulation**: Use HashMap for O(1) score updates per chunk
4. **Floating Point**: Use f32 for consistency with other score types
5. **Sorting**: Use partial_cmp with unwrap (scores should never be NaN)
6. **Integration**: Should work with existing RankedResults and FusedResult types from Phase 2

### Benchmarking Strategy
Create comprehensive benchmark suite comparing:
- **RRF vs Weighted Fusion**: Same query set, compare result quality
- **Scalability**: Performance with varying numbers of results (100, 1000, 10000)
- **Result Set Count**: Performance with 2, 3, and 4 result sets
- **Memory Usage**: Track HashMap allocation and growth

### Testing Strategy
1. **Unit Tests**: Verify RRF calculation math is correct
2. **Integration Tests**: Test with real search results from all four sources
3. **Edge Cases**: Empty sets, single results, all same ranks
4. **k Parameter Tests**: Verify different k values produce expected behavior

## Dependencies
- **HYBRID_SEARCH-2003** (Initial Search Integration) - Required
  - Provides base fusion infrastructure
  - Defines RankedResults and FusedResult types
  - Establishes fusion interface patterns

## Risk Assessment
- **Risk**: RRF may not perform better than weighted fusion for all query types
  - **Mitigation**: Create comprehensive benchmark suite with diverse query patterns; keep both fusion methods available for comparison; document which queries favor which approach

- **Risk**: k parameter tuning may require extensive experimentation
  - **Mitigation**: Start with literature-recommended default (60); create tools to easily test different k values; document k parameter sensitivity in benchmarks

- **Risk**: Performance degradation with large result sets
  - **Mitigation**: Benchmark with realistic result set sizes; optimize HashMap usage; consider result set size limits if needed

- **Risk**: Integration changes may break existing fusion functionality
  - **Mitigation**: Maintain weighted fusion as fallback option; comprehensive integration tests; verify all existing tests still pass

## Files/Packages Affected
- `crates/maproom/src/search/fusion/rrf.rs` - New file, RRF implementation
- `crates/maproom/src/search/fusion/mod.rs` - Update to export RRF module
- `crates/maproom/src/search/fusion.rs` - Add RRF as fusion option/variant
- `crates/maproom/src/search/mod.rs` - Integration with search pipeline
- `crates/maproom/tests/fusion/rrf_test.rs` - New unit tests
- `crates/maproom/tests/integration/fusion_integration_test.rs` - Integration tests
- `crates/maproom/benches/fusion_benchmark.rs` - New benchmark suite
- `crates/maproom/Cargo.toml` - Ensure criterion dependency for benchmarks

## Planning References
- Architecture: `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md` (lines 209-234)
- Phase Plan: HYBRID_SEARCH project plan, Phase 3, Week 3, Task 1
