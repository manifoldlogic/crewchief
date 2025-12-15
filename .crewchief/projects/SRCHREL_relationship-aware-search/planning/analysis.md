# Analysis: Relationship-Aware Search Ranking

## Problem Definition

Maproom's hybrid search currently uses graph signals (PageRank-like importance from `chunk_edges`) as one of four ranking signals, but these graph signals have limited impact on final ranking:

**Current State:**
- Graph executor uses fixed weights: calls (0.3), imports (0.2), tests (0.1)
- Graph signal weight in fusion: 0.10 (10% of final score)
- Result: Important, well-connected code doesn't consistently rank higher than isolated utility functions

**Core Issue:** Graph signals are **underutilized** in search ranking. A central authentication handler with 15 callers and a standalone utility function both get similar treatment in ranking, despite the auth handler being architecturally more significant.

### Concrete Example

Search query: "validate token"

**Current Ranking (without relationship boost):**
```
1. validateTokenFormat (score: 0.85) - utility function, 2 callers
2. TokenValidator class (score: 0.82) - main validator, 15 callers
3. isTokenValid (score: 0.78) - helper function, 1 caller
```

**Desired Ranking (with relationship boost):**
```
1. TokenValidator class (score: 0.89) - boosted due to 15 callers
2. validateTokenFormat (score: 0.85) - still relevant
3. isTokenValid (score: 0.78) - not boosted (few callers)
```

**Impact:** Developers see utility functions ranked above architecturally critical implementations, requiring manual filtering to find the "real" implementation.

## Context

### Prior Work

**SRCHREL Phase 1 (Completed):** Added relationship metadata to search results
- Delivers: `result.related` field with related chunks for high-confidence results
- Purpose: Architectural exploration AFTER finding results
- Limitation: Doesn't affect RANKING - just enriches results

**This Project (SRCHREL Phase 2):** Use relationships to IMPROVE search ranking
- Purpose: Surface architecturally important code in top results
- Mechanism: Boost ranking based on graph centrality (callers, imports, implementations)
- Benefit: Core implementations rank above peripheral utilities

### Current Search Architecture

**Hybrid Search Pipeline:**
```
Query → [FTS, Vector, Graph, Temporal] → RRF Fusion → Final Results
         (parallel executors)              (weighted combination)
```

**Current Fusion Weights:**
```rust
FusionWeights {
  fts: 0.40,      // Keyword matching
  vector: 0.35,   // Semantic similarity
  graph: 0.10,    // Graph importance (UNDERUTILIZED)
  recency: 0.10,  // Recently modified
  churn: 0.05     // Change frequency
}
```

**Graph Executor Logic** (`crates/maproom/src/search/graph.rs`):
```rust
// Fixed weights for different edge types
graph_score = LOG(2 + callers) * 0.3
            + LOG(2 + importers) * 0.2
            + LOG(2 + tests) * 0.1
```

### Research Findings

**From `docs/research/grep-impossible-tasks-report.md`:**
- Relationship discovery tasks: 82% success with semantic search vs 15% with grep
- Architectural understanding: Key differentiator for code search quality
- **Insight:** Users need to find architecturally central code, not just keyword matches

**From completed SRCHREL project:**
- ~20-40% of results qualify for relationship expansion (high confidence)
- Graph traversal overhead: ~8ms per result at depth 2
- **Insight:** Relationship data is already extracted and indexed - we can leverage it for ranking

**From `docs/architecture/MAPROOM_ARCHITECTURE.md`:**
- Graph executor uses PageRank-like scoring from `chunk_edges`
- Current implementation: Simple logarithmic scaling of edge counts
- **Gap:** No differentiation between production code and test code callers

## Existing Solutions

### Industry Patterns

**1. Google PageRank (Web Search)**
- Ranks pages by incoming link quality and quantity
- Dampens low-quality links
- **Adaptation:** Rank code by incoming call/import quality

**2. Sourcegraph Ranking**
- Uses repository popularity, file modification frequency
- Does NOT appear to use intra-codebase graph signals
- **Opportunity:** We have richer graph data via tree-sitter parsing

**3. GitHub Code Search**
- Primarily keyword-based with repository popularity
- No apparent use of call graph for ranking
- **Differentiation:** Our graph-aware ranking is unique

### Codebase Patterns

**Graph Executor** (`crates/maproom/src/search/graph.rs`):
- Already calculates graph importance scores
- Uses `chunk_edges` table with edge types: calls, imports, test_of, extends
- Logarithmic scaling prevents extreme values
- **Limitation:** Fixed weights, no quality differentiation

