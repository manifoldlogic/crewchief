# Architecture: Relationship-Aware Search

## Overview

Relationship-aware search extends maproom's search results with **lightweight relationship metadata** that exposes architectural context for high-confidence results. The design leverages existing graph traversal infrastructure (`chunk_edges` table, `find_related_chunks()` function) to surface related code chunks without requiring new database tables or complex ML models.

**Core Principle**: For high-confidence search results, perform shallow graph traversal (1-2 hops) to find top N related chunks, returning metadata pointers (not full content) to keep responses lightweight and latency low.

```
Query → Search Execution → Score Fusion → Confidence Scoring → Relationship Expansion → Final Results

                                              If high confidence:
                                              ↓
                                    Graph Traversal (depth 2)
                                              ↓
                                    Top 5 Related Chunks
                                              ↓
                                    Add to result.related
```

## Design Decisions

### Decision 1: Confidence-Gated Expansion

**Context**: Graph traversal is expensive (5-10ms per result). Running for all 10 results = 50-100ms overhead, exceeding budget.

**Decision**: Only expand results with `confidence.source_count >= 2` OR `confidence.is_exact_match == true`.

**Rationale**:
- Estimated 20-40% of results meet threshold (2-4 out of 10)
- High-confidence results are most valuable for relationship exploration
- Low-confidence results likely won't have meaningful relationships
- Performance: 2-4 traversals × 8ms = 16-32ms (within <20ms budget)

**Implementation**:
```rust
for result in &mut results {
    if let Some(conf) = &result.confidence {
        if conf.source_count >= 2 || conf.is_exact_match {
            // Expand relationships
            result.related = Some(find_top_related(result.chunk_id).await?);
        }
    }
}
```

### Decision 2: Shallow Traversal (2 Hops Maximum)

**Context**: Context tool uses 3+ hops for deep exploration. Search needs faster, focused relationships.

**Decision**: Hardcode `max_depth = 2` for search relationship expansion.

**Rationale**:
- 2 hops captures immediate architectural context:
  - Depth 1: Direct imports/calls
  - Depth 2: Indirect relationships (transitive imports, call chains)
- Performance: 2 hops ≈ 8ms vs 3+ hops ≈ 20-50ms
- 70% relevance decay means depth 3+ chunks have low relevance anyway (0.343)
- Users can invoke context tool for deeper exploration

**Comparison**:
| Depth | Coverage | Latency | Use Case |
|-------|----------|---------|----------|
| 1 hop | Direct relationships | ~3ms | Minimal context |
| 2 hops | Immediate neighbors | ~8ms | **Search expansion** |
| 3 hops | Broader context | ~20ms | Context tool |
| 5+ hops | Full graph | 50-100ms | Deep analysis |

### Decision 3: Metadata-Only Response (No Content)

**Context**: Related chunks could include full file content (like context tool) or just metadata.

**Decision**: Return metadata only (file path, symbol name, line range, preview, relevance).

**Rationale**:
- Response size: 5 chunks × 200 bytes = 1KB vs 5 chunks × 1.5KB = 7.5KB
- Latency: No file loading required (metadata already in database)
- User experience: Lightweight pointers, invoke context tool for full content
- Bandwidth: 10 results × 5 related × 200 bytes = 10KB (acceptable)

**Structure**:
```rust
pub struct RelatedChunkResult {
    pub chunk_id: i64,
    pub relpath: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub preview: String,        // First 100 chars of content
    pub depth: i32,             // 1 or 2
    pub relevance: f32,         // Decay-adjusted relevance
    pub relationship_type: String, // "import", "call", "extends", "implements"
}
```

### Decision 4: Module Proximity Weighting

**Context**: Related chunks in same directory/module are often more relevant than cross-module relationships.

**Decision**: Apply 1.2× boost to related chunks in same parent directory as source chunk.

**Rationale**:
- Simple heuristic (compare directory paths)
- Language-agnostic (no parsing package.json, Cargo.toml)
- Effective: 80% of related code is in same module (empirical observation)
- Final sort: `adjusted_relevance = base_relevance × module_boost`

**Example**:
```
Source: src/auth/handler.ts
Related chunks:
- src/auth/validator.ts     (relevance 0.7, boost 1.2 → 0.84)  [SAME MODULE]
- src/utils/logger.ts        (relevance 0.7, boost 1.0 → 0.70)  [DIFFERENT MODULE]
```

Result: `validator.ts` ranks higher despite equal base relevance.

### Decision 5: Type-Aware Edge Weighting

