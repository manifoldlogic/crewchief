# Analysis: Confidence Scoring

## Problem Definition

Search results in maproom return relevance scores (0.0-1.0 from FTS, cosine similarity from vector search, or fused scores from RRF), but users cannot assess **result quality** or **confidence** from these scores alone. A high score doesn't indicate whether:

1. **The result is reliable** - Is this a strong match or the best of weak options?
2. **The search coverage was complete** - Did we search all relevant sources (FTS, vector, graph)?
3. **The query was well-formed** - Did the query successfully match expected patterns?
4. **Multiple signals agree** - Do different scoring methods corroborate this result?

**Core Issue**: Relevance scores measure "how well this result matches the query" but not "how confident should you be in this result." Users need confidence indicators to:
- Trust top results when confidence is high
- Investigate further when confidence is low
- Understand when results are marginal vs definitive
- Prioritize follow-up actions based on result quality

## Context

### Prerequisites

This project builds on completed Phase 1 work from the parent initiative:

1. **SRCHTRN (Search Transparency)** - COMPLETE (archived)
   - Delivered `QueryUnderstanding` struct (in `crates/maproom/src/search/results.rs` lines 128-178)
   - Provides query interpretation metadata (mode detection, token expansion)
   - Optional field in `SearchMetadata` via `understanding: Option<QueryUnderstanding>`

2. **SRCHFLTR (Result Filtering)** - COMPLETE (archived)
   - Delivered result filtering infrastructure
   - Provides cleaner result sets for confidence scoring

**Status**: All dependencies complete. SRCHCONF is ready to proceed as Phase 2 project.

### Current Search System

Maproom uses a sophisticated hybrid search architecture:

1. **Search Modes** (SearchMode enum):
   - `Code` - Code-focused queries with patterns like `::`
   - `Text` - Natural language queries
   - `Auto` - Automatic mode detection

2. **Search Executors** (parallel execution):
   - **FTS** - SQLite FTS5 full-text search with semantic ranking (kind multipliers, exact match bonuses)
   - **Vector** - sqlite-vec cosine similarity search (1024-dim embeddings)
   - **Graph** - Relationship traversal (callers/callees, imports)
   - **Signals** - Recency, churn, importance signals

3. **Score Fusion**:
   - **RRF (Reciprocal Rank Fusion)** - Default, rank-based fusion resistant to score distribution differences
   - **Basic Weighted** - Simple weighted average of normalized scores

4. **Result Assembly**:
   - Final results include: `chunk_id`, `score`, `source_scores` (per-executor scores), metadata
   - Optional debug mode shows score breakdown (base_fts, kind_multiplier, exact_match_multiplier)

### Search Result Structure

Current `ChunkSearchResult` (from `crates/maproom/src/search/results.rs`):
```rust
pub struct ChunkSearchResult {
    pub chunk_id: i64,
    pub score: f32,              // Final fused score
    pub source_scores: HashMap<SearchSource, f32>,  // Individual executor scores
    // ... file/symbol metadata
}
```

Current `SearchMetadata`:
```rust
pub struct SearchMetadata {
    pub query_processing: QueryProcessingDetails,
    pub result_counts: HashMap<SearchSource, usize>,  // Results per executor
    pub timing: SearchTiming,
    pub total_unique_chunks: usize,
    pub returned_results: usize,
    pub understanding: Option<QueryUnderstanding>,   // Query interpretation
}
```

### What Users Currently See

Via MCP search tool, users receive:
- Array of results with scores (e.g., `[{score: 6.375, ...}, {score: 4.2, ...}]`)
- Optional debug breakdown showing score components
- Metadata about query understanding (mode, tokens, timing)

**Gap**: No way to know if score 6.375 is "very confident" or "best of bad options."

## Existing Solutions

### Industry Patterns

1. **Elasticsearch Confidence Scoring**:
   - Returns `max_score` alongside results to contextualize individual scores
   - Provides `_explanation` API for score debugging
   - No built-in confidence metric - relies on score interpretation

2. **Google Search Quality**:
   - Uses implicit confidence signals (featured snippets, "People also ask")
   - Rich results indicate high confidence matches
   - No exposed confidence score to users

3. **Vector Database Patterns** (Pinecone, Weaviate):
   - Cosine similarity scores naturally bounded [0, 1]
   - Distance thresholds indicate confidence (e.g., >0.8 = high confidence)
   - Often use score distribution analysis

