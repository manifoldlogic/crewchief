# Ticket: LOCAL-4002: Compare Ollama vs OpenAI Quality Metrics

## Status
- [ ] **Task completed** - WONT DO (not critical for production use)
- [ ] **Tests pass** - N/A
- [ ] **Verified** - N/A

**Rationale**: Quality comparison benchmarks are not essential for the production release. The package documentation already clearly positions Ollama as the "slower but local" option, with OpenAI and Google as the recommended choices for best performance. Users can choose based on their priorities:
- **OpenAI/Google**: Fast, low cost, recommended for best experience
- **Ollama**: Slower (5-10x), but 100% local and private

Formal quality metrics would be nice-to-have but don't change the value proposition. The implementation is complete and working; benchmarks can be added as future enhancement if user demand warrants it.

## Agents
- search-quality-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Measure and compare search quality between Ollama (nomic-embed-text) and OpenAI embeddings to validate that the local model provides acceptable accuracy for code retrieval tasks. This quality validation ensures the local LLM solution is fit for production use.

## Background
The LOCAL project aims to provide a zero-configuration, containerized Maproom MCP service with local LLM embeddings (Ollama + nomic-embed-text). While Phase 1-3 focus on infrastructure, integration, and user experience, Phase 4 validates that the local model meets quality standards.

OpenAI embeddings have been the established baseline for Maproom's semantic search capabilities. Before fully transitioning to local embeddings, we must verify that Ollama's nomic-embed-text model provides comparable search quality for code retrieval tasks.

