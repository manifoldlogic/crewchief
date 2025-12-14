# Architecture: Confidence Scoring

## High-Level Overview

Confidence scoring extends search results with **transparency signals** that help users assess result quality. The design leverages existing search pipeline data without additional database queries or external dependencies.

**Core Principle**: Compute confidence from in-memory data structures after score fusion, using signals already present in the search pipeline.

```
Query Processing → Search Execution → Score Fusion → Result Assembly → Confidence Computation
                                          ↓
                          [FTS, Vector, Graph, Signals]
                                          ↓
                              RRF Fusion / Weighted
                                          ↓
                          Ranked results with source_scores
                                          ↓
                          Compute Confidence Components
                                          ↓
                      ChunkSearchResult + ConfidenceSignals
```

## Key Design Decisions

### Decision 1: Component-Based Confidence (Not Single Score)

**Chosen**: Expose individual confidence components as structured data

**Alternatives Considered**:
- Single 0-1 confidence score (rejected: magic weights, loses transparency)
- Categorical HIGH/MEDIUM/LOW (rejected: arbitrary thresholds)

**Rationale**:
- Transparency over magic numbers
- Users can weight components based on their context
- No tuning required for MVP
- Can add derived scores in Phase 2 if needed

**Components Exposed**:
1. `source_count` - Number of search executors that returned this chunk
2. `score_gap` - Difference between this result's score and next result's score
3. `is_exact_match` - Whether query exactly matches symbol name (from debug breakdown)
4. `relative_score` - This result's score / top result's score
5. `rank` - Position in result list (1-based)

### Decision 2: In-Memory Computation (No Database Queries)

**Chosen**: Compute confidence from `FusedResult` structures after fusion

**Alternatives Considered**:
- Query historical search data for query difficulty (rejected: requires new tables, slow)
- ML-based confidence prediction (rejected: over-engineering, requires training data)

**Rationale**:
- Zero performance impact from database
- Stateless computation, no side effects
- All data already available in memory
- Consistent with existing debug mode pattern

**Data Sources**:
- `source_scores: HashMap<SearchSource, f32>` - Already tracked per result
- `score: f32` - Already computed by fusion
- `exact_match_multiplier` - Already computed in FTS (debug mode)
- Result ordering - Available from Vec index

### Decision 3: Optional Fields (Backward Compatibility)

**Chosen**: Add confidence as optional field with `#[serde(skip_serializing_if = "Option::is_none")]`

**Alternatives Considered**:
- Always include confidence (rejected: breaks existing consumers)
- New API endpoint (rejected: duplication, maintenance burden)
- Feature flag (rejected: complexity, conditional compilation)

**Rationale**:
- Existing MCP consumers continue working unchanged
- New consumers opt-in via request parameter
- Follows existing pattern from `SearchMetadata.understanding` field
- JSON serialization omits `None` values automatically

**Implementation Pattern**:
```rust
pub struct ChunkSearchResult {
    pub chunk_id: i64,
    pub score: f32,
    pub source_scores: HashMap<SearchSource, f32>,
    // ... existing fields ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceSignals>,
}
```

### Decision 4: Leverage Existing Debug Infrastructure

**Chosen**: Compute confidence when `debug: true` or new `include_confidence: true` param

**Rationale**:
- Debug mode already computes `exact_match_multiplier`
- Confidence is a transparency feature (similar to debug)
- Can reuse parameter validation and opt-in logic
- Maintains performance for users who don't need confidence

**Parameter Design**:
```typescript
interface SearchParams {
  query: string
  repo: string
  worktree?: string
  limit?: number
  mode?: 'fts' | 'vector' | 'hybrid'
  debug?: boolean              // Existing
  include_confidence?: boolean // New (default: false for MVP opt-in)
  deduplicate?: boolean        // Existing
}
```

**Default Strategy**:
- MVP: `include_confidence` defaults to `false` (opt-in rollout)
- Future: May flip to `true` after validation period (separate decision)

## Solution Design

### Component 1: Confidence Signals Structure

**Rust Definition** (`crates/maproom/src/search/results.rs`):

