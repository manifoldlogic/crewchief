# Ticket: [SRCHREL-3004]: Developer Documentation and Architecture Guide

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- technical-writer
- verify-ticket
- commit-ticket

## Summary
Create comprehensive developer-facing documentation covering architecture, implementation details, decision rationale, type synchronization, and extension points.

## Background
Future developers need to understand the design decisions, implementation patterns, and extension points for relationship expansion. This documentation enables maintenance, debugging, and future enhancements without requiring deep code archaeology.

This implements Phase 3 deliverables: developer documentation and architecture decision records.

## Acceptance Criteria
- [ ] Architecture guide created at `docs/architecture/relationship-clustering.md`
- [ ] Implementation guide covers key components and data flow
- [ ] Decision rationale documented for major design choices
- [ ] Type synchronization process documented (Rust ↔ TypeScript)
- [ ] Performance optimization strategies documented
- [ ] Extension points identified for future enhancements
- [ ] Inline code documentation reviewed and complete
- [ ] Known limitations and trade-offs documented

## Technical Requirements

### Architecture Documentation
Create `docs/architecture/relationship-clustering.md`:

```markdown
# Architecture: Relationship-Aware Search

## System Overview

```
Query → Search Pipeline → Confidence Scoring → Relationship Expansion → Results
                              ↓                          ↓
                       High confidence?          Graph Traversal (depth 2)
                                                        ↓
                                                 Top 5 Related Chunks
```

## Components

### 1. RelatedChunkResult Type (`crates/maproom/src/search/results.rs`)

Lightweight metadata struct for related chunks. Serializes to JSON for daemon communication.

**Key Design Decision**: Metadata only (no file content) to minimize response size.

**Fields**:
- `chunk_id`: Reference for context tool lookup
- `relpath`, `symbol_name`, `kind`, `start_line`, `end_line`: Location metadata
- `preview`: First 100 characters (truncated with "...")
- `depth`: Graph traversal depth (1 or 2)
- `relevance`: Computed score (0.0-1.0)
- `relationship_type`: "import", "call", "extends", "implements"

**Type Sync**: Mirrored in `packages/daemon-client/src/types.ts::RelatedChunkResult`

### 2. Relationship Expansion Module (`crates/maproom/src/search/relationships.rs`)

Core logic for finding and scoring related chunks.

**Key Functions**:
- `find_top_related_chunks()`: Orchestrates traversal, scoring, selection
- `compute_edge_weight()`: Maps edge type → weight (0.5-1.1)
- `extract_parent_dir()`: Module proximity detection
- `truncate_preview()`: Content preview generation

**Algorithm**:
1. Get source chunk metadata
2. Graph traversal via `find_related_chunks(depth=2)`
3. Compute relevance: `base_decay × edge_weight × module_boost`
4. Sort by relevance descending
5. Take top 5

**Relevance Formula**:
```
relevance = (0.7^depth) × edge_weight × module_boost

Where:
- depth: 1 or 2
- edge_weight: 0.5 (test), 1.0 (default), 1.1 (inheritance)
- module_boost: 1.2 (same directory), 1.0 (different)
```

### 3. Search Pipeline Integration (`crates/maproom/src/search/pipeline.rs`)

Injects relationship expansion after confidence scoring.

**Key Design Decision**: Auto-enable confidence when `include_related=true` to simplify UX.

**Integration Point**:
```rust
// After confidence scoring
if options.include_related.unwrap_or(false) {
    const MAX_CONCURRENT_EXPANSIONS: usize = 3;
    let mut count = 0;

    for result in &mut results {
        if count >= MAX_CONCURRENT_EXPANSIONS { break; }

        if let Some(conf) = &result.confidence {
            if conf.source_count >= 2 || conf.is_exact_match {
                // Expand and increment counter
            }
        }
    }
}
```

**Error Handling**: Failures logged but don't fail search (graceful degradation).

### 4. TypeScript Type Mirroring (`packages/daemon-client/src/types.ts`)

Mirrors Rust types for JSON communication.

**Type Sync Process**:
1. Rust struct has TYPE_SYNC comment: `/// TYPE_SYNC: path/to/typescript`
2. TypeScript interface has Sync comment: `// Sync with: path/to/rust`
3. Validation tests in `types.test.ts` catch divergence

