# Analysis: Relationship-Aware Search

## Problem Definition

Search results in maproom currently return individual code chunks ranked by relevance, but **lack architectural context** about how these chunks relate to each other. Users receive a flat list of results without understanding:

1. **Related implementations** - When finding a function, what other functions call it or are called by it?
2. **Architectural patterns** - Which chunks belong to the same module or implement the same pattern?
3. **Cross-cutting concerns** - How do related chunks cluster across file boundaries?
4. **Import/call relationships** - What dependencies exist between discovered chunks?

**Core Issue**: While individual search results may be highly relevant, users lose valuable architectural context by not seeing **relationships between results** or **related chunks that didn't rank highly enough to appear in the top results**.

### Concrete Example

A developer searches for "authentication handler" and receives:
```
1. authHandler.ts (score: 8.5)
2. validateToken.ts (score: 7.2)
3. authMiddleware.ts (score: 6.8)
```

**What's Missing:**
- `authHandler.ts` imports `validateToken.ts` and calls it - this relationship isn't visible
- `authMiddleware.ts` calls `authHandler.ts` - the control flow isn't shown
- `sessionManager.ts` (score: 4.1) didn't make the top 10 but is directly imported by `authHandler.ts` - a critical architectural dependency is hidden
- All three chunks are in the same module (`src/auth/`) - the module boundary clustering isn't exposed

**Impact**: Developer must manually piece together architectural relationships by inspecting each file, losing the time-saving benefit of semantic search.

## Context

### Prerequisites (Dependencies from Initiative)

This project depends on two completed Phase 1 and Phase 2 projects:

1. **SRCHCONF (Confidence Scoring)** - COMPLETE
   - Delivered `ConfidenceSignals` with `source_count`, `score_gap`, `is_exact_match`, `relative_score`, `rank`
   - Provides confidence thresholds to determine which results deserve related chunk expansion
   - Available via `include_confidence=true` parameter
   - **Why Critical**: Only high-confidence results should trigger expensive graph traversal. Low-confidence results don't warrant relationship exploration.

2. **SRCHFLTR (Result Filtering)** - COMPLETE (archived)
   - Delivered result type filtering (code/docs/tests)
   - Reduces noise before clustering
   - **Why Critical**: Cleaner result set (without archived docs, tests) means more focused graph traversal. Type filtering enables type-aware relationship weighting (e.g., prioritize code→code edges over code→test edges).

**Status**: All dependencies complete. SRCHREL is ready to proceed as Phase 2 project.

### Existing Infrastructure

Maproom already has rich relationship data that's currently **underutilized in search**:

#### 1. chunk_edges Table (SQLite Schema)

```sql
CREATE TABLE chunk_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    src_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    dst_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    UNIQUE(src_chunk_id, dst_chunk_id, type)
);
```

**Edge Types** (from tree-sitter parsing):
- `import` - Module imports (e.g., `import { foo } from './bar'`)
- `call` - Function/method calls
- `extends` - Class inheritance
- `implements` - Interface implementation

**Coverage**: Populated during indexing for TypeScript, Rust, Python, Go, JavaScript files.

#### 2. Graph Traversal Functions (context/graph.rs)

**`find_related_chunks()`** - Already implements multi-hop graph traversal:
```rust
pub async fn find_related_chunks(
    store: &SqliteStore,
    chunk_id: i64,
    max_depth: i32,
    edge_types: Option<Vec<EdgeType>>,
) -> Result<Vec<RelatedChunk>>
```

**Features**:
- Bidirectional traversal (callers + callees + imports)
- Depth limiting (default: 3 hops, max: 10)
- Relevance decay: 0.7 per hop (1.0 → 0.7 → 0.49 → 0.343)
- Edge type filtering (calls, imports, extends, implements)
- Cycle detection (prevents infinite loops)

**Used By**: Context MCP tool (`mcp__maproom__context`) for retrieving code context around a specific chunk.

#### 3. Context Tool Pattern (MCP)

The `context` tool already demonstrates relationship-based code retrieval:
- Accepts `chunk_id` and `expand_options` (callers, callees, tests, docs)
- Returns `ContextBundle` with `items: ContextItem[]`
- Each item includes file content + metadata + relevance score
- Budget-limited to prevent token overflow

**Key Insight**: The context tool solves "given a chunk, show me related code." SRCHREL solves "given search results, show me related code for each high-confidence result."

### Current Search Result Structure

**ChunkSearchResult** (from SRCHCONF):
```rust
pub struct ChunkSearchResult {
    pub chunk_id: i64,
    pub score: f32,
    pub source_scores: HashMap<SearchSource, f32>,
    pub relpath: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceSignals>,  // NEW from SRCHCONF
}
```

**Gap**: No field for related chunks. Need to add `related: Option<Vec<RelatedChunkResult>>`.

## Existing Solutions

### Industry Patterns