**Relationship Clustering** (`crates/maproom/src/search/relationships.rs`):
- Implemented edge quality scoring (test penalty, inheritance boost)
- Module proximity detection (same directory = 1.2× boost)
- **Insight:** We can reuse these quality heuristics for ranking

## Current State

### Graph Importance Calculation

**Current Implementation Location:** `crates/maproom/src/db/sqlite/mod.rs::calculate_graph_importance()`

**Database Query Pattern:**
```sql
WITH edge_counts AS (
  SELECT
    dst_chunk_id,
    COUNT(*) FILTER (WHERE type = 'calls') as callers,
    COUNT(*) FILTER (WHERE type = 'imports') as importers,
    COUNT(*) FILTER (WHERE type = 'test_of') as tests
  FROM chunk_edges
  GROUP BY dst_chunk_id
)
SELECT
  chunk_id,
  LOG(2 + callers) * 0.3 +
  LOG(2 + importers) * 0.2 +
  LOG(2 + tests) * 0.1 as graph_score
FROM edge_counts;
```

**Key Observation:** Graph scoring SQL is hardcoded in database layer with fixed weights (0.3, 0.2, 0.1). This is the actual integration point that must be enhanced, not a separate module.

**Problems with Current Approach:**

1. **No Edge Quality:** All callers count equally (test code = production code)
2. **Fixed Weights:** Edge type weights (0.3, 0.2, 0.1) are hardcoded in SQL
3. **No Source Context:** Doesn't consider WHO is calling (important vs unimportant caller)
4. **Low Fusion Weight:** Graph signal only 10% of final score (underutilized)

### Available Graph Data

**`chunk_edges` Table:**
```sql
CREATE TABLE chunk_edges (
  src_chunk_id INTEGER,    -- Caller/importer
  dst_chunk_id INTEGER,    -- Callee/importee
  type TEXT,               -- calls, imports, extends, implements, test_of
  ...
)
```

**Edge Types (Documented):**
- `calls`: Function A calls function B → B is used/important
- `imports`: Module A imports B → B is a dependency
- `extends`: Class A extends B → B is a base class (high importance)
- `implements`: Class A implements interface B → B defines contract
- `test_of`: Test file tests implementation → Indicates production importance

**Coverage:** Populated for TypeScript, JavaScript, Rust, Python during indexing.

**⚠️ VALIDATION REQUIRED:** Before implementation, must verify which edge types actually exist in production database:
```sql
SELECT DISTINCT type FROM chunk_edges LIMIT 100;
```

This validation will confirm whether `extends` and `implements` edges are actually created by tree-sitter indexing, or if they are theoretical. The proposed SQL query depends on these edge types existing.

### Database Schema Assumptions (To Be Validated)

**Chunks Table:**
```sql
CREATE TABLE chunks (
  id INTEGER PRIMARY KEY,
  kind TEXT,        -- "function", "class", "test", etc.
  relpath TEXT,     -- File path relative to repo root
  ...
)
```

**⚠️ VALIDATION REQUIRED:** Actual `kind` values need verification:
```sql
SELECT DISTINCT kind FROM chunks WHERE kind LIKE '%test%' LIMIT 50;
SELECT DISTINCT kind FROM chunks WHERE kind NOT LIKE '%test%' LIMIT 50;
```

This validation will inform test detection heuristic accuracy and determine if file path patterns are necessary.

**Indexes Available:**
```sql
CREATE INDEX idx_chunk_edges_dst ON chunk_edges(dst_chunk_id, type);
CREATE INDEX idx_chunk_edges_src ON chunk_edges(src_chunk_id, type);
CREATE INDEX idx_chunks_file_id ON chunks(file_id);
```

**⚠️ VALIDATION REQUIRED:** Performance testing needed to verify indexes are sufficient for quality-weighted JOIN query:
```sql
EXPLAIN QUERY PLAN
SELECT ... FROM chunk_edges ce
JOIN chunks src ON src.id = ce.src_chunk_id
WHERE ...
```

If EXPLAIN shows full table scan, additional composite index may be needed.

## Research Findings

### Key Insights from SRCHREL Phase 1

**Edge Quality Matters:**
- Production code → production code: High quality signal
- Test code → production code: Medium quality signal
- Production code → test code: Low quality signal (test helper)

**Constants from `relationships.rs`:**
```rust
const EDGE_WEIGHT_DEFAULT: f32 = 1.0;
const EDGE_WEIGHT_TEST_PENALTY: f32 = 0.5;
const EDGE_WEIGHT_INHERITANCE_BOOST: f32 = 1.1;
```

**Validation:** These heuristics improved related chunk relevance in Phase 1.

### Graph Centrality Research