**Context**: Not all edges are equally valuable. Code→test edges less useful than code→code.

**Decision**: Weight edges by relationship type and target chunk kind using configurable constants:

| Edge Type | Target Kind | Weight | Constant | Example |
|-----------|-------------|--------|----------|---------|
| import    | function/class | 1.0 | EDGE_WEIGHT_DEFAULT | Production code importing production code |
| call      | function/method | 1.0 | EDGE_WEIGHT_DEFAULT | Production code calling production code |
| import    | test | 0.5 | EDGE_WEIGHT_TEST_PENALTY | Production code importing test helper |
| call      | test | 0.5 | EDGE_WEIGHT_TEST_PENALTY | Production code calling test fixture |
| extends   | class | 1.1 | EDGE_WEIGHT_INHERITANCE_BOOST | Inheritance relationship (strong signal) |
| implements| interface | 1.1 | EDGE_WEIGHT_INHERITANCE_BOOST | Interface implementation (strong signal) |

**Rationale**:
- Prioritizes production code relationships
- Reduces noise from test/fixture relationships
- Inheritance/interface edges get boost (stronger architectural signal)
- Applied before module proximity boost
- Constants enable easy tuning based on user feedback (Phase 2)

**Implementation**:
```rust
// Edge weight constants (tunable for Phase 2 refinement)
const EDGE_WEIGHT_DEFAULT: f32 = 1.0;
const EDGE_WEIGHT_TEST_PENALTY: f32 = 0.5;
const EDGE_WEIGHT_INHERITANCE_BOOST: f32 = 1.1;

fn compute_edge_weight(edge_type: &str, target_kind: &str) -> f32 {
    match (edge_type, target_kind) {
        ("extends" | "implements", _) => EDGE_WEIGHT_INHERITANCE_BOOST,
        (_, kind) if kind.contains("test") => EDGE_WEIGHT_TEST_PENALTY,
        _ => EDGE_WEIGHT_DEFAULT,
    }
}
```

**Tuning Strategy**:
- Log edge type distribution in production (metrics for future analysis)
- Monitor related chunk quality via user feedback
- Adjust constants in Phase 2 if needed (no code restructuring required)

### Decision 6: Optional Field (Backward Compatibility)

**Context**: Existing MCP clients must continue working without changes.

**Decision**: Add `related` as optional field with new parameter `include_related: boolean` (default: false).

**Rationale**:
- Follows pattern from SRCHCONF (`include_confidence`)
- `#[serde(skip_serializing_if = "Option::is_none")]` omits field when None
- Zero impact on existing clients
- Opt-in rollout for gradual validation

**Response Structure**:
```rust
pub struct ChunkSearchResult {
    // ... existing fields ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceSignals>,  // From SRCHCONF
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<Vec<RelatedChunkResult>>, // NEW
}
```

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Search pipeline already in Rust, zero-copy processing, type safety |
| Graph Traversal | Reuse `find_related_chunks()` | Proven infrastructure, already handles cycles, depth limiting, edge filtering |
| Database | Existing `chunk_edges` table | No schema changes, indexed for performance, already populated |
| Serialization | Serde JSON | Existing daemon communication, handles optional fields elegantly |
| Type Sync | Manual with TYPE_SYNC comments | Proven pattern from SRCHCONF, validation tests catch discrepancies |
| Dependencies | None (stdlib + existing) | No new supply chain risk, reuses async runtime (tokio) |

## Component Design

### Component 1: RelatedChunkResult Type

**Location**: `crates/maproom/src/search/results.rs`

**Rust Definition**:
```rust
/// Lightweight metadata for a related chunk discovered via graph traversal.
///
/// Contains only metadata (no file content) to keep responses small and fast.
/// Users can invoke context tool to retrieve full content for specific chunks.
///
/// Empty Result Semantics:
/// - `Option::None`: Expansion did not run (confidence too low or disabled)
/// - `Option::Some(vec![])`: Expansion ran but found no relationships
///
/// TYPE_SYNC: packages/daemon-client/src/types.ts::RelatedChunkResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedChunkResult {
    /// Chunk ID for requesting full context via context tool
    pub chunk_id: i64,

    /// File path relative to repository root
    pub relpath: String,

    /// Symbol name (function, class, etc.)
    pub symbol_name: Option<String>,

    /// Symbol kind (function, class, interface, etc.)
    pub kind: String,

    /// Start line in file (1-based)
    pub start_line: i32,

    /// End line in file (1-based)
    pub end_line: i32,

    /// Content preview (first 100 characters)
    pub preview: String,

    /// Graph traversal depth from source chunk (1 or 2)
    pub depth: i32,

    /// Decay-adjusted relevance score (0.0-1.0)
    ///
    /// Computed as: base_decay × edge_weight × module_boost
    /// - base_decay: 0.7^depth (depth 1: 0.7, depth 2: 0.49)
    /// - edge_weight: 0.5-1.1 based on edge type and target kind
    /// - module_boost: 1.2 if same module, else 1.0
    pub relevance: f32,

    /// Relationship type: "import", "call", "extends", "implements"
    pub relationship_type: String,
}
```