1. **Sourcegraph Code Intelligence**:
   - Shows "References" and "Implementations" panels alongside search results
   - Click on a result → see callers/callees in sidebar
   - Uses language server protocol (LSP) for precise relationships
   - **Limitation**: Requires LSP server per language, complex setup

2. **GitHub Code Search**:
   - Shows file tree context in sidebar
   - "Jump to definition" and "Find references" from results
   - **Limitation**: Requires full repository clone, no semantic clustering

3. **Elasticsearch "More Like This"**:
   - Returns documents similar to a given document
   - Uses TF-IDF or vector similarity, not code relationships
   - **Limitation**: Similarity ≠ architectural relationship (imports, calls)

### Codebase Patterns

**Context Tool Expansion** (`crates/maproom/src/context/strategies/default.rs`):
```rust
// Expands a chunk with related code based on options
if options.callers {
    let callers = find_callers(chunk_id, max_depth).await?;
    items.extend(callers);
}
if options.callees {
    let callees = find_callees(chunk_id, max_depth).await?;
    items.extend(callees);
}
```

**Key Difference for SRCHREL**:
- Context tool: Single chunk → all related chunks (deep traversal, budget-limited)
- SRCHREL: Multiple search results → top N related chunks per result (shallow traversal, count-limited)

**Relevance Decay** (from `context/graph.rs`):
```rust
const RELEVANCE_DECAY: f64 = 0.7;

// Depth 0: relevance 1.0
// Depth 1: relevance 0.7
// Depth 2: relevance 0.49
// Depth 3: relevance 0.343
```

**Insight**: Existing decay model works well for context retrieval. SRCHREL can reuse this pattern but with shallower depth (1-2 hops vs 3+).

## Constraints

### Technical Constraints

1. **Performance Budget**: <20ms overhead for relationship clustering
   - Current search latency: ~40ms p95 (after SEMRANK + SRCHCONF)
   - Initiative target: <100ms p95
   - Remaining budget: ~60ms
   - SRCHREL allocation: <20ms (conservative, leaves headroom)

2. **Database Schema**: No schema changes allowed
   - Must use existing `chunk_edges` table
   - Cannot add new tables or columns
   - JSON columns for new metadata acceptable

3. **Backward Compatibility**: Optional feature (opt-in)
   - New parameter: `include_related: boolean` (default: false for MVP)
   - Existing consumers unaffected
   - Response structure remains compatible (new optional field)

4. **Type Synchronization**: Rust ↔ TypeScript
   - New types in `crates/maproom/src/search/results.rs`
   - Mirror types in `packages/daemon-client/src/types.ts`
   - Validation tests required

### Business Constraints

1. **MVP Scope**: Basic relationship clustering only
   - Defer: Machine learning-based clustering
   - Defer: User preference persistence for relationship types
   - Defer: Cross-repository relationship traversal
   - Ship: Import/call-based clustering with confidence thresholds

2. **Query Modes**: Works for all search modes (FTS, vector, hybrid)
   - Relationship traversal happens **after** score fusion
   - Same clustering logic regardless of how results were found

### Operational Constraints

1. **Graph Traversal Depth**: Limit to 1-2 hops maximum
   - Prevents unbounded queries
   - Reduces latency (2 hops ≈ 5-10ms vs 3+ hops ≈ 20-50ms)
   - Still captures immediate architectural context

2. **Related Chunk Count**: 3-5 related chunks per result maximum
   - Prevents response bloat
   - Focuses on highest-relevance relationships
   - User can request full context via context tool if needed

3. **Confidence Threshold**: Only expand high-confidence results
   - `confidence.source_count >= 2` OR `confidence.is_exact_match == true`
   - Prevents wasting graph traversal on weak results
   - Estimated: 20-40% of results qualify (top 2-4 out of 10)

## Success Criteria

### Must Achieve

1. **Relationship Discovery**: Related chunks appear for high-confidence search results
   - Given: Search with `include_related=true`
   - When: Result has `confidence.source_count >= 2` or exact match
   - Then: Result includes `related: [...]` with 3-5 chunks

2. **Performance**: <20ms overhead for relationship clustering
   - Measured: p95 latency increase from baseline search
   - Baseline: ~40ms (SEMRANK + SRCHCONF)
   - Target: <60ms with SRCHREL enabled

3. **Graph Traversal**: Import and call edges traversed correctly
   - Test: Search result A imports B, B calls C
   - Expected: A.related includes B (depth 1), may include C (depth 2)
   - Decay: B has relevance 0.7, C has relevance 0.49

4. **Backward Compatibility**: Existing consumers unaffected
   - Test: Search without `include_related` parameter
   - Expected: No `related` field in results, same latency as before

### Should Achieve

1. **Clustering by Module Boundaries**: Related chunks in same directory/module weighted higher
   - Test: Search finds `auth/handler.ts` and `auth/validator.ts`
   - Expected: Same-module chunks appear first in `related` list

2. **Type-Aware Weighting**: Code→code edges prioritized over code→test edges
   - Test: Result has edges to both implementation and test file
   - Expected: Implementation file ranks higher in `related` list (if both have same depth)