4. **Academic IR (Information Retrieval)**:
   - **Query Performance Prediction (QPP)** - Predict query difficulty before execution
   - **Result Set Coherence** - Measure agreement between top results
   - **Score Gap Analysis** - Large gap between #1 and #2 indicates confidence

### Codebase Patterns

Maproom already has foundational confidence indicators:

1. **Source Score Agreement** (`source_scores` map):
   - If FTS, vector, and graph all score a result highly → high confidence
   - If only one source scores highly → lower confidence

2. **Debug Mode Score Breakdown**:
   - Shows `base_fts`, `kind_multiplier`, `exact_match_multiplier`
   - Exact match (3.0×) indicates high confidence
   - High kind multiplier (2.5× for func) indicates implementation match

3. **Query Understanding Metadata**:
   - `mode` detection (code vs text) indicates query clarity
   - `expanded_terms` count shows query expansion success
   - `result_counts` per source shows coverage

4. **Search Metadata**:
   - `total_unique_chunks` shows search breadth
   - `returned_results` vs limit shows saturation

## Research Findings

### Key Insights from Codebase Analysis

1. **Existing Confidence Signals**:
   - **Source agreement**: Results appearing in multiple executors (FTS + vector + graph) are more reliable
   - **Score separation**: Large gap between top result and next result indicates confidence
   - **Exact matches**: Exact symbol name matches (3.0× multiplier) are high confidence
   - **Result saturation**: Returning < limit results may indicate low confidence (few matches)

2. **RRF Fusion Properties**:
   - RRF scores are comparable across queries (rank-based, not score-distribution dependent)
   - Top result in RRF appears in multiple sources at high ranks → inherent confidence signal
   - Formula: `score = sum(1.0 / (k + rank + 1.0))` across sources

3. **Semantic Ranking Signals**:
   - Kind multipliers (2.5× for func, 0.6× for tests) already encode result quality
   - Exact match bonus (3.0×) is a strong confidence indicator
   - Base FTS scores correlate with keyword frequency (high frequency ≠ high confidence)

4. **Metadata Richness**:
   - `QueryUnderstanding` provides transparency but not confidence
   - `TimingBreakdown` useful for performance but not confidence
   - `result_counts` per source useful for coverage assessment

### Confidence Scoring Approaches

**Option A: Multi-Signal Confidence Score**
- Combine: source agreement, score gap, exact match, result saturation
- Output: Single confidence value [0.0, 1.0] per result
- **Pros**: Simple, interpretable, easy to display
- **Cons**: Loses signal details, requires tuning weights

**Option B: Confidence Components**
- Expose: `source_count`, `score_gap`, `is_exact_match`, `coverage_ratio`
- Output: Structured object with individual signals
- **Pros**: Transparent, flexible, no weight tuning
- **Cons**: More complex for users to interpret

**Option C: Confidence Categories**
- Map signals to: `HIGH`, `MEDIUM`, `LOW` confidence
- Output: Categorical label per result
- **Pros**: Very simple, actionable
- **Cons**: Loses nuance, threshold tuning needed

**Recommendation**: **Option B (Confidence Components)** for MVP
- Most transparent and flexible
- Allows users to interpret based on context
- No magic weights or thresholds to tune
- Can add Option C (categories) in Phase 2 if needed

**Initiative Alignment Note**:
The parent initiative specified "0-100 confidence scale" and "confidence bands (HIGH/MEDIUM/LOW)", but this planning chooses the component-based approach instead. This deviation is justified:
- Component-based is more transparent (no magic normalization)
- Aligns with maproom's principle: expose data, don't hide complexity
- Follows existing debug mode pattern (multiple score components)
- More flexible for different use cases
- Can add derived categories in Phase 2 if user feedback requests them

This approach prioritizes transparency over simplicity, which is more consistent with maproom's design philosophy.

## Current State

### What Exists Today

1. **Raw Scoring Data** (`ChunkSearchResult`):
   - `score` - Final fused score
   - `source_scores` - Individual executor scores

2. **Search Metadata** (`SearchMetadata`):
   - `result_counts` - Results per executor
   - `total_unique_chunks` - Total matches
   - `returned_results` - Actual returned count
   - `understanding: Option<QueryUnderstanding>` - Query interpretation metadata (from SRCHTRN Phase 1)

3. **Debug Mode** (optional):
   - `score_breakdown` with FTS components
   - Available via `debug: true` parameter