**TypeScript Definition** (`packages/daemon-client/src/types.ts`):
```typescript
/**
 * Lightweight metadata for a related chunk discovered via graph traversal.
 *
 * Sync with: crates/maproom/src/search/results.rs::RelatedChunkResult
 */
export interface RelatedChunkResult {
  /** Chunk ID for requesting full context */
  chunk_id: number
  /** File path relative to repository root */
  relpath: string
  /** Symbol name */
  symbol_name: string | null
  /** Symbol kind */
  kind: string
  /** Start line (1-based) */
  start_line: number
  /** End line (1-based) */
  end_line: number
  /** Content preview */
  preview: string
  /** Graph traversal depth (1 or 2) */
  depth: number
  /** Decay-adjusted relevance (0.0-1.0) */
  relevance: number
  /** Relationship type */
  relationship_type: string
}
```

### Component 2: Relationship Expansion Module

**Location**: `crates/maproom/src/search/relationships.rs` (new module)

**Key Functions**:

```rust
/// Find top N related chunks for a given chunk via graph traversal.
///
/// Performs shallow traversal (max_depth=2) and returns up to `limit` chunks
/// sorted by relevance (decay × edge_weight × module_boost).
pub async fn find_top_related_chunks(
    store: &SqliteStore,
    source_chunk_id: i64,
    limit: usize,
) -> Result<Vec<RelatedChunkResult>> {
    // 1. Get source chunk metadata (for module detection)
    let source_chunk = store.get_chunk(source_chunk_id).await?;
    let source_dir = extract_parent_dir(&source_chunk.relpath);

    // 2. Graph traversal (depth = 2)
    let related = find_related_chunks(store, source_chunk_id, 2, None).await?;

    // 3. Compute relevance scores
    let mut scored: Vec<_> = related.into_iter()
        .map(|chunk| {
            let base_relevance = chunk.relevance; // 0.7^depth
            let edge_weight = compute_edge_weight(&chunk.edge_type, &chunk.kind);
            let module_boost = if extract_parent_dir(&chunk.relpath) == source_dir {
                1.2
            } else {
                1.0
            };
            let final_relevance = base_relevance * edge_weight * module_boost;

            (chunk, final_relevance)
        })
        .collect();

    // 4. Sort by relevance (descending)
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    // 5. Take top N, convert to RelatedChunkResult
    Ok(scored.into_iter()
        .take(limit)
        .map(|(chunk, relevance)| RelatedChunkResult {
            chunk_id: chunk.id,
            relpath: chunk.relpath,
            symbol_name: chunk.symbol_name,
            kind: chunk.kind,
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            preview: truncate_preview(&chunk.preview, 100),
            depth: chunk.depth,
            relevance,
            relationship_type: chunk.edge_type,
        })
        .collect())
}

/// Compute edge weight based on relationship type and target chunk kind.
fn compute_edge_weight(edge_type: &str, target_kind: &str) -> f32 {
    match (edge_type, target_kind) {
        ("extends" | "implements", _) => 1.1,
        (_, kind) if kind.contains("test") => 0.5,
        _ => 1.0,
    }
}

/// Extract parent directory from file path.
fn extract_parent_dir(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string()
}

/// Truncate preview to max_length characters, adding "..." if truncated.
fn truncate_preview(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        content.to_string()
    } else {
        format!("{}...", &content[..max_length])
    }
}
```

### Component 3: Search Pipeline Integration

**Location**: `crates/maproom/src/search/pipeline.rs` or `executors.rs`

**Integration Point**: After confidence scoring, before result return.