3. **Confidence-Based Expansion**: Only high-confidence results expanded
   - Test: Search returns 10 results, 3 with high confidence
   - Expected: Only those 3 results have `related` field populated

### Nice to Have

1. **Progressive Ranking**: Related chunks sorted by combined relevance (decay × intrinsic score)
   - If related chunk also appeared in main results, boost its relevance
   - Cross-reference between main results and related chunks

2. **Deduplication**: Don't include main result chunks in `related` lists
   - If chunk A and B both in main results, A.related doesn't include B
   - Prevents redundancy

## Open Questions

1. **Depth Configuration**: Should users control traversal depth?
   - Option 1: Hardcode depth=2 (simpler, predictable performance)
   - Option 2: Allow `max_depth` parameter (flexibility, complexity)
   - **Recommendation**: Hardcode for MVP, add parameter in Phase 2 if requested

2. **Edge Type Filtering**: Should users specify which relationships to traverse?
   - Option 1: Always traverse all edge types (imports + calls + extends)
   - Option 2: Allow `edge_types: ['import', 'call']` parameter
   - **Recommendation**: All edges for MVP (simpler), filter in Phase 2 if needed

3. **Module Boundary Detection**: How to identify module boundaries?
   - Option 1: Same directory = same module (simple, language-agnostic)
   - Option 2: Parse package.json / Cargo.toml / go.mod (accurate, complex)
   - **Recommendation**: Directory-based for MVP (80% accuracy, zero config)

4. **Related Count**: Fixed 5 or dynamic based on result count?
   - Option 1: Always return top 5 related chunks (predictable)
   - Option 2: Scale with result count (1 result → 10 related, 10 results → 3 related)
   - **Recommendation**: Fixed 5 for MVP (simpler, consistent UX)

5. **Cross-Result Deduplication**: If related chunk appears in multiple `related` lists?
   - Option 1: Allow duplicates (simple, each result self-contained)
   - Option 2: Deduplicate across all `related` lists (complex, prevents redundancy)
   - **Recommendation**: Allow duplicates for MVP (simpler, acceptable for 3-5 chunks)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Graph traversal exceeds 20ms budget | Medium | High | Limit depth to 2 hops, cap at 5 related chunks, measure in benchmarks |
| Related chunks not useful (low quality) | Medium | Medium | Use confidence thresholds, decay relevance by depth, type-aware weighting |
| Response size bloat (JSON payload) | Low | Medium | Cap at 5 related chunks, exclude content (only metadata), measure payload size |
| Type sync errors (Rust ↔ TypeScript) | Low | High | Validation tests, TYPE_SYNC comments, CI checks |
| Cyclic graph traversal loops | Low | Medium | Existing cycle detection in `find_related_chunks()` handles this |

## Alternatives Considered

### Alternative 1: ML-Based Clustering

**Approach**: Use embeddings to find similar chunks, not graph edges.

**Pros**:
- Finds semantic relationships even without explicit imports/calls
- Language-agnostic (works for any code)

**Cons**:
- Requires embeddings (not always available)
- "Similar" ≠ "related" (may find unrelated but similar-looking code)
- Higher latency (vector search for each result)

**Decision**: Rejected for MVP. Graph edges provide precise, structural relationships. ML clustering can be Phase 2 enhancement.

### Alternative 2: Full Context Expansion

**Approach**: Return full `ContextBundle` for each result (like context tool).

**Pros**:
- Rich context (file content, not just metadata)
- Reuses existing context assembly logic

**Cons**:
- Massive response size (10 results × 6KB each = 60KB+)
- Slow (budget calculation, file loading for each result)
- Overwhelming for users (too much information)

**Decision**: Rejected. SRCHREL provides lightweight metadata pointers. Users can invoke context tool for deep dives.

### Alternative 3: Client-Side Graph Traversal

**Approach**: Return all `chunk_edges` for result chunks, let client traverse.

**Pros**:
- Flexible (client controls depth, filtering)
- Lower server load per request

**Cons**:
- Large data transfer (all edges for all chunks)
- Duplicates graph logic in TypeScript
- Slower (network latency + client processing)

**Decision**: Rejected. Server-side traversal is faster (local database access) and more consistent (single implementation).

## Key Insights

1. **Leverage Existing Infrastructure**: `chunk_edges` table and `find_related_chunks()` function already solve graph traversal. SRCHREL adapts this for search results.

2. **Confidence Gating is Critical**: Only ~20-40% of results should trigger relationship expansion. Confidence scoring (from SRCHCONF) provides the threshold mechanism.

3. **Shallow is Better**: Deep traversal (3+ hops) explodes complexity and latency. 1-2 hops captures immediate architectural context without performance penalty.

4. **Metadata, Not Content**: Related chunks should include metadata (file, symbol, line range) but not full content. Users invoke context tool for deep inspection.

5. **Module Clustering is Valuable**: Directory-based module boundaries provide simple, effective clustering without language-specific parsing.