4. **Query Understanding** (from Phase 1 SRCHTRN):
   - `QueryUnderstanding` struct already exists and provides query-level transparency
   - Includes mode detection, token expansion, timing
   - Optional field following same pattern confidence will use

### What's Missing

1. **Per-Result Confidence Indicators** (MVP Scope - 3 core signals):
   - No measure of source agreement → Need `source_count`
   - No score gap calculation → Need `score_gap`
   - No exact match indicator → Need `is_exact_match` (always available, not just debug)

2. **Deferred to Phase 2** (validate core signals first):
   - `relative_score` - Result score / top score
   - `rank` - Position in result list
   - Query-level confidence summary
   - Categorical confidence bands (if user feedback requests)

3. **Out of Scope for MVP**:
   - Automatic filtering based on confidence (progressive cutoff)
   - 0-100 score normalization
   - Confidence-based result reranking

## Constraints

### Technical Constraints

1. **Type Synchronization**:
   - Rust structs in `crates/maproom/src/search/results.rs` must sync with TypeScript in `packages/daemon-client/src/types.ts`
   - Changes require updates to both sides with `TYPE_SYNC` comments

2. **Backward Compatibility**:
   - MCP tools used by VS Code extension, Claude Code CLI
   - Cannot break existing response structure
   - Must use optional fields for new data

3. **Performance**:
   - Confidence calculation must not significantly impact search latency
   - Target: <50ms p95 total search time (currently 40ms after SEMRANK)
   - Confidence computation should be <5ms overhead

4. **SQLite Limitations**:
   - All data computed from existing in-memory structures after fusion
   - No additional database queries for confidence scoring
   - Stateless computation only

### Business Constraints

1. **MVP Scope**:
   - Ship value, not ceremonies
   - Focus on transparency over complex ML models
   - Avoid tuning weights/thresholds that require data science expertise

2. **User Experience**:
   - Confidence signals must be interpretable
   - Should help users make decisions (trust result, investigate further, refine query)
   - Avoid overwhelming with too many metrics

3. **Development Time**:
   - Leverage existing data structures
   - Minimize new code paths
   - Reuse existing debug mode patterns

## Success Criteria

### Functional Success

1. **Confidence Signals Exposed**:
   - ✅ Each search result includes confidence components
   - ✅ Components include: source count, score gap, exact match flag, relative score
   - ✅ Query-level confidence summary available in metadata

2. **Type Synchronization**:
   - ✅ Rust structs match TypeScript interfaces with TYPE_SYNC comments
   - ✅ Serialization roundtrip tests pass
   - ✅ No type errors in maproom-mcp or vscode-maproom

3. **Backward Compatibility**:
   - ✅ Existing search responses unchanged (confidence fields optional)
   - ✅ MCP tool continues working for all consumers
   - ✅ Debug mode still functions as before

### Quality Success

1. **Interpretability**:
   - ✅ Confidence signals are self-explanatory (e.g., `source_count: 3` = "appeared in 3 search sources")
   - ✅ Documentation explains each confidence component
   - ✅ Examples show high vs low confidence scenarios

2. **Performance**:
   - ✅ Confidence computation adds <5ms overhead
   - ✅ Search latency remains <50ms p95
   - ✅ No performance regression in benchmarks

3. **Transparency**:
   - ✅ Users understand why a result has high/low confidence
   - ✅ Confidence signals help users refine queries when needed
   - ✅ No "black box" scoring - all signals traceable to source data

### User Success Criteria

1. **Developers can assess result reliability**:
   - High confidence results (3+ sources, exact match, large score gap) → trust immediately
   - Low confidence results (1 source, no exact match, small gap) → verify manually

2. **Developers can improve queries**:
   - Low source count → query too specific or narrow
   - Low score gap → query ambiguous, multiple equally good matches
   - No exact matches → query doesn't align with symbol names

3. **Developers understand search coverage**:
   - Query-level metrics show which executors contributed
   - Coverage ratio shows percentage of possible sources used
   - Result saturation shows if more results exist

### Measurable Outcomes

- **Adoption**: Confidence fields accessed in >50% of search API calls (track via logging)
- **Utility**: Developers report confidence signals reduce manual verification time
- **Accuracy**: High confidence results (3+ sources, exact match) have >90% user acceptance
- **Performance**: <5ms overhead, <50ms p95 latency maintained