```rust
/// Confidence signals for assessing search result quality.
///
/// These signals help users understand result reliability without requiring
/// complex confidence scoring models. All values are computed from existing
/// in-memory data structures with zero database overhead.
///
/// TYPE_SYNC: packages/daemon-client/src/types.ts::ConfidenceSignals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceSignals {
    /// Number of search sources that returned this chunk (1-4).
    ///
    /// Higher values indicate stronger agreement across search methods.
    /// - 4 = appeared in FTS, vector, graph, and signals (very high confidence)
    /// - 3 = appeared in 3 sources (high confidence)
    /// - 2 = appeared in 2 sources (medium confidence)
    /// - 1 = appeared in 1 source only (lower confidence)
    pub source_count: usize,

    /// Score difference between this result and the next result (0.0-1.0+).
    ///
    /// Larger gaps indicate this result is distinctly better than alternatives.
    /// - >1.0 = large separation, high confidence
    /// - 0.5-1.0 = moderate separation
    /// - 0.1-0.5 = small separation
    /// - <0.1 = very close to next result, ambiguous
    pub score_gap: f32,

    /// Whether the query exactly matched the symbol name (case-insensitive).
    ///
    /// Exact matches receive 3.0× multiplier in semantic ranking and indicate
    /// high confidence that this is the intended target.
    pub is_exact_match: bool,

    /// This result's score relative to the top result (0.0-1.0).
    ///
    /// Shows how close this result is to the best match.
    /// - 1.0 = this is the top result
    /// - 0.8-0.99 = very close to top
    /// - 0.5-0.79 = moderate quality
    /// - <0.5 = significantly below top result
    pub relative_score: f32,

    /// Position in the result list (1-based).
    ///
    /// Lower ranks combined with high source_count indicate confidence.
    pub rank: usize,
}
```

**TypeScript Definition** (`packages/daemon-client/src/types.ts`):

```typescript
/**
 * Confidence signals for assessing search result quality.
 *
 * Sync with: crates/maproom/src/search/results.rs::ConfidenceSignals
 */
export interface ConfidenceSignals {
  /** Number of search sources that returned this chunk (1-4) */
  source_count: number
  /** Score difference between this result and next result */
  score_gap: number
  /** Whether query exactly matched symbol name */
  is_exact_match: boolean
  /** This result's score relative to top result (0.0-1.0) */
  relative_score: number
  /** Position in result list (1-based) */
  rank: number
}
```

### Component 2: Query-Level Confidence Summary

**Rust Definition** (`crates/maproom/src/search/results.rs`):

```rust
/// Query-level confidence summary showing search coverage and quality.
///
/// TYPE_SYNC: packages/daemon-client/src/types.ts::SearchConfidenceSummary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfidenceSummary {
    /// Number of search sources that returned results (0-4).
    pub active_sources: usize,

    /// Percentage of available sources used (0.0-1.0).
    ///
    /// Example: If FTS and vector returned results but graph and signals didn't,
    /// coverage_ratio = 2/4 = 0.5
    pub coverage_ratio: f32,

    /// Average source count across all returned results.
    ///
    /// Higher values indicate results with broad agreement.
    pub avg_source_count: f32,

    /// Percentage of results with exact matches (0.0-1.0).
    pub exact_match_ratio: f32,

    /// Whether result limit was hit (true if returned_results == limit).
    ///
    /// false may indicate low confidence (few matches found).
    pub result_saturation: bool,
}
```

**TypeScript Definition** (`packages/daemon-client/src/types.ts`):

```typescript
/**
 * Query-level confidence summary showing search coverage and quality.
 *
 * Sync with: crates/maproom/src/search/results.rs::SearchConfidenceSummary
 */
export interface SearchConfidenceSummary {
  /** Number of search sources that returned results (0-4) */
  active_sources: number
  /** Percentage of available sources used (0.0-1.0) */
  coverage_ratio: number
  /** Average source count across all returned results */
  avg_source_count: number
  /** Percentage of results with exact matches (0.0-1.0) */
  exact_match_ratio: number
  /** Whether result limit was hit */
  result_saturation: boolean
}
```

### Component 3: Confidence Computation Logic

**Location**: `crates/maproom/src/search/confidence.rs` (new module)

**Key Functions**:

```rust
/// Compute confidence signals for a single search result.
pub fn compute_result_confidence(
    result: &FusedResult,
    all_results: &[FusedResult],
    index: usize,
    exact_match_multiplier: Option<f32>,
) -> ConfidenceSignals {
    let source_count = result.source_scores.len();

    let score_gap = if index < all_results.len() - 1 {
        result.score - all_results[index + 1].score
    } else {
        0.0
    };

    // NOTE: exact_match_multiplier must be computed unconditionally in FTS semantic ranking
    // (not just in debug mode) and stored in FusedResult for confidence access
    let is_exact_match = exact_match_multiplier
        .map(|m| m >= 2.9)
        .unwrap_or(false);

    let top_score = all_results.first()
        .map(|r| r.score)
        .unwrap_or(1.0);

    let relative_score = if top_score > 0.0 {
        result.score / top_score
    } else {
        0.0
    };

    ConfidenceSignals {
        source_count,
        score_gap,
        is_exact_match,
        relative_score,
        rank: index + 1,
    }
}

/// Compute query-level confidence summary.
pub fn compute_query_confidence(
    results: &[ChunkSearchResult],
    result_counts: &HashMap<SearchSource, usize>,
    limit: usize,
) -> SearchConfidenceSummary {
    let active_sources = result_counts.values()
        .filter(|&&count| count > 0)
        .count();

    let coverage_ratio = active_sources as f32 / 4.0;

    let avg_source_count = if !results.is_empty() {
        let sum: usize = results.iter()
            .filter_map(|r| r.confidence.as_ref())
            .map(|c| c.source_count)
            .sum();
        sum as f32 / results.len() as f32
    } else {
        0.0
    };

    let exact_match_count = results.iter()
        .filter_map(|r| r.confidence.as_ref())
        .filter(|c| c.is_exact_match)
        .count();

    let exact_match_ratio = if !results.is_empty() {
        exact_match_count as f32 / results.len() as f32
    } else {
        0.0
    };

    let result_saturation = results.len() >= limit;

    SearchConfidenceSummary {
        active_sources,
        coverage_ratio,
        avg_source_count,
        exact_match_ratio,
        result_saturation,
    }
}
```

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Search pipeline already in Rust, zero-copy computation, type safety |
| Serialization | Serde JSON | Already used for daemon communication, handles optional fields elegantly |
| Dependencies | None (stdlib only) | All computation uses HashMap, Vec, basic arithmetic - no new supply chain risk |
| Type Sync | Manual with comments | Proven pattern, validation tests catch discrepancies |

