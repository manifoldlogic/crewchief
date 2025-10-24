# Ticket: HYBRID_SEARCH-3901: Test Score Fusion

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- search-quality-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Integration and validation testing for Phase 3 score fusion implementations, including RRF (Reciprocal Rank Fusion), weighted fusion, signal integration, and quality metrics validation.

## Background
Phase 3 of the hybrid search system implements score fusion algorithms that combine multiple ranking signals (FTS, vector similarity, graph importance, recency, churn) into final search result rankings. This ticket ensures all fusion algorithms are correctly implemented, properly tested, and deliver measurable quality improvements.

The score fusion layer is critical because it determines how different ranking signals are balanced and combined to produce the best search results. Testing must validate both algorithmic correctness and search quality outcomes.

## Acceptance Criteria
- [ ] RRF implementation verified with correct formula (1 / (k + rank + 1))
- [ ] Weighted fusion configurable with hot-reloadable weights
- [ ] All signals (FTS, vector, graph, recency, churn) integrated and contributing to scores
- [ ] Score explanations available showing signal contributions in debug mode
- [ ] Quality metrics (NDCG, MRR) measured and show improvement over baseline
- [ ] Fusion performance measured at <10ms per query
- [ ] Edge cases tested (empty result sets, single signal, extreme weights)
- [ ] Comprehensive test coverage (>90%) for fusion modules

## Technical Requirements
- Implement RRF tests with various k values (30, 60, 120)
- Test weighted fusion with default and extreme weight configurations
- Validate graph importance calculation on well-connected chunks
- Test recency decay favors recent commits
- Verify churn normalization (high churn = lower score)
- Compare RRF vs weighted fusion on golden test query set
- Measure ranking quality using NDCG and MRR metrics
- Validate score breakdown in debug mode shows all signal contributions
- Test weight hot-reload functionality (config changes without restart)
- Performance benchmarks for fusion operations

## Implementation Notes

### Test Structure
Create four integration test modules:

1. **RRF Fusion Tests** (`crates/maproom/tests/integration/rrf_fusion_test.rs`)
   - Test RRF formula correctness with known rank inputs
   - Verify k parameter effects (lower k = more focus on top ranks)
   - Test edge cases: empty sets, single result, duplicate chunks across result sets
   - Compare RRF vs simple average on test queries

2. **Weighted Fusion Tests** (`crates/maproom/tests/integration/weighted_fusion_test.rs`)
   - Test default weight configuration (FTS=0.4, vector=0.35, graph=0.1, recency=0.1, churn=0.05)
   - Verify weight hot-reload from config file changes
   - Test extreme weight configurations (e.g., FTS=1.0, all others=0.0)
   - Validate score breakdown in debug mode shows individual signal contributions
   - Test weight normalization if implemented

3. **Signal Integration Tests** (`crates/maproom/tests/integration/signal_integration_test.rs`)
   - Test graph importance calculation using known chunk relationships
   - Verify recency decay formula favors recent commits
   - Test churn normalization (1 / (1 + churn_score))
   - Validate each signal's contribution to final scores
   - Test missing signal handling (graceful degradation)

4. **Fusion Quality Tests** (`crates/maproom/tests/integration/fusion_quality_test.rs`)
   - Golden test query set with expected top results
   - Measure NDCG (Normalized Discounted Cumulative Gain)
   - Measure MRR (Mean Reciprocal Rank)
   - Compare fusion methods (RRF vs weighted) on quality metrics
   - Test score explanation generation and accuracy
   - Performance benchmarks (<10ms requirement)

### Test Data Requirements
- Well-connected chunk graph for graph signal testing
- Chunks with varying commit timestamps for recency testing
- Chunks with different churn levels for churn testing
- Golden query set with human-judged relevant results
- Baseline search results for quality comparison

### Architecture Reference
See `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`:
- RRF implementation (lines 209-234)
- Weighted fusion (lines 236-260)
- Graph signals (lines 182-205)

### Quality Metrics
- **NDCG**: Measures ranking quality with graded relevance
- **MRR**: Measures how quickly first relevant result appears
- **Baseline**: Compare against single-signal search (FTS-only or vector-only)
- **Target**: 15-20% improvement in NDCG over baseline

## Dependencies
- HYBRID_SEARCH-3001 (RRF Fusion Implementation)
- HYBRID_SEARCH-3002 (Weighted Fusion Implementation)
- HYBRID_SEARCH-3003 (Signal Integration)
- Golden test query dataset (may need to be created)

## Risk Assessment
- **Risk**: Golden test query set may not exist yet
  - **Mitigation**: Create minimal golden set as part of this ticket, or use synthetic test data with known expected rankings

- **Risk**: Quality metrics may show no improvement if signal weights are not tuned
  - **Mitigation**: Document current metrics as baseline; weight tuning can be separate optimization ticket

- **Risk**: Performance benchmarks may not meet <10ms target on large result sets
  - **Mitigation**: Profile fusion operations, identify bottlenecks, create performance optimization ticket if needed

- **Risk**: Signal integration may be incomplete if upstream tickets are not finished
  - **Mitigation**: Use mock signals or stubs for testing fusion logic independently

## Files/Packages Affected
- `crates/maproom/tests/integration/rrf_fusion_test.rs` (NEW)
- `crates/maproom/tests/integration/weighted_fusion_test.rs` (NEW)
- `crates/maproom/tests/integration/signal_integration_test.rs` (NEW)
- `crates/maproom/tests/integration/fusion_quality_test.rs` (NEW)
- `crates/maproom/src/search/fusion.rs` (tested)
- `crates/maproom/src/search/signals.rs` (tested)
- `crates/maproom/tests/fixtures/` (test data fixtures)
- `docs/testing/phase3_fusion_test_results.md` (test report - NEW)