**Key Metrics:**
1. **In-degree:** Number of incoming edges (current approach)
2. **Weighted in-degree:** Quality-adjusted incoming edges (proposed)
3. **PageRank:** Recursive importance from caller importance (future enhancement)

**For MVP:** Weighted in-degree is sufficient (simple, fast, effective).

**Formula:**
```
graph_score = Σ(quality_weight(edge) × source_importance(edge))

where quality_weight considers:
- Edge type (calls > imports)
- Source chunk kind (production > test)
- Target chunk kind (interface/base class boost)
```

### Performance Constraints

**Current Graph Executor Performance:**
- Calculates scores for ALL chunks (not query-specific)
- Pre-computed aggregations in database query
- Latency: ~10-20ms for typical repository

**For Enhanced Approach:**
- Same query pattern (aggregate edges)
- Additional WHERE clauses for edge quality filtering
- Expected latency: <30ms (within acceptable range)

## Constraints

### Technical Constraints

**1. Performance Budget:** <50ms additional latency for graph scoring
- Current graph executor: ~10-20ms
- Enhanced quality scoring: Expected +10-20ms
- Total: <30-40ms (acceptable within 100ms p95 target)

**2. Database Schema:** No schema changes allowed
- Must use existing `chunk_edges` table
- Cannot add new columns
- Can add indexes if needed (but existing indexes likely sufficient)

**3. Backward Compatibility:**
- Enhanced scoring is an improvement, not a breaking change
- No API changes required
- Existing fusion weights can be tuned without code changes (configuration)

**4. Type System:**
- Rust implementation (graph executor)
- No TypeScript changes needed (server-side only)

### Business Constraints

**1. MVP Scope:**
- Focus on quality-weighted in-degree (not full PageRank)
- Reuse edge quality heuristics from SRCHREL Phase 1
- Defer: Recursive importance calculation, ML-based edge weights

**2. Configuration:**
- Edge quality weights should be configurable (not hardcoded)
- Allow tuning without code redeployment
- Configuration file: `crates/maproom/config/maproom-search.yml`

**3. Validation:**
- Must improve ranking quality (measured via test queries)
- Should not degrade performance beyond budget
- Must not change API contract

### Operational Constraints

**1. Index Requirements:**
- Existing indexes on `chunk_edges(src_chunk_id)` and `chunk_edges(dst_chunk_id)`
- Should be sufficient for quality-weighted queries
- Validate with EXPLAIN QUERY PLAN

**2. Monitoring:**
- Track graph executor latency (p50, p95, p99)
- Monitor fusion weight impact on result quality
- Log edge quality distributions for tuning

**3. Rollout:**
- Feature flag: `enable_enhanced_graph_scoring`
- Gradual rollout: Compare rankings with flag on/off
- A/B test: Measure user satisfaction (query reformulation rate)

## Success Criteria

### Must Achieve

**1. Improved Ranking Quality**
- **Metric:** Architecturally important code ranks in top 3 results
- **Test:** Search for "authentication" → auth handler (15 callers) ranks above utils (2 callers)
- **Validation:** Manual evaluation on 50 representative queries

**2. Performance Within Budget**
- **Metric:** Graph executor latency <40ms p95 (was ~20ms, budget +20ms)
- **Test:** Benchmark on large repository (100K chunks, 500K edges)
- **Validation:** Performance regression tests

**3. Configurable Edge Weights**
- **Metric:** Edge quality weights in configuration file
- **Test:** Change weights, verify ranking changes
- **Validation:** Integration tests with different configurations

### Should Achieve

**1. Quality-Aware Edge Scoring**
- **Metric:** Production code callers weighted 2× test code callers
- **Test:** Function called by 5 prod + 5 test ranks above function called by 15 tests
- **Validation:** Unit tests for edge quality computation

**2. Inheritance/Interface Boost**
- **Metric:** Base classes/interfaces rank higher (1.5× boost)
- **Test:** Search "Controller" → BaseController ranks above specific controllers
- **Validation:** Test queries on class hierarchies

**3. Fusion Weight Optimization**
- **Metric:** Find optimal graph fusion weight (current: 0.10, target: 0.15-0.25)
- **Test:** Grid search on validation queries
- **Validation:** Ranking quality improvement on test set

### Nice to Have

**1. Source Importance (Recursive)**
- **Metric:** Caller importance propagates (called by important caller = more important)
- **Test:** Utility called by auth handler ranks above utility called by test
- **Defer:** Full PageRank calculation (complexity vs benefit)

**2. Negative Signals**
- **Metric:** Deprecated code (no callers) ranks lower
- **Test:** Search "logger" → active logger ranks above unused legacy logger
- **Implementation:** Inverse weight for 0 callers (small penalty)

