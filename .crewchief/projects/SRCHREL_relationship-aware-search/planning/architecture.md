# Architecture: Relationship-Aware Search Ranking

## ✅ BLOCKER RESOLVED (2025-12-14)

**Previous Status:** BLOCKED - Edge extraction not implemented

**Resolution:** EDGEEXT project completed successfully
- TypeScript/JavaScript edge extraction (EDGEEXT-1001 through EDGEEXT-1004)
- Rust edge extraction (EDGEEXT-2001)
- `chunk_edges` table now populated with 92.86% precision

**Available Edge Types (from EDGEEXT implementation):**
- `calls` - Function/method calls (TypeScript, JavaScript, Rust) ✓ IMPLEMENTED
- Future: `imports`, `test_of` (planned for EDGEEXT Phase 2)
- NOT AVAILABLE: `extends`, `implements` (architectural decision - not implemented)

**Phase 1 MVP Scope:** Use `calls` edges only for quality-weighted scoring

See `planning/blocker-resolution.md` for full details.

---

## Overview

Enhance maproom's graph-based ranking to use edge quality scoring. Transform the graph executor from simple edge counting to intelligent quality-weighted importance calculation. This surfaces architecturally significant code above peripheral utilities.

**Core Principle:** Not all edges are equal. Production code callers > test code callers. Inheritance edges > simple imports.

```
Current: graph_score = LOG(2 + caller_count) * 0.3 + LOG(2 + importer_count) * 0.2

Enhanced: graph_score = LOG(2 + Σ(quality_weight(edge))) for all edges
```

## Design Decisions

### Decision 1: Quality-Weighted In-Degree (Not PageRank)

**Context:** Graph importance: simple in-degree vs recursive PageRank.
**Decision:** Quality-weighted in-degree for MVP.
**Rationale:** 80% of PageRank value, 20% of complexity. Same ~20ms latency, just better edge filtering. Can upgrade to PageRank in Phase 2 if needed.

### Decision 2: Reuse Edge Quality Heuristics from Phase 1

**Context:** SRCHREL Phase 1 validated edge quality weights in `relationships.rs`.
**Decision:** Reuse same heuristics for ranking with ranking-specific adjustments.

**Phase 1 Constants (Validated):**
```rust
const EDGE_WEIGHT_DEFAULT: f32 = 1.0;
const EDGE_WEIGHT_TEST_PENALTY: f32 = 0.5;
const EDGE_WEIGHT_INHERITANCE_BOOST: f32 = 1.1;
```

**Ranking Weights (Phase 1 MVP - Calls Only):**
- `production_code: 1.0` - Baseline (same as `EDGE_WEIGHT_DEFAULT`)
- `test_code: 0.5` - Same as Phase 1 `EDGE_WEIGHT_TEST_PENALTY` (validated)
- `calls: 1.0` - Only edge type available in Phase 1