**Rust → TypeScript Mappings**:
- `i64` → `number`
- `f32` → `number`
- `String` → `string`
- `Option<String>` → `string | null`
- `Vec<T>` → `T[]`

### 5. MCP Tool Schema (`packages/maproom-mcp/src/tools/search_schema.ts`)

Exposes `include_related` parameter to users.

**Schema**:
```typescript
include_related: {
  type: 'boolean',
  description: 'Include related chunks for high-confidence results...',
  default: false,
}
```

## Design Decisions

### Decision 1: Confidence-Gated Expansion

**Problem**: Graph traversal is expensive (~8ms per result). Running for all 10 results = 80ms overhead.

**Solution**: Only expand results with `source_count >= 2` OR `is_exact_match`.

**Rationale**:
- Estimated 20-40% of results meet threshold (2-4 out of 10)
- Performance: 2-4 × 8ms = 16-32ms (within <20ms budget)
- High-confidence results are most valuable for exploration

**Trade-off**: Low-confidence results don't get relationships (acceptable).

### Decision 2: Shallow Traversal (Depth 2)

**Problem**: Context tool uses 3+ hops for deep exploration. Search needs faster traversal.

**Solution**: Hardcode `max_depth = 2`.

**Rationale**:
- 2 hops captures immediate architectural context (direct + transitive)
- Performance: 2 hops ≈ 8ms vs 3+ hops ≈ 20-50ms
- 70% decay means depth 3+ chunks have low relevance (0.343)
- Users can invoke context tool for deeper exploration

**Trade-off**: Misses distant relationships (acceptable, not search use case).

### Decision 3: Metadata-Only Response

**Problem**: Include full file content (like context tool) or just metadata?

**Solution**: Metadata only (no file content).

**Rationale**:
- Response size: 5 × 200 bytes = 1KB vs 5 × 1.5KB = 7.5KB
- Latency: No file loading required
- Bandwidth: 10 results × 5 related × 200 bytes = 10KB (acceptable)

**Trade-off**: Users must invoke context tool for full content (acceptable).

### Decision 4: Module Proximity Weighting

**Problem**: Related chunks in same directory often more relevant.

**Solution**: Apply 1.2× boost to same-directory chunks.

**Rationale**:
- Simple heuristic (compare directory paths)
- Language-agnostic (no parsing package.json, Cargo.toml)
- Effective: 80% of related code is in same module

**Trade-off**: Directory ≠ module in all languages (e.g., barrel exports). Acceptable 80% accuracy.

### Decision 5: Type-Aware Edge Weighting

**Problem**: Not all edges equally valuable (code→test less useful than code→code).

**Solution**: Configurable edge weight constants.

| Edge | Target | Weight | Constant |
|------|--------|--------|----------|
| * → test | 0.5 | EDGE_WEIGHT_TEST_PENALTY |
| extends/implements | 1.1 | EDGE_WEIGHT_INHERITANCE_BOOST |
| default | 1.0 | EDGE_WEIGHT_DEFAULT |

**Rationale**:
- Prioritizes production code relationships
- Inheritance edges stronger architectural signal
- Constants enable easy tuning

**Trade-off**: "test" detection via `kind.contains("test")` is heuristic (acceptable).

### Decision 6: MAX_CONCURRENT_EXPANSIONS Cap

**Problem**: If 10 results are high-confidence, 10 × 8ms = 80ms (exceeds budget).

**Solution**: Hard cap at 3 expansions.

**Rationale**:
- 3 × 8ms = 24ms (slightly over 20ms but acceptable)
- Users get relationships for top results (highest confidence)
- Prevents performance degradation if confidence thresholds looser than expected

**Trade-off**: 4+ high-confidence results don't all get relationships (acceptable).

## Data Flow

1. **User calls MCP search tool** with `include_related=true`
2. **TypeScript daemon client** validates params, calls daemon
3. **Rust search pipeline** executes:
   - Query processing
   - Parallel executors (FTS, vector, graph, signals)
   - Score fusion (RRF) → FusedResult[]
   - Result assembly → ChunkSearchResult[]
   - **Confidence scoring** (auto-enabled)
   - **Relationship expansion** (this feature)
4. **For each result** with high confidence:
   - Call `find_top_related_chunks(chunk_id, 5)`
   - Graph traversal (depth=2)
   - Compute relevance (decay × edge_weight × module_boost)
   - Sort by relevance, take top 5
   - Convert to RelatedChunkResult
   - Set `result.related = Some(chunks)`