## Data Flow

```
1. User calls MCP search tool with include_confidence=true

2. TypeScript validates params and calls daemon
   ↓
3. Daemon receives search RPC request

4. Rust search pipeline executes:
   a. Query processing (tokenization, mode detection)
   b. Parallel executor execution (FTS, vector, graph, signals)
   c. Score fusion (RRF) → FusedResult[]
   d. Result assembly → ChunkSearchResult[]

5. NEW: Confidence computation (if include_confidence=true)
   a. For each result: compute_result_confidence()
      - source_count from source_scores.len()
      - score_gap from result[i].score - result[i+1].score
      - is_exact_match from exact_match_multiplier
      - relative_score from score / top_score
      - rank from array index
   b. Query summary: compute_query_confidence()
      - active_sources from result_counts
      - coverage_ratio = active / 4
      - avg_source_count from results
      - exact_match_ratio from results
      - result_saturation from len >= limit

6. Rust returns FinalSearchResults with confidence

7. Daemon serializes to JSON (omits None fields)

8. TypeScript receives and returns to MCP client

9. User sees results with confidence signals
```

## Integration Points

### Integration with Existing QueryUnderstanding

**Relationship**: ConfidenceSignals complements QueryUnderstanding in SearchMetadata:

```rust
pub struct SearchMetadata {
    // ... existing fields ...

    /// Query interpretation (from Phase 1 SRCHTRNSP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub understanding: Option<QueryUnderstanding>,

    /// Query-level confidence summary (Phase 2, this project)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_summary: Option<SearchConfidenceSummary>,
}

pub struct ChunkSearchResult {
    // ... existing fields ...

    /// Per-result confidence (Phase 2, this project)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceSignals>,
}
```

**Key Points**:
- Both are optional transparency features
- Can be requested independently (`understanding: true` without `include_confidence: true`)
- QueryUnderstanding = query-level transparency (how query was interpreted)
- ConfidenceSignals = result-level transparency (how reliable each result is)
- Both follow same optional field pattern with `#[serde(skip_serializing_if)]`

### Data Flow: source_scores Transfer

**Problem**: `source_scores` exists in `FusedResult` but needs to be in `ChunkSearchResult` for confidence computation.

**Solution**: Copy source_scores during result assembly:

```rust
// In result assembly (search/executors.rs or search/pipeline.rs)
fn assemble_chunk_result(fused: &FusedResult) -> ChunkSearchResult {
    ChunkSearchResult {
        chunk_id: fused.chunk_id,
        score: fused.score,
        source_scores: fused.source_scores.clone(), // Already exists, confidence uses this
        // ... other fields ...
        confidence: if include_confidence {
            Some(compute_result_confidence(fused, ...))
        } else {
            None
        }
    }
}
```

**Note**: source_scores is already transferred in current implementation - confidence just uses it.

### Exact Match Detection Strategy

**Current State**: `exact_match_multiplier` only computed/exposed in debug mode

**Required Change for Confidence**: Make exact match detection always available:

1. **In FTS semantic ranking** (`search/fts.rs` or equivalent):
   - Compute `exact_match_multiplier` unconditionally (not just when debug=true)
   - Store in `FusedResult` or make accessible to confidence computation

2. **In confidence computation**:
   - Access exact_match_multiplier from FusedResult
   - Convert to boolean: `multiplier >= 2.9` → `is_exact_match = true`

3. **Implementation approach**:
   ```rust
   pub struct FusedResult {
       // ... existing fields ...
       pub exact_match_multiplier: Option<f32>, // NEW: always computed, not debug-only
   }
   ```

