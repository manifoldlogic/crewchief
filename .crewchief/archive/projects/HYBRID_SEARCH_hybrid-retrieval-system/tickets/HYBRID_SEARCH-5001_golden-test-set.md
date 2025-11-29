# Ticket: HYBRID_SEARCH-5001: Golden Test Set Creation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- search-quality-engineer
- integration-tester
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Create a comprehensive golden test set of 100 representative search queries with manually curated ground truth results. Implement automated evaluation metrics (precision@k, recall@k, NDCG, MRR) to measure and track hybrid search quality over time.

## Background
As the hybrid search system integrates multiple retrieval methods (FTS, vector search, graph-enhanced ranking), we need a rigorous way to evaluate search quality objectively. A golden test set provides:

1. **Baseline Quality Measurement**: Establish current system performance
2. **Regression Detection**: Catch quality degradation during future changes
3. **Optimization Validation**: Verify that fusion weight tuning improves results
4. **Comparative Analysis**: Compare different fusion strategies (RRF vs weighted)

This task is part of Phase 5 (Quality Assurance & Tuning) and is foundational for all subsequent optimization work. Without ground truth data, we cannot objectively measure improvements.

## Acceptance Criteria
- [ ] 100 diverse test queries defined covering multiple search patterns
- [ ] Ground truth top-10 results manually curated for each query
- [ ] Relevance labels established (highly relevant, somewhat relevant, not relevant)
- [ ] Evaluation metrics implemented: precision@k, recall@k, NDCG, MRR
- [ ] Automated test framework created that runs against current search system
- [ ] Baseline quality scores documented for current hybrid search implementation
- [ ] Test methodology documentation created
- [ ] All tests pass and can be run via `cargo test --test golden_test`

## Technical Requirements

### Query Diversity
Queries must cover the following categories:

1. **Simple Symbol Searches** (20 queries)
   - Single identifiers: "useState", "createConnection", "handleError"
   - Class/type names: "AuthManager", "HttpClient", "QueryBuilder"

2. **Semantic Concept Searches** (25 queries)
   - Architectural patterns: "authentication flow", "error handling", "state management"
   - Functional concepts: "database connection pooling", "retry logic", "caching layer"

3. **Code Pattern Searches** (20 queries)
   - Language-specific patterns: "async error handling", "React hooks with effects"
   - API usage: "REST endpoint definitions", "GraphQL resolvers"

4. **Multi-Word Technical Queries** (20 queries)
   - Component descriptions: "React component with state", "PostgreSQL query builder"
   - Feature searches: "user authentication middleware", "file upload handler"

5. **Edge Cases** (15 queries)
   - Single-character queries: "x", "i"
   - Very long queries: "how to implement a secure authentication flow with JWT tokens and refresh token rotation"
   - Ambiguous terms: "get", "handle", "process"

### Ground Truth Methodology
For each query:
1. Run query against current codebase
2. Manually review top 20 results
3. Label each result with relevance score:
   - **3** (highly relevant): Direct match, primary implementation
   - **2** (relevant): Related functionality, secondary implementation
   - **1** (somewhat relevant): Tangentially related
   - **0** (not relevant): No meaningful relationship

4. Document rationale for top-10 selections in `ground_truth.jsonl`

### Data Format

**queries.jsonl**:
```jsonl
{"id": 1, "query": "useState", "category": "simple_symbol", "expected_language": "typescript"}
{"id": 2, "query": "authentication flow", "category": "semantic", "expected_language": null}
```

**ground_truth.jsonl**:
```jsonl
{"query_id": 1, "results": [
  {"chunk_id": 1234, "file_path": "src/hooks/useState.ts", "symbol": "useState", "relevance": 3, "rationale": "Primary hook definition"},
  {"chunk_id": 5678, "file_path": "src/components/Form.tsx", "symbol": "FormComponent", "relevance": 2, "rationale": "Example usage"}
]}
```

### Evaluation Metrics

Implement the following metrics in `crates/maproom/src/evaluation/metrics.rs`:

1. **Precision@K**: Proportion of relevant results in top-K
   - Formula: `relevant_in_top_k / k`
   - Calculate for k = 1, 5, 10, 20

2. **Recall@K**: Proportion of all relevant results found in top-K
   - Formula: `relevant_in_top_k / total_relevant`
   - Calculate for k = 1, 5, 10, 20

3. **NDCG (Normalized Discounted Cumulative Gain)**:
   - Formula: `DCG@k / IDCG@k`
   - Use relevance scores (0-3) for graded relevance
   - Calculate for k = 10, 20

4. **MRR (Mean Reciprocal Rank)**:
   - Formula: `1 / rank_of_first_relevant_result`
   - Average across all queries

### Test Framework Requirements