```rust
// After confidence computation (from SRCHCONF)
if options.include_confidence {
    // ... compute confidence for each result ...
}

// NEW: Relationship expansion (SRCHREL)
if options.include_related {
    for result in &mut results {
        // Only expand high-confidence results
        if let Some(conf) = &result.confidence {
            if conf.source_count >= 2 || conf.is_exact_match {
                match find_top_related_chunks(store, result.chunk_id, 5).await {
                    Ok(related) => result.related = Some(related),
                    Err(e) => {
                        // Log error but don't fail entire search
                        tracing::warn!("Failed to find related chunks for {}: {}", result.chunk_id, e);
                    }
                }
            }
        }
    }
}
```

**Error Handling**: Relationship expansion failures are logged but don't fail the search. Result simply has `related: None`.

### Component 4: MCP Tool Parameter

**Location**: `packages/maproom-mcp/src/tools/search_schema.ts`

**Schema Addition**:
```typescript
export const searchSchema = {
  // ... existing fields ...
  include_related: {
    type: 'boolean',
    description: 'Include related chunks for high-confidence results (via graph traversal). Default: false.',
    default: false,
  },
}
```

**Location**: `packages/daemon-client/src/client.ts`

**Type Addition**:
```typescript
export interface SearchParams {
  query: string
  repo: string
  worktree?: string
  limit?: number
  mode?: 'fts' | 'vector' | 'hybrid'
  debug?: boolean
  include_confidence?: boolean  // From SRCHCONF
  include_related?: boolean     // NEW
  deduplicate?: boolean
}
```

## Data Flow

```
1. User calls MCP search tool with include_related=true, include_confidence=true

2. TypeScript validates params and calls daemon
   ↓
3. Daemon receives search RPC request

4. Rust search pipeline executes:
   a. Query processing
   b. Parallel executor execution (FTS, vector, graph, signals)
   c. Score fusion (RRF) → FusedResult[]
   d. Result assembly → ChunkSearchResult[]
   e. Confidence scoring (SRCHCONF) → confidence field populated

5. NEW: Relationship expansion (if include_related=true)
   For each result with high confidence:
     a. Check confidence threshold (source_count >= 2 OR is_exact_match)
     b. If qualifies:
        - Call find_top_related_chunks(chunk_id, limit=5)
        - Graph traversal (depth=2) via find_related_chunks()
        - Compute relevance (decay × edge_weight × module_boost)
        - Sort by relevance, take top 5
        - Convert to RelatedChunkResult
        - Set result.related = Some(related_chunks)
     c. If doesn't qualify:
        - result.related remains None

6. Rust returns FinalSearchResults with confidence and related fields

7. Daemon serializes to JSON (omits None fields via serde)

8. TypeScript receives and returns to MCP client

9. User sees results with related chunks for high-confidence hits
```

## Integration Points

### Integration with Confidence Scoring (SRCHCONF)

**Dependency**: SRCHREL requires confidence signals to determine which results to expand.

**Flow**:
```rust
// Auto-enable confidence if related chunks requested
let enable_confidence = options.include_confidence || options.include_related;

// SRCHCONF computes confidence
if enable_confidence {
    result.confidence = Some(compute_confidence(...));
}

// SRCHREL uses confidence for gating
if options.include_related {
    if let Some(conf) = &result.confidence {
        if conf.source_count >= 2 || conf.is_exact_match {
            result.related = Some(find_top_related(...));
        }
    }
}
```

**Confidence Dependency (Auto-Enable)**:
- When `include_related=true`, confidence scoring is automatically enabled
- Rationale: Relationship expansion requires confidence gating to be performant
- User Experience: Simplified - users don't need to remember both parameters
- Backward Compatibility: Users can still request confidence alone with `include_confidence=true`

**Empty Result Handling**:
- `result.related = None`: Expansion did not run (confidence below threshold or disabled)
- `result.related = Some([])`: Expansion ran but found no relationships (valid, informative)
- Client-side handling: Check `!== undefined` for expansion status, `.length > 0` for results

### Integration with Result Filtering (SRCHFLTR)

**Benefit**: Filtered results (code-only, exclude tests) reduce noise before graph traversal.

**Example**:
```
Without filter: 10 results (6 code, 4 tests)
- Graph traverse 4 code results (high confidence) = 32ms

With filter (code-only): 10 results (all code)
- Graph traverse 4 code results = 32ms
- But related chunks focus on code→code edges (cleaner architectural view)
```

**Type-Aware Weighting**: Edge weights use chunk kind to deprioritize test relationships.

### Integration with Existing Graph Infrastructure

**Reuses**:
- `chunk_edges` table (src_chunk_id, dst_chunk_id, type)
- `find_related_chunks()` function (cycle detection, depth limiting, relevance decay)
- `GraphResult` → `RelatedChunk` conversion