5. **Rust serializes** to JSON (omits None fields via serde)
6. **TypeScript deserializes** and returns to MCP client
7. **User sees** results with `related` field

## Performance Characteristics

### Latency Budget

**Target**: <20ms p95 overhead

**Measured Costs** (depth-2 traversal):
- Confidence check: <0.1ms per result
- Graph traversal: ~8ms per result
- Relevance computation: <0.01ms per chunk
- Sorting: <0.1ms per result
- Top-N selection: <0.01ms

**Total**: 3 results × 8ms = 24ms (slightly over but acceptable)

### Optimizations

**Sequential Traversal** (current):
```rust
for result in &mut results {
    if qualifies {
        result.related = Some(find_top_related(...).await?);
    }
}
```

**Parallel Traversal** (contingency if budget exceeded):
```rust
let futures = high_conf_results.iter()
    .map(|r| find_top_related(r.chunk_id, 5));
let related_results = join_all(futures).await;
```

**Trade-off**: Parallel adds database load (3 concurrent queries).

**Decision**: Implement if Phase 1 benchmarks show sequential exceeds 20ms.

### Database Indexes

**Existing Indexes** (sufficient):
- `idx_chunk_edges_src` on `(src_chunk_id, type)`
- `idx_chunk_edges_dst` on `(dst_chunk_id, type)`

**Query Pattern** (recursive CTE):
```sql
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

## Type Synchronization

### Process

1. **Define Rust struct** with TYPE_SYNC comment
2. **Mirror in TypeScript** with Sync comment
3. **Write validation test** in `types.test.ts`
4. **CI enforces** tests pass before merge

### Example

**Rust** (`crates/maproom/src/search/results.rs`):
```rust
/// TYPE_SYNC: packages/daemon-client/src/types.ts::RelatedChunkResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedChunkResult {
    pub chunk_id: i64,
    // ...
}
```

**TypeScript** (`packages/daemon-client/src/types.ts`):
```typescript
/** Sync with: crates/maproom/src/search/results.rs::RelatedChunkResult */
export interface RelatedChunkResult {
  chunk_id: number
  // ...
}
```

**Validation Test** (`types.test.ts`):
```typescript
it('matches Rust struct fields exactly', () => {
  const sample: RelatedChunkResult = { /* all fields */ };
  expect(typeof sample.chunk_id).toBe('number');
  // ... validate all fields
});
```

## Extension Points

### 1. Configurable Depth

**Current**: Hardcoded `max_depth = 2`

**Future**: Add `max_depth` parameter to search options

**Implementation**:
- Add `max_depth?: number` to SearchParams
- Pass to `find_related_chunks(depth=max_depth)`
- Validate range (1-5 reasonable)

### 2. Relationship Type Filtering

**Current**: Returns all relationship types

**Future**: Allow `edge_types: string[]` parameter

**Implementation**:
- Add `edge_types?: string[]` to SearchParams
- Filter graph traversal by edge type
- Example: `edge_types: ['import', 'call']` (exclude extends)

### 3. Cross-Repository Relationships

**Current**: Single repository only

**Future**: Traverse edges across repo boundaries

**Implementation**:
- Extend `chunk_edges` schema with `repo_id`
- Graph traversal checks repository boundaries
- Security: Validate user has access to target repos

### 4. ML-Based Clustering

**Current**: Heuristic-based relevance (decay, edge weights, module proximity)

**Future**: Augment with embedding similarity

**Implementation**:
- Compute cosine similarity between chunk embeddings
- Blend with graph relevance: `0.7 × graph_relevance + 0.3 × embedding_similarity`
- Requires embedding storage and indexing

## Known Limitations

### 1. Cross-Result Duplication

**Issue**: Shared dependencies appear in multiple `result.related` arrays.

**Impact**: Minor response size bloat, user sees duplicates.

**Mitigation**: Monitor response size, defer deduplication to Phase 2 if needed.

### 2. Module Proximity Accuracy

**Issue**: Directory-based module detection is 80% accurate (not 100%).

**Failure Cases**: Barrel exports, monorepos, workspace members.

**Mitigation**: Acceptable trade-off, Phase 2 could add language-specific detection.

### 3. Fixed Depth

**Issue**: Users cannot customize traversal depth.

**Impact**: Power users can't explore deeper relationships.

**Mitigation**: Context tool provides deep exploration, defer to Phase 2.

## Testing Strategy

### Unit Tests
- Edge weight computation (all relationship types)
- Module proximity boost (same vs different directory)
- Relevance sorting (higher ranks first)
- Preview truncation (>100 chars)
- Empty results (no relationships)

### Integration Tests
- Search with relationships (high-confidence results)
- Confidence gating (only high-confidence expanded)
- Backward compatibility (without parameter)
- MAX_CONCURRENT_EXPANSIONS cap
- Graceful degradation (graph errors)

### Performance Tests
- Baseline search latency
- Relationship expansion overhead (<20ms p95)
- Response size (<10KB)
- Graph traversal scaling (edge count)

### E2E Tests
- MCP tool parameter passing
- JSON serialization round-trip
- Type synchronization validation
- Real database queries

## Maintenance

### Updating Edge Weights

1. Edit constants in `relationships.rs`:
   ```rust
   const EDGE_WEIGHT_TEST_PENALTY: f32 = 0.5;
   ```
2. Run tests: `cargo test relationships`
3. Benchmark impact: `cargo bench search_relationships`
4. Update documentation if behavior changes

### Adding New Relationship Types

1. Ensure indexer populates `chunk_edges` with new type
2. Add edge weight case in `compute_edge_weight()`
3. Add test case in `test_edge_weight_computation()`
4. Update documentation with new type

### Type Synchronization Maintenance

1. Modify Rust struct in `results.rs`
2. Mirror change in TypeScript `types.ts`
3. Update validation test in `types.test.ts`
4. Run tests: `cargo test && npm test`
5. CI fails if types diverge

## References

- [Planning Document](.crewchief/projects/SRCHREL_relationship-clustering/planning/plan.md)
- [Architecture Document](.crewchief/projects/SRCHREL_relationship-clustering/planning/architecture.md)
- [Quality Strategy](.crewchief/projects/SRCHREL_relationship-clustering/planning/quality-strategy.md)
- [User Documentation](../features/relationship-aware-search.md)
```