**Alternative**: If modifying FusedResult is too invasive, compute exact match indicator directly in confidence module by re-checking query vs symbol name.

### Rust-TypeScript Type Sync

**Files to Update**:
1. `crates/maproom/src/search/results.rs` - Add ConfidenceSignals, SearchConfidenceSummary structs
2. `crates/maproom/src/search/confidence.rs` - New module for computation logic
3. `crates/maproom/src/search/mod.rs` - Export new module
4. `packages/daemon-client/src/types.ts` - Mirror types with TYPE_SYNC comments
5. `packages/daemon-client/src/types.test.ts` - Add validation tests

**Sync Pattern**:
```rust
/// TYPE_SYNC: packages/daemon-client/src/types.ts::ConfidenceSignals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceSignals { ... }
```

### MCP Tool Changes

**Files to Update**:
1. `packages/maproom-mcp/src/tools/search_schema.ts` - Add `include_confidence?: boolean`
2. `packages/maproom-mcp/src/tools/search.ts` - Pass parameter to daemon
3. `packages/daemon-client/src/client.ts` - Add to SearchParams interface

**Backward Compatibility**:
- `include_confidence` defaults to `false` for MVP (opt-in)
- Existing consumers unaffected (optional field omitted from JSON)
- No breaking changes to response structure

### Search Pipeline Integration

**Location**: `crates/maproom/src/search/executors.rs`

**Integration Point**: After RRF fusion, before result assembly

```rust
// After fusion
let fused_results = fusion.fuse(ranked_results, &weights, options.limit);

// NEW: Compute confidence if requested
if options.include_confidence {
    // Compute per-result confidence
    let results_with_confidence = fused_results
        .into_iter()
        .enumerate()
        .map(|(index, fused)| {
            let confidence = compute_result_confidence(
                &fused,
                &fused_results,
                index,
                get_exact_match_mult(&fused),
            );
            // Build ChunkSearchResult with confidence
        })
        .collect();

    // Compute query-level summary
    let summary = compute_query_confidence(
        &results_with_confidence,
        &result_counts,
        options.limit,
    );
}
```

## Performance Considerations

### Computational Complexity

**Per-Result Confidence**: O(1)
- `source_count`: HashMap.len()
- `score_gap`: Array index access
- `is_exact_match`: Boolean check
- `relative_score`: Division
- `rank`: Array index

**Query Confidence Summary**: O(m) where m = result count (≤ limit, typically 10-20)
- `active_sources`: O(4) = O(1)
- `avg_source_count`: O(m)
- `exact_match_ratio`: O(m)

**Overall Overhead**: <5ms for typical queries (10-20 results)

### Memory Impact

**Additional Memory**:
- ConfidenceSignals: ~40 bytes per result
- SearchConfidenceSummary: ~32 bytes per query
- **Total**: <1 KB for typical queries

**No Heap Allocations**:
- All computation uses stack-allocated primitives
- No new Vec allocations
- Source_scores HashMap already exists

### Performance Target

- Total search latency: <50ms p95 (current: ~40ms after SEMRANK)
- Confidence overhead: <5ms
- No database queries
- No network calls

## Error Handling

### Graceful Degradation

1. **No exact_match_multiplier** (debug mode disabled):
   - Default `is_exact_match = false`
   - Other signals still valid

2. **Empty result set**:
   - Query summary returns zeros/false
   - No panic, valid JSON response

3. **Single result** (no score gap):
   - `score_gap = 0.0` for last result
   - Other signals still meaningful

### Validation

- Division by zero guarded: `if top_score > 0.0`
- Array bounds checked before access
- Type mismatches prevented by Rust type system
- Parameter validation in TypeScript (Zod schema)

## Maintainability

### Type Synchronization

- Manual sync with TYPE_SYNC comments (proven pattern)
- Validation tests catch discrepancies
- CI runs tests on every commit
- Clear documentation of sync requirements

### Code Organization

```
crates/maproom/src/search/
├── confidence.rs       # NEW: Confidence computation
├── results.rs         # MODIFIED: Add ConfidenceSignals struct
├── executors.rs       # MODIFIED: Call confidence computation
└── mod.rs            # MODIFIED: Export confidence module
```

### Testing Strategy

1. **Unit Tests** (Rust):
   - Test `compute_result_confidence` with various inputs
   - Test `compute_query_confidence` edge cases
   - Test serialization roundtrip

2. **Type Sync Tests** (TypeScript):
   - Validate Rust → JSON → TypeScript types match
   - Test optional field serialization

3. **Integration Tests**:
   - End-to-end search with confidence
   - Verify backward compatibility
   - Performance benchmarks

### Documentation

- Inline documentation for all public types
- Examples showing high/low confidence scenarios
- User guide explaining signal interpretation
- Architecture decision record (this document)