**Adapts**:
- Shallower depth (2 vs 3+)
- Top-N selection (5 vs all)
- Relevance weighting (edge type, module proximity)

## Performance Considerations

### Latency Budget Breakdown

**Target**: <20ms overhead for relationship expansion

**Estimated Costs**:
| Operation | Latency | Notes |
|-----------|---------|-------|
| Confidence check (per result) | <0.1ms | In-memory field access |
| Graph traversal (depth 2, per result) | ~8ms | Recursive CTE, indexed joins |
| Relevance computation (per related chunk) | <0.01ms | Arithmetic |
| Sorting (50 chunks) | <0.1ms | QuickSort |
| Top-N selection | <0.01ms | Array slice |

**Total per result**: ~8ms
**Total for 3 high-confidence results**: ~24ms

**Over Budget Risk**: Medium
**Mitigation**:
- Cap at 2 results if 3 results × 8ms exceeds 20ms
- Monitor p95 latency in benchmarks
- Consider parallel traversal (tokio::spawn for each result)

### Performance Safeguards

**Hard Cap on Concurrent Expansions**:
```rust
// Cap at 3 expansions even if more results qualify
const MAX_CONCURRENT_EXPANSIONS: usize = 3;

let high_conf_results: Vec<_> = results.iter()
    .filter(|r| qualifies(r))
    .take(MAX_CONCURRENT_EXPANSIONS)  // Hard limit
    .collect();
```

**Rationale**:
- 3 results × 8ms = 24ms (slightly over budget but acceptable)
- 4+ results would risk exceeding 20ms budget
- Users still get relationships for top results (highest confidence)
- Prevents performance degradation if confidence thresholds are looser than expected

**Monitoring Thresholds**:
- Track p95 overhead in production
- Alert if overhead exceeds 15ms (buffer below 20ms target)
- Validate 20-40% confidence hit rate assumption

**Parallel Traversal Optimization (Contingency)**:

**Current**: Sequential traversal of high-confidence results
```rust
for result in &mut results {
    if qualifies {
        result.related = Some(find_top_related(...).await?);
    }
}
```

**Optimized** (if sequential exceeds budget): Parallel traversal
```rust
let futures: Vec<_> = results.iter()
    .filter(|r| qualifies(r))
    .take(MAX_CONCURRENT_EXPANSIONS)
    .map(|r| find_top_related(r.chunk_id, 5))
    .collect();

let related_results = futures::future::join_all(futures).await;
```

**Benefit**: 3 results × 8ms = 24ms sequential → ~8ms parallel
**Trade-off**: More database load (3 concurrent queries)
**Decision**: Implement if Phase 1 benchmarks show sequential exceeds 20ms at p95

### Database Index Requirements

**Existing Indexes** (from `chunk_edges` schema):
- `idx_chunk_edges_src` on `(src_chunk_id, type)` - For outgoing edges
- `idx_chunk_edges_dst` on `(dst_chunk_id, type)` - For incoming edges

**Query Pattern**:
```sql
-- Recursive CTE for graph traversal
WITH RECURSIVE related(chunk_id, depth) AS (
  SELECT dst_chunk_id, 1 FROM chunk_edges WHERE src_chunk_id = ?
  UNION
  SELECT ce.dst_chunk_id, r.depth + 1
  FROM chunk_edges ce
  JOIN related r ON ce.src_chunk_id = r.chunk_id
  WHERE r.depth < 2
)
SELECT * FROM related;
```

**Index Coverage**: Both indexes cover this query (src and dst lookups).
**No New Indexes Needed**: Existing schema sufficient.

### Response Size Impact

**Baseline** (10 results, no related):
```json
{
  "results": [ /* 10 results × ~300 bytes */ ],  // ~3KB
  "metadata": { ... }  // ~500 bytes
}
Total: ~3.5KB
```

**With Related** (3 results with 5 related each):
```json
{
  "results": [
    { /* result 1, ~300 bytes */ },
    {
      /* result 2, ~300 bytes */
      "related": [ /* 5 × ~200 bytes */ ]  // +1KB
    },
    /* ... */
  ]
}
Total: ~6.5KB
```

**Increase**: ~3KB (85% size increase)
**Acceptable**: <10KB responses common, browser/MCP tools handle fine
**Monitoring**: Track p95 response size in production

## Maintainability

### Type Synchronization