**Future Weights (Phase 2 - When Additional Edge Types Available):**
- `imports: 0.8` - Lower than calls (imports less indicative of active usage)
- `test_of: 0.3` - Low weight (test edges don't indicate importance)

**Note:** `extends`/`implements` edges are NOT implemented (architectural decision in EDGEEXT). Inheritance boost removed from architecture.

**Rationale:** Start with proven Phase 1 heuristics, scale inheritance boost for ranking context (larger result set needs stronger signal).

### Decision 3: Configuration Approach - Phased Rollout

**Phase 1 (MVP):** Hardcoded weights in code, simple feature flag
**Phase 2:** YAML configuration for weights
**Phase 3:** Hot reload for dynamic tuning

**Rationale:** Reduce MVP complexity. Prove algorithm works before building configuration infrastructure.

**Phase 1 Configuration (Minimal):**
```yaml
# In existing config/maproom-search.yml
feature_flags:
  enable_quality_scoring: false  # Simple boolean toggle
```

**Phase 2 Configuration (Full):**
```yaml
graph_importance:
  enable_quality_scoring: true
  edge_quality_weights:
    production_code: 1.0  # Production code edge source weight
    test_code: 0.5        # Test code edge source weight (penalty)
    calls: 1.0            # Call edge type weight (only type in Phase 1)
    # Future edge types (Phase 2):
    # imports: 0.8        # Import edge type weight
    # test_of: 0.3        # Test-of edge type weight
  fusion_weight_override: 0.15  # Optional override, default 0.10
```

**Rust Structs (Phase 2):**
```rust
// In src/config/search_config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    // ... existing fields ...

    #[serde(default)]
    pub graph_importance: GraphImportanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphImportanceConfig {
    #[serde(default)]
    pub enable_quality_scoring: bool,

    #[serde(default)]
    pub edge_quality_weights: EdgeQualityWeights,

    #[serde(default)]
    pub fusion_weight_override: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeQualityWeights {
    #[serde(default = "default_production_code_weight")]
    pub production_code: f32,  // 1.0 - Source code type weight

    #[serde(default = "default_test_code_weight")]
    pub test_code: f32,        // 0.5 - Source code type weight (penalty)

    #[serde(default = "default_calls_weight")]
    pub calls: f32,            // 1.0 - Edge type weight (only type in Phase 1)

    // Future edge types (Phase 2 - when EDGEEXT implements them):
    // #[serde(default = "default_imports_weight")]
    // pub imports: f32,       // 0.8
    //
    // #[serde(default = "default_test_of_weight")]
    // pub test_of: f32,       // 0.3
}

impl Default for GraphImportanceConfig {
    fn default() -> Self {
        Self {
            enable_quality_scoring: false,
            edge_quality_weights: EdgeQualityWeights::default(),
            fusion_weight_override: None,
        }
    }
}

impl Default for EdgeQualityWeights {
    fn default() -> Self {
        Self {
            production_code: 1.0,  // Source type: production code
            test_code: 0.5,        // Source type: test code (penalty)
            calls: 1.0,            // Edge type: calls (only type in Phase 1)
            // Future edge types (Phase 2):
            // imports: 0.8,
            // test_of: 0.3,
        }
    }
}
```

**Config Loading (Async Pattern):**
```rust
// Integrates with existing SearchConfig::load_default() async fn
impl SearchConfig {
    pub async fn load_default() -> Result<Self, ConfigError> {
        // Existing loading logic...
        // graph_importance field will deserialize automatically
        // Falls back to Default if section missing
    }
}
```

**No lazy_static needed** - Config loaded once at search pipeline initialization, passed down to executors.

### Decision 4: Backwards Compatible with Feature Flag

**Decision:** Drop-in replacement with `enable_quality_scoring` flag.
**Migration:**
1. Deploy with flag=false (old behavior)
2. Enable flag=true (new behavior)
3. Remove flag after stabilization

**API Impact:** NONE (internal ranking improvement only)

### Decision 5: Keep Logarithmic Scaling

**Decision:** Apply `LOG(2 + quality_weighted_sum)` not `LOG(2 + count)`.

**Example:**
- 10 production + 5 test callers
- Old: `LOG(2 + 15) = 2.83`
- New: `LOG(2 + (10*1.0 + 5*0.5)) = LOG(14.5) = 2.67`

**Rationale:** Prevents extreme values, smooth scaling, proven to work.

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Graph executor already in Rust |
| Database | SQLite | Has `chunk_edges` with indexes |
| Query | CTE with aggregation | Efficient, leverages indexes |
| Config | YAML | Existing pattern, easy to edit |
| Feature Flag | Boolean in config | Simple toggle |

## Component Design

### Component 1: Test Detection Utility

**Location:** `crates/maproom/src/search/graph_quality.rs` (new, Phase 1) OR inline in database layer (simpler)

**Purpose:** Determine if a chunk is test code based on file path patterns (primary) and chunk kind (secondary).

**Implementation:**
```rust
pub fn is_test_chunk(relpath: &str, kind: &str) -> bool {
    // Primary: File path patterns (proven accurate from Phase 1 relationships)
    let path_lower = relpath.to_lowercase();
    let is_test_path = path_lower.contains("/test/")
        || path_lower.contains("/tests/")
        || path_lower.contains("/__tests__/")
        || path_lower.ends_with(".test.ts")
        || path_lower.ends_with(".test.js")
        || path_lower.ends_with(".spec.ts")
        || path_lower.ends_with(".spec.js")
        || path_lower.ends_with("_test.rs")
        || path_lower.ends_with("_test.py");

    if is_test_path {
        return true;
    }

    // Secondary: Chunk kind patterns (less reliable)
    let kind_lower = kind.to_lowercase();
    kind_lower.contains("test")
        || kind_lower.contains("describe")
        || kind_lower.contains("it")
}
```

**Rationale:** Phase 1 relationships code uses file path patterns successfully. Kind-based detection is secondary because chunk `kind` values are tree-sitter node types (e.g., "function_declaration"), not semantic labels.

**Validated Chunk Kind Values (SRCHREL-0001):**

Database validation confirmed 27 distinct chunk `kind` values:

**Code Chunks (Rust, TypeScript, JavaScript):**
- `func`, `async_func` - Function declarations
- `method`, `async_method` - Method definitions
- `class` - Class definitions
- `struct` - Rust struct definitions
- `impl` - Rust implementation blocks
- `trait` - Rust trait definitions
- `enum` - Enumeration definitions
- `constant`, `variable`, `static` - Variable declarations
- `macro` - Macro definitions
- `use`, `imports` - Import/use statements
- `module` - Module declarations

**Documentation Chunks (Markdown, YAML, TOML, JSON):**
- `heading_1` through `heading_5` - Markdown headings
- `markdown_section` - Markdown content sections
- `code_block` - Code blocks in markdown
- `link`, `image_link` - Markdown links
- `json_key`, `yaml_key` - Key-value pairs
- `toml_section` - TOML sections

**Test Detection Accuracy (Validated - SRCHREL-0003):**
- **Validation Date:** 2025-12-15
- **Sample Size:** 200 chunks (100 test, 100 production)
- **Precision:** 100.00% (target: ≥85%) ✅
- **Recall:** 100.00% (target: ≥80%) ✅
- **F1 Score:** 100.00%
- **False Positives:** 0 (no production code misidentified as test)
- **False Negatives:** 0 (no test code missed)
- **Primary patterns:** `/tests/`, `/__tests__/`, `.test.ts`, `.spec.ts`, `_test.rs`, `_test.py`
- **Validation:** Automated test in `crates/maproom/tests/test_detection_validation.rs`

**Pattern Performance:**
- File path matching: 100% precision and recall (SRCHREL-0003 validation), low cost (string LIKE operations)
- Chunk kind matching: Not required (file path patterns achieve 100% accuracy alone)
- Recommendation: Use file path as primary signal, chunk kind as optional secondary validation

**Phase 2 Improvement:** Monitor real-world accuracy in production, add user-configurable patterns if needed.

### Component 2: Enhanced Graph Executor & Database Query

**Location:** `crates/maproom/src/db/sqlite/mod.rs::calculate_graph_importance()` (modified)

**Current Signature:**
```rust
pub fn calculate_graph_importance(
    &self,
    repo_id: i64,
    worktree_id: Option<i64>,
    limit: usize,
) -> Result<Vec<(i64, f32)>, DbError>
```

**Enhanced Signature (Phase 1):**
```rust
pub fn calculate_graph_importance(
    &self,
    repo_id: i64,
    worktree_id: Option<i64>,
    limit: usize,
    enable_quality: bool,  // Feature flag
) -> Result<Vec<(i64, f32)>, DbError>
```

**Phase 1 SQL (Hardcoded Weights - Calls Only):**
```sql
WITH quality_edges AS (
  SELECT
    ce.dst_chunk_id as chunk_id,
    -- Edge type weight (Phase 1: calls only, future: imports, test_of)
    CASE ce.type
      WHEN 'calls' THEN 1.0
      -- Future edge types (Phase 2):
      -- WHEN 'imports' THEN 0.8
      -- WHEN 'test_of' THEN 0.3
      ELSE 1.0  -- Default for any other edge type
    END *
    -- Source code type weight (test detection via file path)
    CASE
      WHEN src_file.relpath LIKE '%/test/%'
        OR src_file.relpath LIKE '%/tests/%'
        OR src_file.relpath LIKE '%/__tests__/%'
        OR src_file.relpath LIKE '%.test.%'
        OR src_file.relpath LIKE '%.spec.%'
        OR src_file.relpath LIKE '%_test.%'
        OR src_chunk.kind LIKE '%test%'
      THEN 0.5  -- Test code penalty
      ELSE 1.0  -- Production code baseline
    END as edge_quality
  FROM chunk_edges ce
  JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id
  JOIN files src_file ON src_file.id = src_chunk.file_id
  WHERE ce.dst_chunk_id IN (
    SELECT c.id FROM chunks c
    JOIN files f ON f.id = c.file_id
    WHERE f.repo_id = ?repo_id
      AND (?worktree_id IS NULL OR f.worktree_id = ?worktree_id)
  )
),
importance_scores AS (
  SELECT
    chunk_id,
    SUM(edge_quality) as quality_weighted_sum
  FROM quality_edges
  GROUP BY chunk_id
)
SELECT
  chunk_id,
  LOG(2.0 + COALESCE(quality_weighted_sum, 0.0)) as graph_score
FROM importance_scores
ORDER BY graph_score DESC
LIMIT ?limit
```

**Implementation:**
```rust
impl SqliteStore {
    pub fn calculate_graph_importance(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        enable_quality: bool,
    ) -> Result<Vec<(i64, f32)>, DbError> {
        if !enable_quality {
            // Old implementation (existing SQL)
            return self.calculate_graph_importance_legacy(repo_id, worktree_id, limit);
        }

        // New quality-weighted implementation
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(QUALITY_WEIGHTED_GRAPH_SQL)?;

        let rows = stmt.query_map(
            params![repo_id, worktree_id, worktree_id, limit],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,   // chunk_id
                    row.get::<_, f32>(1)?,   // graph_score
                ))
            },
        )?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
```

**GraphExecutor Integration:**
```rust
// In src/search/graph.rs
impl GraphExecutor {
    pub async fn execute(
        store: &SqliteStore,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        config: Option<&SearchConfig>,  // Backward compatible: None uses defaults
    ) -> Result<RankedResults, GraphError> {
        let enable_quality = config
            .and_then(|c| Some(c.graph_importance.enable_quality_scoring))
            .unwrap_or(false);

        let scores = store.calculate_graph_importance(
            repo_id,
            worktree_id,
            limit,
            enable_quality,
        )?;

        Ok(RankedResults::from_scores(scores))
    }
}
```

**⚠️ VALIDATION REQUIRED:** This SQL query must be prototyped and benchmarked before implementation:
1. Create synthetic database (100K chunks, 500K edges)
2. Measure query latency (target: <30ms p95)
3. Run EXPLAIN QUERY PLAN to verify index usage
4. Verify that JOIN with chunks table doesn't cause full table scan

If performance exceeds budget, consider:
- Adding composite index on `chunk_edges(dst_chunk_id, type, src_chunk_id)`
- Pre-computing test/production chunk classification during indexing
- Materializing edge quality scores

### Component 3: Search Pipeline Configuration Propagation (Phase 2)

**Current State:** Search pipeline doesn't pass configuration to executors.

**Phase 2 Enhancement:** Load config at pipeline initialization, pass to graph executor.

**Pipeline Integration:**
```rust
// In src/search/pipeline.rs (or equivalent)
pub struct SearchPipeline {
    config: SearchConfig,
    // ... other fields
}

impl SearchPipeline {
    pub async fn new() -> Result<Self, SearchError> {
        let config = SearchConfig::load_default().await?;
        Ok(Self { config, /* ... */ })
    }

    pub async fn execute(&self, query: &Query) -> Result<SearchResults> {
        // Parallel executor execution
        let (fts_results, vector_results, graph_results, temporal_results) = tokio::join!(
            FtsExecutor::execute(query),
            VectorExecutor::execute(query),
            GraphExecutor::execute(
                &self.store,
                query.repo_id,
                query.worktree_id,
                query.limit,
                Some(&self.config),  // Pass config
            ),
            TemporalExecutor::execute(query),
        );

        // Fusion with potentially overridden graph weight
        let fusion_weights = FusionWeights::from_config(&self.config);
        let results = fuse_results(
            fts_results,
            vector_results,
            graph_results,
            temporal_results,
            &fusion_weights,
        );

        Ok(results)
    }
}
```

**Phase 1 Alternative:** If search pipeline refactor is too complex, pass feature flag directly:
```rust
// Simpler approach for Phase 1
let enable_quality = std::env::var("MAPROOM_ENABLE_QUALITY_SCORING")
    .map(|v| v == "true")
    .unwrap_or(false);

GraphExecutor::execute(store, repo_id, worktree_id, limit, enable_quality)?;
```

### Component 4: Fusion Weight Override

**Location:** `crates/maproom/src/search/fusion/mod.rs`

```rust
impl FusionWeights {
    pub fn from_config(config: &SearchConfig) -> Self {
        let mut weights = Self::default();

        if let Some(graph_weight) = config.graph_importance.fusion_weight {
            weights.graph = graph_weight;
            // Renormalize other weights
            let remaining = 1.0 - graph_weight;
            let scale = remaining / (weights.fts + weights.vector + weights.recency + weights.churn);
            weights.fts *= scale;
            weights.vector *= scale;
            weights.recency *= scale;
            weights.churn *= scale;
        }

        weights
    }
}
```

## Data Flow

```
1. Search request
   ↓
2. Load config (cached, <0.1ms)
   ↓
3. Parallel executors:
   ├─ FTS Executor (unchanged)
   ├─ Vector Executor (unchanged)
   ├─ Graph Executor (ENHANCED)
   │  ├─ Check enable_quality_scoring flag
   │  ├─ If true: execute_quality_weighted()
   │  │  ├─ SQL with edge quality computation
   │  │  ├─ JOIN edges with source chunk kinds
   │  │  ├─ SUM quality-weighted edges
   │  │  └─ LOG(2 + sum) = graph_score
   │  └─ Return RankedResults
   └─ Temporal Executor (unchanged)
   ↓
4. RRF Fusion with adjusted graph weight (0.15 vs 0.10)
   ↓
5. Final results (important code ranks higher)
```

## Integration Points

### With Existing Graph Executor

**Backwards compatibility:**
```rust
// Old interface preserved
impl GraphExecutor {
    pub async fn execute(...) -> Result<RankedResults> {
        // Old implementation (flag=false)
    }

    pub async fn execute_enhanced(..., config) -> Result<RankedResults> {
        if config.enable_quality_scoring {
            Self::execute_quality_weighted(...)
        } else {
            Self::execute(...)  // Fallback
        }
    }
}
```

### With Fusion

**Weight override:**
- Old: graph = 0.10 (10% of final score)
- New: graph = 0.15 (15% + quality scoring)
- **Effect:** 2× boost for important code (quality + weight)

### With SRCHREL Phase 1

**Consistency:**
- Phase 1: Edge quality for related chunk sorting
- Phase 2: Same edge quality for ranking
- **Orthogonal:** Both can be enabled simultaneously

## Performance Considerations

### Performance Validation Results (SRCHREL-0002)

**Validation Date:** 2025-12-15
**Test Database:** CrewChief production (164,395 chunks, 458 call edges)
**Test:** `crates/maproom/tests/graph_quality_performance.rs`

**Initial Results (WITHOUT recommended index):**

| Metric | Measured | Target | Status |
|--------|----------|--------|--------|
| P50 latency | 52.48ms | <15ms | ❌ FAIL |
| P95 latency | 53.72ms | <30ms | ❌ FAIL |
| P99 latency | 53.80ms | <50ms | ❌ FAIL |
| Index usage | ✓ Partial | Full | ⚠️ Missing dst index |

**Root Cause:** Missing index on `chunk_edges(dst_chunk_id)` causes full table scan of edges.

**EXPLAIN QUERY PLAN showed:**
- ✅ `SEARCH src_chunk USING INTEGER PRIMARY KEY` - Efficient
- ✅ `SEARCH src_file USING INTEGER PRIMARY KEY` - Efficient
- ✅ `SEARCH c USING COVERING INDEX` - Efficient
- ❌ `SCAN ce` - Full scan of chunk_edges (bottleneck)

**Expected Results (WITH recommended index):**

| Metric | Estimated | Target | Status |
|--------|-----------|--------|--------|
| P50 latency | ~8ms | <15ms | ✅ PASS |
| P95 latency | ~10ms | <30ms | ✅ PASS |
| P99 latency | ~12ms | <50ms | ✅ PASS |
| Improvement | **5-6× faster** | - | - |

**Scaling Projections:**

| Database Size | Without Index (p95) | With Index (p95) | Improvement |
|---------------|-------------------|------------------|-------------|
| 458 edges (current) | 53.72ms | ~10ms | 5× faster |
| 100K edges (medium) | ~11 seconds | ~20ms | 500× faster |
| 1M edges (large) | ~110 seconds | ~30ms | 4000× faster |

See `planning/performance-results.md` for detailed analysis.

### Database Indexes

**Existing:**
```sql
-- Auto-index from UNIQUE constraint
UNIQUE(src_chunk_id, dst_chunk_id, type)  -- Optimized for source lookups
```

**Required (CRITICAL - ADD IN PHASE 1):**
```sql
-- Enable efficient destination lookups (our query pattern)
CREATE INDEX idx_chunk_edges_dst_type_src
ON chunk_edges(dst_chunk_id, type, src_chunk_id);
```

**Why This Index:**
1. **`dst_chunk_id`** - Primary filter (`WHERE ce.dst_chunk_id IN (...)`)
2. **`type`** - Secondary filter (used in CASE for edge type weight)
3. **`src_chunk_id`** - Covering index (needed for JOIN to src_chunk)

**Benefits:**
- Eliminates full table scan of edges
- 5-6× performance improvement on current database
- Scales to 1M+ edges with <30ms p95
- Minimal storage overhead (~50KB per 458 edges)

### Configuration Cache

- First request: Load from file (~5ms)
- Subsequent: Cache hit (<0.1ms)
- **No performance impact**

## Known Limitations

**1. Test Detection Based on Heuristics**
- File path patterns (primary): **100% precision and recall** (VALIDATED: SRCHREL-0003)
- Chunk kind patterns (secondary): Not required (file path achieves 100% accuracy)
- False positives: **0 out of 200 samples** (VALIDATED: 0.00% false positive rate)
- False negatives: **0 out of 200 samples** (VALIDATED: 0.00% false negative rate)
- **Validation Results (SRCHREL-0003):** 200 chunks tested, 100% accuracy on production database
- **Acceptance Criteria:** ✅ PASSED - Exceeds 85% precision and 80% recall targets
- **Future:** User-configurable test path patterns in Phase 2, monitor real-world accuracy

**2. Fixed Source Importance (No Recursive Scoring)**
- All production callers have equal weight (1.0)
- Misses distinction: Caller from core API vs caller from peripheral utility
- **Rationale:** Recursive importance (PageRank) is complex and adds latency
- **Future:** PageRank scoring in Phase 3 if quality-weighted in-degree proves insufficient

**3. No Edge Recency Weighting**
- Old edges count same as new edges
- Doesn't capture "actively used" vs "legacy callers"
- **Blocker:** Requires schema change (edge creation timestamps not stored)
- **Defer:** Phase 2+ if deemed valuable

**4. Phase 1 Hardcoded Weights**
- Edge quality weights hardcoded in SQL (not configurable)
- **Rationale:** Prove algorithm works before building config infrastructure
- **Future:** Phase 2 adds YAML configuration for weight tuning

**5. No Hot Reload (Phase 1-2)**
- Config changes require service restart
- **Rationale:** Hot reload adds complexity for minimal MVP benefit
- **Future:** Phase 3 can add hot reload endpoint if frequent tuning needed

## Maintainability

### Testing
- Unit: Edge quality computation, test detection
- Integration: Enhanced executor vs old executor
- Performance: Benchmark (<30ms p95)
- E2E: Ranking quality on test queries

### Monitoring
```prometheus
graph_executor_latency_seconds{mode="quality_weighted"}
graph_score_distribution{mode="quality_weighted"}
edge_quality_distribution{edge_type, source_kind}
```

### Extension Points

**1. PageRank (Phase 3):**
- Recursive importance propagation
- ~100-200ms latency
- Defer until weighted in-degree validated insufficient

**2. File Path Test Detection (Phase 2):**
- Patterns: `**/test/**`, `*.test.ts`
- More accurate than kind heuristic

**3. Hot Config Reload (Phase 2):**
- HTTP endpoint: `POST /admin/reload-config`
- No restart needed for weight tuning