**3. Cross-Repository Importance**
- **Metric:** Core library functions rank higher across dependent repos
- **Defer:** Requires cross-repo graph (out of MVP scope)

## Open Questions

**1. Edge Quality Weight Ranges**
- Question: What weight should test code callers receive?
- Option A: 0.5× (same as SRCHREL Phase 1)
- Option B: 0.3× (stronger penalty)
- **Recommendation:** Start with 0.5× (validated), tune based on feedback

**2. Logarithmic Scaling**
- Question: Keep LOG(2 + count) or try linear scaling?
- Current: Prevents extreme values (100 callers ≈ 6.6, 10 callers ≈ 3.5)
- Alternative: Linear with cap (max 10 callers count)
- **Recommendation:** Keep logarithmic (proven to work well)

**3. Fusion Weight Adjustment**
- Question: Increase graph weight from 0.10 to what value?
- Option A: 0.15 (modest increase)
- Option B: 0.20 (balanced with FTS/vector)
- Option C: Grid search for optimal value
- **Recommendation:** Start 0.15, validate, consider grid search in Phase 2

**4. Bi-directional Edges**
- Question: Should outgoing edges (callees, exports) contribute to importance?
- Current: Only incoming edges (callers, importers)
- Insight: High out-degree = utility/helper pattern (less important)
- **Recommendation:** Incoming only for MVP (cleaner signal)

**5. Edge Recency**
- Question: Should recent edges count more than old edges?
- Use case: Recently added callers = active usage
- Complexity: Requires edge timestamps (not in current schema)
- **Recommendation:** Defer to Phase 2 (would require schema change)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance degradation (>50ms overhead) | Low | High | Benchmark early, optimize query, add timeout |
| Ranking quality doesn't improve | Medium | High | Validate on test queries, A/B test, rollback flag |
| Edge quality heuristics wrong | Medium | Medium | Make configurable, monitor metrics, iterate weights |
| Database index inefficiency | Low | Medium | EXPLAIN queries, add indexes if needed, optimize SQL |
| Configuration complexity | Low | Low | Simple YAML config, validation tests, defaults |

## Alternatives Considered

### Alternative 1: Full PageRank Implementation

**Approach:** Recursive importance calculation (caller's importance influences callee's importance).

**Pros:**
- More sophisticated (Google's algorithm)
- Captures transitive importance

**Cons:**
- Complex implementation (iterative convergence)
- Higher latency (multiple graph traversals)
- Harder to debug and tune

**Decision:** Rejected for MVP. Quality-weighted in-degree is simpler and likely 80% as effective.

### Alternative 2: ML-Based Edge Weights

**Approach:** Train model to predict edge importance from features (caller kind, edge type, module distance).

**Pros:**
- Data-driven weights
- Can discover non-obvious patterns

**Cons:**
- Requires labeled training data (which queries found best results)
- Model training/deployment complexity
- Harder to explain to users

**Decision:** Rejected for MVP. Heuristic weights (validated in Phase 1) are good starting point. ML can be Phase 3.

### Alternative 3: Pre-Compute Graph Scores

**Approach:** Calculate and store graph scores during indexing, not at query time.

**Pros:**
- Zero query-time latency
- Can use more expensive algorithms (PageRank)

**Cons:**
- Stale scores (need re-indexing on code changes)
- Schema change required (new column)
- Harder to tune (need re-indexing to adjust weights)

**Decision:** Rejected for MVP. Query-time calculation is flexible and fresh. Consider materialized view in Phase 2 if latency is issue.

## Key Insights

**1. Graph Data is Gold, Currently Under-Used**
- We have rich relationship data from tree-sitter parsing
- Current graph executor uses simple edge counts
- Opportunity: Quality-weighted edges for smarter ranking

**2. SRCHREL Phase 1 Validated Edge Quality Heuristics**
- Test code penalty (0.5×) improves relevance
- Inheritance boost (1.1×) surfaces base classes
- Module proximity (1.2×) favors same-directory relationships
- **Reuse:** Apply same heuristics to ranking (proven to work)

**3. Simple Weighted In-Degree is Sufficient for MVP**
- Full PageRank is overkill (complex, slow, hard to tune)
- Weighted in-degree captures 80% of value with 20% of complexity
- Can always upgrade to PageRank in Phase 2 if needed

**4. Configuration is Critical**
- Edge weights will need tuning based on codebase characteristics
- YAML configuration enables tuning without code redeployment
- Feature flag enables safe rollout and A/B testing

**5. Performance Headroom Exists**
- Current graph executor: ~20ms
- Budget: <100ms p95 end-to-end
- Headroom: Can afford +20-30ms for quality improvement
- Optimization: Can pre-compute if needed (materialized view)