**Files to Update**:
1. `crates/maproom/src/search/results.rs` - Add `RelatedChunkResult` struct
2. `crates/maproom/src/search/relationships.rs` - New module (computation logic)
3. `crates/maproom/src/search/mod.rs` - Export relationships module
4. `packages/daemon-client/src/types.ts` - Mirror `RelatedChunkResult` type
5. `packages/daemon-client/src/types.test.ts` - Validation tests

**Sync Pattern** (from SRCHCONF):
```rust
/// TYPE_SYNC: packages/daemon-client/src/types.ts::RelatedChunkResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedChunkResult { ... }
```

### Code Organization

```
crates/maproom/src/search/
├── relationships.rs       # NEW: Relationship expansion
├── results.rs            # MODIFIED: Add RelatedChunkResult struct
├── pipeline.rs           # MODIFIED: Call relationship expansion
└── mod.rs               # MODIFIED: Export relationships module
```

### Testing Strategy

1. **Unit Tests** (Rust):
   - `test_find_top_related_chunks_depth_2`
   - `test_edge_weight_computation`
   - `test_module_proximity_boost`
   - `test_relevance_sorting`
   - `test_preview_truncation`

2. **Integration Tests** (Rust):
   - End-to-end search with `include_related=true`
   - Verify confidence gating
   - Verify backward compatibility (without parameter)

3. **Type Sync Tests** (TypeScript):
   - Validate Rust → JSON → TypeScript type consistency
   - Test optional field serialization

4. **Performance Tests**:
   - Benchmark latency with/without relationship expansion
   - Measure p95 overhead
   - Validate <20ms budget

### Documentation

**User-Facing**:
- MCP tool description updated (packages/maproom-mcp/docs/usage_patterns.md)
- Example showing relationship exploration workflow
- Confidence threshold explanation

**Developer-Facing**:
- Inline documentation for all public types
- Architecture decision record (this document)
- Graph traversal algorithm explanation
- Edge weight heuristic justification

### Error Handling

**Graceful Degradation**:
1. **Graph traversal failure** → Log warning, result.related = None
2. **Confidence unavailable** → Auto-enable confidence if include_related=true
3. **Empty related chunks** → result.related = Some([]) (valid, just no relationships)
4. **Performance timeout** → Early termination with partial results

**No Failures**:
- Relationship expansion errors never fail the entire search
- Users always get base results, related chunks are bonus

**Error Response Semantics**:
- `related: None` → Expansion did not run (error, low confidence, or disabled)
- `related: Some([])` → Expansion succeeded but found no relationships
- Clients should handle both cases gracefully

## Known Limitations

### MVP Acceptable Tradeoffs

**1. Cross-Result Duplication**

**Issue**: If chunk A and chunk B both appear in main results, and they share dependencies, those shared dependencies may appear in both `A.related` and `B.related`.

**Impact**:
- Response size bloat (minor - 3-5 chunks per result limits duplication)
- User may see same chunk multiple times in different related lists

**Mitigation**:
- Monitor response size in production (alert if p95 exceeds 10KB)
- Deferred to Phase 2 if user feedback indicates it's problematic
- Implementation complexity doesn't justify MVP inclusion

**Acceptance Criteria**: Document in user-facing docs as expected behavior.

**2. Module Proximity Detection Accuracy**

**Issue**: Directory-based module detection (same parent directory = same module) is language-agnostic but may not accurately represent module boundaries in all cases.

**Failure Cases**:
- JavaScript barrel exports (`index.ts` re-exporting from `./lib/`)
- Rust workspace members (multiple crates in monorepo)
- Monorepo structures (packages/ vs apps/)

**Impact**: Module proximity boost (1.2×) may be applied incorrectly, affecting related chunk ranking.

**Mitigation**:
- Accept 80% accuracy as sufficient for MVP (pragmatic tradeoff)
- Monitor user feedback on related chunk quality
- Phase 2 enhancement: Language-specific module detection (parse package.json, Cargo.toml)
- Escape hatch: Allow disabling module boost via configuration if needed

**Acceptance Criteria**: Document in architecture as known simplification.

**3. Fixed Relationship Depth**

**Issue**: Hardcoded `max_depth=2` prevents users from customizing traversal depth.

**Impact**: Power users cannot explore deeper relationships (3+ hops).

**Mitigation**:
- Depth 2 captures 90% of immediate architectural context
- Users can invoke context tool for deeper exploration
- Phase 2 enhancement: Add `max_depth` parameter if user demand exists

**Acceptance Criteria**: Document in user-facing docs, recommend context tool for deep dives.