### Inline Code Documentation Review

Ensure all public functions have:
- Doc comments (Rust: `///`, TypeScript: `/** */`)
- Parameter descriptions
- Return value descriptions
- Example usage (where helpful)
- Links to related functions

Example:
```rust
/// Find top N related chunks for a given chunk via graph traversal.
///
/// Performs shallow traversal (max_depth=2) and returns up to `limit` chunks
/// sorted by relevance (decay × edge_weight × module_boost).
///
/// # Arguments
/// * `store` - Database connection
/// * `source_chunk_id` - Starting chunk for traversal
/// * `limit` - Maximum number of related chunks to return
///
/// # Returns
/// Vector of `RelatedChunkResult` sorted by relevance (descending)
///
/// # Example
/// ```
/// let related = find_top_related_chunks(&store, 123, 5).await?;
/// ```
pub async fn find_top_related_chunks(...)
```

## Implementation Notes

Documentation organization:
- Start with overview (system context)
- Component details (what each does)
- Design decisions (why choices were made)
- Data flow (how it works end-to-end)
- Extension points (how to extend)
- Known limitations (honest trade-offs)

Writing style for developers:
- Assume technical background
- Include code snippets liberally
- Link to source code
- Document rationale (not just what)
- Call out extension points

Diagrams (optional):
- System architecture diagram
- Data flow diagram
- Type synchronization process

## Dependencies
- All previous tickets (complete implementation must exist)

## Risk Assessment
- **Risk**: Documentation becomes stale as code evolves
  - **Mitigation**: Link to code with line numbers; add "last updated" dates; review quarterly

## Files/Packages Affected
- `docs/architecture/relationship-clustering.md` (new file)
- Review inline documentation in:
  - `crates/maproom/src/search/results.rs`
  - `crates/maproom/src/search/relationships.rs`
  - `crates/maproom/src/search/pipeline.rs`
  - `packages/daemon-client/src/types.ts`

## Verification Notes
The verify-ticket agent should check:
- Architecture documentation file exists and is comprehensive
- All components documented with design rationale
- Type synchronization process clearly explained
- Extension points identified with implementation guidance
- Known limitations documented honestly
- Inline code documentation reviewed (sample 3-5 functions)
- Tests pass - N/A (documentation-only ticket)