The automated test in `golden_test.rs` must:
1. Load all 100 queries from `queries.jsonl`
2. Execute each query through the full hybrid search pipeline
3. Compare results against ground truth
4. Calculate all evaluation metrics
5. Report aggregate statistics
6. Flag queries with significant quality degradation (precision@10 < 0.5)
7. Support filtering by query category for targeted testing

## Implementation Notes

### Test Creation Process
1. **Phase 1: Query Collection** (Week 5, Days 1-2)
   - Analyze existing codebase to identify representative searches
   - Collect queries from real user search patterns (if available)
   - Distribute queries across categories to ensure balance

2. **Phase 2: Ground Truth Curation** (Week 5, Days 3-4)
   - For each query, run against current search system
   - Manually review and label results
   - Document rationale for relevance scores
   - Have second reviewer validate top-10 for critical queries

3. **Phase 3: Metrics Implementation** (Week 5, Day 5)
   - Implement evaluation metrics in Rust
   - Add comprehensive unit tests for metric calculations
   - Validate against known examples

4. **Phase 4: Test Framework** (Week 5, Day 5)
   - Build automated test runner
   - Integrate with existing test infrastructure
   - Document baseline scores

### Evaluation Metrics Implementation

```rust
// Example structure for metrics.rs
pub struct EvaluationMetrics {
    pub precision_at_k: HashMap<usize, f64>,
    pub recall_at_k: HashMap<usize, f64>,
    pub ndcg_at_k: HashMap<usize, f64>,
    pub mrr: f64,
}

pub fn calculate_precision_at_k(
    results: &[ChunkId],
    ground_truth: &[GroundTruthResult],
    k: usize
) -> f64 {
    // Implementation
}

pub fn calculate_ndcg_at_k(
    results: &[ChunkId],
    ground_truth: &[GroundTruthResult],
    k: usize
) -> f64 {
    // Implementation with relevance grading
}
```

### Baseline Quality Targets
While we're establishing the baseline, we should aim for:
- **Precision@10**: > 0.7 (70% of top-10 results are relevant)
- **NDCG@10**: > 0.65 (good ranking of graded relevance)
- **MRR**: > 0.8 (first relevant result typically in top 2)

These targets will be validated and adjusted based on actual baseline measurements.

### Continuous Integration
The golden test should be:
- Run as part of CI/CD pipeline
- Generate reports showing metric trends over time
- Fail build if quality drops below threshold (e.g., NDCG@10 < 0.5)

## Dependencies
- **HYBRID_SEARCH-4003**: Complete hybrid search system must be operational
- **HYBRID_SEARCH-3001**: RRF fusion implementation (to test against)
- **HYBRID_SEARCH-3002**: Weighted fusion implementation (to compare)
- **HYBRID_SEARCH-2003**: Search integration must be complete

## Risk Assessment
- **Risk**: Manual curation of 100 queries is time-consuming and subjective
  - **Mitigation**: Start with 25 high-priority queries, expand iteratively; use two reviewers for validation

- **Risk**: Ground truth may become stale as codebase evolves
  - **Mitigation**: Document query creation date; plan quarterly review/refresh; focus on evergreen query patterns

- **Risk**: Metrics may not capture all aspects of search quality
  - **Mitigation**: Start with standard IR metrics; collect user feedback; add custom metrics as needed (e.g., category-specific performance)

- **Risk**: Test execution may be slow with 100 queries
  - **Mitigation**: Parallelize test execution; implement query result caching; support running subsets by category

- **Risk**: Relevance judgments may be inconsistent
  - **Mitigation**: Define clear relevance criteria; document rationale; use inter-rater reliability checks for critical queries

## Files/Packages Affected

### New Files
- `crates/maproom/tests/golden/queries.jsonl` - 100 test queries with metadata
- `crates/maproom/tests/golden/ground_truth.jsonl` - Ground truth results with relevance scores
- `crates/maproom/src/evaluation/mod.rs` - Evaluation module entry point
- `crates/maproom/src/evaluation/metrics.rs` - Metric implementations (precision, recall, NDCG, MRR)
- `crates/maproom/tests/golden/golden_test.rs` - Automated test runner
- `crates/maproom/docs/golden_test_methodology.md` - Test creation and maintenance guide

### Modified Files
- `crates/maproom/Cargo.toml` - Add any dependencies for metrics calculation (e.g., serde for JSONL)
- `crates/maproom/src/lib.rs` - Expose evaluation module
- `.github/workflows/test.yml` - Add golden test to CI pipeline (if applicable)

### Supporting Infrastructure
- `scripts/golden_test_report.sh` - Generate HTML reports from test results
- `scripts/validate_ground_truth.py` - Validate ground truth data format and consistency