From LOCAL_ANALYSIS.md:
- nomic-embed-text: 768 dimensions (vs OpenAI's 1536) - 50% smaller vectors
- Optimized for code and text embeddings
- Excellent performance on MTEB benchmarks
- Expected quality: Within 10% of OpenAI (NDCG metric)

This ticket validates whether local embeddings meet the quality threshold for production use.

## Acceptance Criteria
- [ ] Golden test suite created with 20+ representative queries across categories:
  - Exact matches (function/class names)
  - Semantic queries (natural language like "authentication logic", "error handling")
  - Partial matches (code fragments)
  - Cross-file relationships
- [ ] Manual relevance judgments documented for top-10 results per query
- [ ] Ollama (nomic-embed-text) metrics calculated:
  - Precision@k (k=1, 5, 10)
  - Recall@k (k=1, 5, 10)
  - Mean Reciprocal Rank (MRR)
  - Normalized Discounted Cumulative Gain (NDCG)
- [ ] OpenAI metrics calculated using identical test queries
- [ ] Comparison report generated with delta analysis
  - Side-by-side metric tables
  - Category-level breakdowns (exact vs semantic vs partial)
  - Statistical significance testing
- [ ] Visual comparison created (charts/graphs showing metric deltas)
- [ ] Quality threshold verified: Ollama within 10% of OpenAI on NDCG metric
- [ ] Critical query validation: Function/class name searches have 100% accuracy (Precision@1)
- [ ] Semantic query validation: 80%+ relevance on natural language queries
- [ ] Regression test suite created for future validation (automated quality checks)

## Technical Requirements

### Test Dataset Creation
- **Query Categories**:
  - Exact matches: 5+ queries (e.g., "spawnAgent", "MessageBus", "TreeSitterParser")
  - Semantic queries: 8+ queries (e.g., "agent orchestration", "message handling", "error logging")
  - Partial matches: 4+ queries (e.g., code fragments, incomplete identifiers)
  - Cross-file relationships: 3+ queries (e.g., "functions that call X", "implementations of Y")
- **Relevance Judgments**:
  - Binary relevance (relevant/not relevant) for top-10 results per query
  - Manual inspection by domain expert (search-quality-engineer)
  - Document judgment rationale for edge cases

### Metrics Implementation
- **Precision@k**: (# relevant results in top k) / k
- **Recall@k**: (# relevant results in top k) / (total # relevant results)
- **MRR**: 1 / (rank of first relevant result), averaged across queries
- **NDCG**: Normalized discounted cumulative gain
  - DCG = sum(rel_i / log2(i+1)) for i=1 to k
  - IDCG = DCG for ideal ranking
  - NDCG = DCG / IDCG
  - Reference: https://en.wikipedia.org/wiki/Discounted_cumulative_gain

### Testing Procedure
1. Index identical codebase with both Ollama and OpenAI embeddings
2. Run same query set against both indexes
3. Collect top-10 results for each query/provider combination
4. Apply relevance judgments to results
5. Calculate metrics for each provider
6. Generate comparison report with visualizations

### Quality Thresholds (from LOCAL_ANALYSIS.md)
- **Primary**: NDCG within 10% between Ollama and OpenAI
- **Critical**: Precision@1 = 100% for exact match queries (function/class names)
- **Semantic**: 80%+ relevance for natural language queries

### Tools/Libraries
- Maproom MCP search API (both Ollama and OpenAI modes)
- Python/Rust for metric calculation
- Visualization library (matplotlib, plotly, or similar)
- Statistical testing (t-test or Mann-Whitney U for significance)

## Implementation Notes

### Search Quality Engineer Pattern
From .agents/specialized-agents/search-quality-engineer.md:
- Curate representative queries across use cases
- Manual relevance judgments (ground truth)
- Calculate standard IR metrics
- Compare ranking quality between systems
- Document methodology and findings

### Testing Strategy
1. **Baseline Establishment**: Run OpenAI embeddings first to establish ground truth
2. **Ollama Comparison**: Index same codebase with Ollama, run identical queries
3. **Metric Calculation**: Implement standard IR metrics (precision, recall, MRR, NDCG)
4. **Analysis**: Identify categories where Ollama underperforms (if any)
5. **Regression Suite**: Automate key queries for future quality validation

### Expected Outcomes
Based on nomic-embed-text specifications:
- **Exact matches**: Should have 100% accuracy (model is good at lexical overlap)
- **Semantic queries**: May have 5-10% lower accuracy than OpenAI (smaller model)
- **Overall NDCG**: Expected to be within 10% threshold

If quality gaps are identified:
- Document specific query types with issues
- Assess whether gaps are acceptable for MVP
- Consider hybrid mode (local + OpenAI fallback) for critical queries

### Deliverables Format
- **Markdown Report**: `LOCAL-4002_quality_comparison_report.md`
  - Executive summary with pass/fail on 10% threshold
  - Methodology section (dataset, metrics, procedure)
  - Results tables (metrics by category and overall)
  - Visualizations (bar charts, scatter plots)
  - Recommendations
- **Test Data**: JSON files with queries, relevance judgments, raw results
- **Regression Suite**: Automated test script for future quality checks

## Dependencies
- **LOCAL-4001**: Performance benchmarks complete (verifies Ollama is functional)
- **LOCAL-3001**: npx startup flow working (enables easy testing of both modes)
- **Working Ollama Integration**: Phase 2 Ollama provider integration must be complete
- **Working OpenAI Integration**: Baseline OpenAI embeddings must be available for comparison

## Risk Assessment
- **Risk**: Manual relevance judgments are subjective and time-consuming
  - **Mitigation**: Document judgment criteria clearly, focus on high-value queries, consider inter-rater reliability check with second reviewer for subset of queries

- **Risk**: Quality may not meet 10% threshold, blocking MVP
  - **Mitigation**: Identify specific failing categories, assess if acceptable for initial release, document gaps for future improvement, consider hybrid mode with OpenAI fallback

- **Risk**: Small test dataset (20 queries) may not represent all use cases
  - **Mitigation**: Ensure diversity across categories, prioritize most common user queries, plan for expanded dataset in future iterations

- **Risk**: Codebase-specific results may not generalize
  - **Mitigation**: Test on CrewChief codebase (TypeScript + Rust), document language/domain coverage, note limitations in report

## Files/Packages Affected
- `crates/maproom/src/embeddings/` - May need provider comparison utilities
- `crates/maproom/tests/` - Regression test suite for quality checks
- `docs/LOCAL-4002_quality_comparison_report.md` - Comparison report output
- `tests/quality/golden_test_suite.json` - Test dataset with queries and judgments
- `tests/quality/quality_metrics.rs` or `.py` - Metric calculation implementation
- `tests/quality/visualizations/` - Charts and graphs for report
