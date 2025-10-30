# Golden Test Set Methodology

## Overview

The golden test set is a comprehensive collection of 100 representative search queries with manually curated ground truth results. It serves as the foundation for objective evaluation of hybrid search quality, enabling regression detection, optimization validation, and comparative analysis.

## Table of Contents

1. [Test Creation Process](#test-creation-process)
2. [Query Categories](#query-categories)
3. [Relevance Criteria](#relevance-criteria)
4. [Evaluation Metrics](#evaluation-metrics)
5. [Running the Tests](#running-the-tests)
6. [Interpreting Results](#interpreting-results)
7. [Maintenance Procedures](#maintenance-procedures)
8. [Baseline Scores](#baseline-scores)

## Test Creation Process

### Phase 1: Query Collection

Queries were selected to represent real-world search patterns across different use cases:

1. **Codebase Analysis**: Identified common code patterns, symbols, and concepts in the CrewChief project
2. **Use Case Coverage**: Ensured queries span different developer tasks (finding functions, understanding architecture, debugging)
3. **Category Distribution**: Balanced queries across 5 categories to ensure comprehensive coverage
4. **Language Diversity**: Included both language-specific (TypeScript, Rust) and language-agnostic queries

### Phase 2: Ground Truth Curation

For each query, ground truth was established through:

1. **Initial Search**: Executed query against the current search system
2. **Manual Review**: Examined top 20 results for relevance
3. **Relevance Labeling**: Assigned relevance scores (0-3) to results based on defined criteria
4. **Rationale Documentation**: Documented why each result received its relevance score
5. **Top-10 Selection**: Selected the 10 most relevant results as ground truth

**Quality Assurance**:
- Critical queries (high-frequency patterns) were validated by a second reviewer
- Edge case queries were explicitly documented with expected behavior
- Relevance judgments were based on objective criteria, not subjective preferences

### Phase 3: Data Format

Test data is stored in JSONL (JSON Lines) format for easy parsing and versioning:

**queries.jsonl**: One query per line
```jsonl
{"id": 1, "query": "useState", "category": "simple_symbol", "expected_language": "typescript"}
```

**ground_truth.jsonl**: One ground truth entry per line
```jsonl
{"query_id": 1, "results": [
  {"chunk_id": 1001, "file_path": "packages/cli/src/hooks/useState.ts", "symbol": "useState", "relevance": 3, "rationale": "Primary hook definition"},
  {"chunk_id": 1002, "file_path": "packages/cli/src/components/Form.tsx", "symbol": "FormComponent", "relevance": 2, "rationale": "Example usage of useState"}
]}
```

## Query Categories

### 1. Simple Symbol Searches (20 queries)

**Purpose**: Test exact matching and symbol lookup

**Examples**:
- `useState` - React hook
- `createConnection` - Database function
- `AuthManager` - Class name

**Expected Behavior**:
- Top result should be the primary definition
- Secondary results should include usage examples and documentation
- Precision@10 should be > 0.8

### 2. Semantic Concept Searches (25 queries)

**Purpose**: Test conceptual understanding and semantic search

**Examples**:
- `authentication flow` - Architectural pattern
- `error handling` - Cross-cutting concern
- `database connection pooling` - System concept

**Expected Behavior**:
- Results should span multiple files implementing the concept
- Should prioritize core implementations over peripheral mentions
- NDCG@10 should be > 0.7 (proper ranking of graded relevance)

### 3. Code Pattern Searches (20 queries)

**Purpose**: Test language-specific pattern recognition

**Examples**:
- `async error handling` - Language pattern
- `React hooks with effects` - Framework pattern
- `Promise.all pattern` - API usage

**Expected Behavior**:
- Should find diverse examples of the pattern
- Should rank canonical examples higher
- Precision@10 should be > 0.7

### 4. Multi-Word Technical Queries (20 queries)

**Purpose**: Test complex query understanding

**Examples**:
- `user authentication middleware` - Multi-faceted concept
- `PostgreSQL query builder` - Technology-specific component
- `JWT token validation` - Specific functionality

**Expected Behavior**:
- Should understand query intent (all terms relevant)
- Should not over-weight individual terms
- NDCG@10 should be > 0.65

### 5. Edge Cases (15 queries)

**Purpose**: Test robustness and graceful degradation

**Examples**:
- `x` - Single character
- `get` - Extremely common/ambiguous term
- Very long natural language queries

**Expected Behavior**:
- Should return reasonable results even for difficult queries
- May have lower precision than other categories
- Should not crash or return zero results

## Relevance Criteria

### Relevance Scale

**3 - Highly Relevant**:
- **Symbol searches**: Primary definition of the symbol
- **Concept searches**: Core implementation files central to the concept
- **Pattern searches**: Canonical examples demonstrating the pattern

**2 - Relevant**:
- **Symbol searches**: Usage examples, secondary implementations, tests
- **Concept searches**: Supporting files that contribute to the concept
- **Pattern searches**: Valid but non-canonical examples

**1 - Somewhat Relevant**:
- **Symbol searches**: Tangentially related code (imports, references)
- **Concept searches**: Files that mention but don't implement the concept
- **Pattern searches**: Partial matches or related patterns

**0 - Not Relevant**:
- No meaningful relationship to the query
- False matches (keyword collisions)
- Completely unrelated functionality

### Relevance Guidelines

When labeling results, consider:

1. **Intent Match**: Does the result match what a developer is likely looking for?
2. **Completeness**: Does the result contain substantive implementation, not just mentions?
3. **Rank Appropriateness**: Should this result appear before others?
4. **Context**: Is this result useful in the context of the query?

## Evaluation Metrics

### Precision@K

**Definition**: Proportion of relevant results in top K

**Formula**: `P@K = (relevant results in top K) / K`

**Interpretation**:
- P@10 = 0.8 means 8 out of top 10 results are relevant
- Higher is better
- Target: > 0.7 overall, > 0.8 for simple symbols

### Recall@K

**Definition**: Proportion of all relevant results found in top K

**Formula**: `R@K = (relevant results in top K) / (total relevant results)`

**Interpretation**:
- R@10 = 0.6 means we found 60% of all relevant results
- Higher is better
- Limited by K (top-20 ground truth limits recall@10)

### NDCG@K (Normalized Discounted Cumulative Gain)

**Definition**: Ranking quality with graded relevance

**Formula**:
```
DCG@K = Σ(rel_i / log2(i + 1))  for i=1 to K
IDCG@K = DCG of ideal ranking
NDCG@K = DCG@K / IDCG@K
```

**Interpretation**:
- NDCG@10 = 1.0 means perfect ranking
- NDCG@10 = 0.7 means good but not perfect ranking
- Accounts for both relevance AND position
- Target: > 0.65 overall

### MRR (Mean Reciprocal Rank)

**Definition**: Average of 1/rank of first relevant result

**Formula**: `MRR = 1 / rank_of_first_relevant`

**Interpretation**:
- MRR = 1.0 means first result is relevant
- MRR = 0.5 means first relevant result at position 2
- Emphasizes getting at least one good result quickly
- Target: > 0.8

## Running the Tests

### Prerequisites

1. **Database Setup**: PostgreSQL with indexed codebase
2. **Embedding Service**: Running for vector search
3. **Environment**: `.env` configured with database credentials

### Running All Tests

```bash
# Run all 100 golden tests
cargo test --test golden_test -- --ignored

# With detailed output
cargo test --test golden_test -- --ignored --nocapture
```

### Running Specific Categories

```bash
# Test only simple symbol queries (20 queries)
cargo test --test golden_test test_simple_symbol_queries -- --ignored

# Test only semantic queries (25 queries)
cargo test --test golden_test test_semantic_queries -- --ignored

# Test only edge cases (15 queries)
cargo test --test golden_test test_edge_case_queries -- --ignored
```

### Running Without Database (Unit Tests)

The test file includes unit tests that don't require a database:

```bash
# Run only unit tests (data loading, metrics aggregation)
cargo test --test golden_test
```

## Interpreting Results

### Successful Test Run

```
Overall Quality Metrics (All 100 Queries)
============================================================

Precision@K:
  P@ 1: 0.9200
  P@ 5: 0.8400
  P@10: 0.7800
  P@20: 0.6500

Recall@K:
  R@ 1: 0.2100
  R@ 5: 0.5200
  R@10: 0.7300
  R@20: 0.9100

NDCG@K:
  NDCG@ 1: 0.8900
  NDCG@ 5: 0.7600
  NDCG@10: 0.7200
  NDCG@20: 0.6800

MRR: 0.8700
============================================================

✓ All quality targets met
```

### Quality Targets

| Metric | Target | Meaning |
|--------|--------|---------|
| Precision@10 | > 0.7 | At least 70% of top-10 results are relevant |
| NDCG@10 | > 0.65 | Good ranking quality with graded relevance |
| MRR | > 0.8 | First relevant result typically in top 2 |

### Degraded Query Detection

The test automatically flags queries with Precision@10 < 0.5:

```
⚠️  Queries with P@10 < 0.5:
  Query 86 (x): P@10 = 0.20
  Query 91 (get): P@10 = 0.30
  Query 93 (process): P@10 = 0.40
```

**Action**: Review these queries to understand:
1. Is the query inherently difficult (edge case)?
2. Is there a search algorithm issue?
3. Is the ground truth incorrect?

### Category-Specific Analysis

Different categories have different expected performance:

- **Simple Symbols**: P@10 > 0.8 (most precise)
- **Semantic**: NDCG@10 > 0.7 (good ranking important)
- **Code Patterns**: P@10 > 0.7
- **Multi-Word**: NDCG@10 > 0.65 (complex queries)
- **Edge Cases**: Lower metrics expected (stress testing)

## Maintenance Procedures

### Quarterly Review

Every quarter (or after major codebase changes):

1. **Relevance Validation**:
   - Re-run queries against current codebase
   - Verify top results still match ground truth
   - Update relevance scores if code has changed

2. **Ground Truth Updates**:
   - Add new files if they're now more relevant
   - Remove files that no longer exist
   - Adjust relevance scores based on code evolution

3. **Query Refresh**:
   - Retire outdated queries (e.g., removed features)
   - Add queries for new features
   - Maintain 100-query target

### When to Update Ground Truth

**Update ground truth if**:
- Codebase structure significantly changes (e.g., major refactoring)
- New primary implementations replace old ones
- Tests consistently fail due to legitimate code changes

**Don't update ground truth if**:
- Search algorithm changes (tests should catch regressions)
- Temporary code quality dips (tests should fail)
- Individual query failures (investigate root cause first)

### Adding New Queries

When adding queries to the golden set:

1. **Select Query**: Choose from real user searches or common patterns
2. **Determine Category**: Classify into one of the 5 categories
3. **Generate Results**: Run against current search system
4. **Manual Review**: Examine top 20 results
5. **Label Relevance**: Assign 0-3 scores with rationales
6. **Add to JSONL**: Append to `queries.jsonl` and `ground_truth.jsonl`
7. **Assign ID**: Use next available ID (101, 102, etc.)

### Version Control

Track changes to golden test data:

```bash
# Tag baseline version
git tag -a golden-test-v1.0 -m "Initial golden test set"

# Track changes in commits
git add tests/golden/queries.jsonl
git add tests/golden/ground_truth.jsonl
git commit -m "Update ground truth for refactored auth system"
```

## Baseline Scores

### Initial Baseline (2025-01-24)

Performance against 100-query golden test set:

| Category | Count | P@1 | P@10 | NDCG@10 | MRR |
|----------|-------|-----|------|---------|-----|
| Simple Symbols | 20 | TBD | TBD | TBD | TBD |
| Semantic | 25 | TBD | TBD | TBD | TBD |
| Code Patterns | 20 | TBD | TBD | TBD | TBD |
| Multi-Word | 20 | TBD | TBD | TBD | TBD |
| Edge Cases | 15 | TBD | TBD | TBD | TBD |
| **Overall** | **100** | **TBD** | **TBD** | **TBD** | **TBD** |

**Note**: Baseline scores will be established after first complete test run with indexed data.

### Tracking Improvements

Record baseline scores here after each major change:

| Date | Version | P@10 | NDCG@10 | MRR | Notes |
|------|---------|------|---------|-----|-------|
| 2025-01-24 | v0.1.0 | TBD | TBD | TBD | Initial golden test |
| ... | ... | ... | ... | ... | ... |

### Expected Improvements

After implementing optimization tickets:

- **HYBRID_SEARCH-5002 (Fusion Tuning)**: +5-10% NDCG@10
- **HYBRID_SEARCH-5003 (FTS Tuning)**: +3-5% P@10 for simple symbols
- **HYBRID_SEARCH-6001 (Graph Signals)**: +5-8% NDCG@10 for semantic queries

## Best Practices

### For Test Creators

1. **Be Objective**: Base relevance on observable code facts, not opinions
2. **Document Rationale**: Future reviewers need to understand your reasoning
3. **Test Diversity**: Ensure queries span different difficulties
4. **Representative Ground Truth**: Top-10 should cover common result types
5. **Consistent Criteria**: Apply same relevance standards across all queries

### For Test Users

1. **Run Regularly**: Include in CI/CD pipeline
2. **Investigate Failures**: Don't ignore degraded queries
3. **Track Trends**: Monitor metrics over time, not just pass/fail
4. **Category Analysis**: Different categories reveal different issues
5. **Baseline Updates**: Re-establish baseline after major algorithmic changes

### For Codebase Maintainers

1. **Preserve Test Data**: Golden tests are valuable assets
2. **Version Control**: Track ground truth changes like code
3. **Review Updates**: Validate ground truth updates carefully
4. **Document Changes**: Explain why ground truth was updated
5. **Coordinate Refreshes**: Schedule updates with search team

## Troubleshooting

### Low Precision@10

**Symptoms**: Too many irrelevant results in top 10

**Possible Causes**:
- FTS weights too high (matching irrelevant keywords)
- Vector search returning off-topic results
- Metadata signals (recency, churn) dominating

**Investigation**:
1. Examine failed queries by category
2. Check score breakdowns (FTS vs vector vs metadata)
3. Review top results manually
4. Consider fusion weight adjustments

### Low NDCG@10

**Symptoms**: Relevant results present but poorly ranked

**Possible Causes**:
- Fusion algorithm not optimizing for rank quality
- Vector search missing semantic nuances
- Metadata signals disrupting good ranking

**Investigation**:
1. Compare DCG vs IDCG (how far from ideal?)
2. Look at rank positions of highly relevant (grade=3) results
3. Test different fusion strategies (RRF vs weighted)
4. Analyze score distributions

### Low MRR

**Symptoms**: First result often not relevant

**Possible Causes**:
- First result dominated by single signal (e.g., recency)
- FTS exact matching too aggressive
- Vector search not capturing query intent

**Investigation**:
1. Check rank of first relevant result across queries
2. Examine score of rank-1 result vs rank-2, rank-3
3. Consider boosting top results differently
4. Review query preprocessing

## Future Enhancements

Planned improvements to the golden test framework:

1. **Automated Ground Truth Generation**: Use LLM-assisted relevance labeling
2. **Inter-Rater Reliability**: Multiple reviewers for critical queries
3. **User Feedback Integration**: Incorporate real usage data
4. **A/B Testing Support**: Compare two fusion strategies
5. **Temporal Tracking**: Monitor metric trends over time
6. **Category Weighting**: Weight categories by usage frequency
7. **Explainability Reports**: Show why each result was ranked where it was

## References

- Järvelin, K., & Kekäläinen, J. (2002). "Cumulated gain-based evaluation of IR techniques." ACM TOIS.
- Manning, C. D., Raghavan, P., & Schütze, H. (2008). "Introduction to Information Retrieval."
- Robertson, S. (2008). "On GMAP: and other transformations." CIKM 2008.

## Contact

For questions about golden test methodology:
- Search Quality Team
- File issues: `HYBRID_SEARCH-*` tickets
- Documentation updates: `docs/golden_test_methodology.md`
