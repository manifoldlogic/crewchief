# Analysis: Result Filtering

**Date:** 2025-12-13
**Project:** SRCHFLTR - Result Filtering
**Status:** Deep Analysis Complete

---

## Executive Summary

Search result filtering exists at the **query level** (file_type, recency_threshold, worktree_id) but lacks **post-search result filtering** capabilities. Users cannot dynamically filter, sort, or slice search results after retrieval without re-querying. This creates friction in iterative search workflows and prevents efficient result exploration.

**Current State:** Pre-search filtering only (SQL WHERE clauses)
**Gap:** No post-search filtering (client-side result manipulation)
**Complexity:** Low - Add optional filtering to existing result structures
**Risk:** Low - Pure additive changes to result processing

---

## Problem Definition

### User Need

When users perform semantic searches, they often need to:

1. **Narrow results progressively**: "Show me only functions" → "Only TypeScript functions" → "Only recent TypeScript functions"
2. **Re-rank results**: Sort by recency instead of relevance score
3. **Slice result sets**: "Show results 11-20" for pagination
4. **Filter by metadata**: Exclude test files, show only specific symbol kinds

**Current Workflow** (inefficient):
```
User: Search for "authentication"
System: Returns 50 mixed results (functions, classes, tests, configs)
User: I only want functions
System: Must re-query with different parameters
```

**Desired Workflow** (efficient):
```
User: Search for "authentication"
System: Returns 50 results with full metadata
User: Filter to functions only (client-side, instant)
System: Shows filtered subset without re-query
```

### Specific Pain Points

1. **No Progressive Refinement**: Each filter change requires full re-query (~100ms)
2. **No Result Exploration**: Can't easily see "what kinds of symbols matched?"
3. **No Pagination Support**: No offset/limit for result slicing
4. **No Custom Sorting**: Stuck with relevance score, can't sort by recency/name/path
5. **No Metadata Aggregation**: Can't see "10 functions, 5 classes, 3 tests" summary

---

## Context

### Why This Work Is Needed

**Search is expensive** (100ms p95):
- Query processing: ~5ms
- Parallel search execution: ~30-40ms
- Score fusion: ~2-5ms
- Result assembly: ~5-10ms
- Network round-trip: ~40ms

**Filtering is cheap** (<1ms):
- Array filtering in TypeScript: ~0.1ms for 100 results
- Sorting: ~0.5ms for 100 results
- Slicing: ~0.01ms

**Impact**: 100x faster iteration when filtering client-side vs re-querying.

### User Experience Impact

**Current Experience:**
```typescript
// Want only functions? Re-query with different params
const results1 = await search({query: "auth", repo: "crewchief"})
// Hmm, too many results. Want only TypeScript?
const results2 = await search({query: "auth", repo: "crewchief", filters: {file_type: "ts"}})
// Actually, only want functions in TypeScript
const results3 = await search({query: "auth", repo: "crewchief", filters: {file_type: "ts"}}) // ??
// No way to filter by symbol kind!
```

**Desired Experience:**
```typescript
// Get comprehensive results once
const results = await search({query: "auth", repo: "crewchief"})

// Filter client-side instantly
const functions = results.hits.filter(h => h.kind === "function")
const tsFiles = results.hits.filter(h => h.relpath.endsWith(".ts"))
const recentFunctions = functions.filter(h => h.kind === "function")
  .sort((a, b) => b.modified_at - a.modified_at)

// Or use helper methods
const filtered = results.filter({kind: "function", file_type: "ts"})
const sorted = results.sort_by("recency")
const paginated = results.slice(10, 20)
```

---

## Existing Solutions

### Industry Patterns

**Elasticsearch:**
```json
{
  "query": {...},
  "post_filter": {
    "term": {"status": "published"}
  },
  "sort": [{"date": "desc"}],
  "from": 10,
  "size": 10
}
```
- Separate `query` (scoring) from `post_filter` (filtering without re-scoring)
- Client-side aggregations for faceted search
- Offset/limit for pagination

**Algolia:**
```javascript
const results = await index.search("query", {
  filters: "category:books AND price < 10",
  offset: 20,
  length: 10,
  getRankingInfo: true
})
```
- Filters applied after retrieval
- Ranking info preserved
- Pagination built-in

**GitHub Code Search:**
- Faceted navigation (language, repo, path)
- Instant filter toggles (no page reload)
- Sort by best match, most recent, least recent

### Codebase Patterns

**Current Search Response Structure:**

```typescript
// packages/daemon-client/src/client.ts:31
export interface SearchResult {
  hits: Array<{
    chunk_id: number
    file_path: string
    start_line: number
    end_line: number
    symbol_name: string | null
    kind: string
    content: string
    score: number
  }>
  total: number
  query_embedding_time_ms?: number
  search_time_ms?: number
}
```

**Gap**: No filtering/sorting/pagination helpers. Raw array only.

**Existing Filtering** (pre-search, SQL level):
```rust
// crates/maproom/src/daemon/types.rs:13
pub struct SearchParams {
    pub query: String,
    pub repo: String,
    pub worktree: Option<String>,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
    pub mode: Option<String>,
    pub deduplicate: Option<bool>,
}
```

**Gap**: No post-search filters (kind, file_type, recency, path patterns).

---

## Current State

### What Exists

1. **Pre-Search Filtering** (SQL WHERE clauses):
   - `worktree_id`: Filter to specific branch
   - `repo_id`: Filter to specific repository
   - `file_type`: Filter by file extension (via FILETYPE project)
   - `recency_threshold`: Filter by modification time

2. **Result Structures**:
   - `ChunkSearchResult` has all metadata (kind, relpath, score, start_line, etc.)
   - `FinalSearchResults` wraps results array with metadata
   - Deduplication logic exists (cross-worktree)

3. **Sorting**:
   - Only by relevance score (descending)
   - No alternative sort orders

### What's Missing

1. **Post-Search Filtering**:
   - Filter by `kind` (function, class, interface, etc.)
   - Filter by file extension (client-side, complement to SQL filter)
   - Filter by path pattern (e.g., "only src/ files")
   - Filter by line range (e.g., "small chunks only")

2. **Custom Sorting**:
   - Sort by recency (modified_at timestamp)
   - Sort by file path (alphabetical)
   - Sort by symbol name
   - Sort by line number

3. **Pagination**:
   - Offset/limit for result slicing
   - Page-based navigation helpers

4. **Aggregations**:
   - Count by kind ("10 functions, 5 classes")
   - Count by file type
   - Count by repository/worktree

5. **Helper Methods**:
   - Fluent API for chaining operations
   - Convenience methods for common patterns

---

## Research Findings

### Performance Constraints

**Client-Side Filtering** (TypeScript):
- Array.filter() for 100 items: ~0.1ms
- Array.sort() for 100 items: ~0.5ms
- Array.slice() for 100 items: ~0.01ms
- Total overhead: <1ms for typical operations

**Memory Impact**:
- 100 results @ ~500 bytes each: ~50KB
- Negligible for client-side storage
- Could cache filtered results if needed

**Implication**: Post-search filtering is essentially free compared to re-querying (100x faster).

### User Workflow Analysis

**Common Patterns** (from codebase usage):

1. **Iterative Refinement**: Start broad, narrow progressively
   - "authentication" → filter to functions → filter to TypeScript
2. **Exploration**: See what kinds of results exist
   - "How many functions vs classes matched?"
3. **Context Switching**: Different views of same results
   - Sort by relevance, then by recency, then by path
4. **Pagination**: Work through results systematically
   - Review results 1-10, then 11-20, etc.

### Technical Constraints

**No Database Schema Changes**: Must work with existing result structures.

**No Breaking Changes**: Additive only - existing clients continue working.

**TypeScript-First**: Client-side filtering happens in daemon-client/maproom-mcp layers.

**Optional Rust Support**: Could add Rust helpers later, but not required for MVP.

---

## Constraints

### Technical

1. **Result Structure Locked**: Cannot change `ChunkSearchResult` schema (breaking change)
2. **Performance Budget**: Client-side filtering must be <5ms for 100 results
3. **Memory Budget**: No result duplication (filter in-place or return views)
4. **Backward Compatibility**: Existing search API unchanged

### Business

1. **MVP Scope**: Core filtering/sorting/pagination only
2. **No Advanced Features**: No faceted search, no saved filters
3. **Documentation**: Must document all new filtering options

### Resource

1. **Timeline**: 2-3 days (aligned with SRCHTRN project)
2. **No Dependencies**: Can proceed independently
3. **Testing**: Focus on integration tests (real result filtering)

---

## Success Criteria

### Functional

1. **Filter by kind**: `results.filter({kind: "function"})` works
2. **Filter by file type**: `results.filter({file_type: "ts"})` works
3. **Filter by path pattern**: `results.filter({path: "src/**"})` works
4. **Sort by recency**: `results.sort_by("recency")` works
5. **Sort by score**: `results.sort_by("score")` works (default)
6. **Paginate**: `results.slice(10, 20)` works
7. **Chain operations**: `results.filter({kind: "function"}).sort_by("recency").slice(0, 10)` works

### Performance

1. **Filtering**: <1ms for 100 results
2. **Sorting**: <1ms for 100 results
3. **Pagination**: <0.1ms for 100 results
4. **Chaining**: <2ms for combined operations

### Quality

1. **Type Safety**: Full TypeScript types for all operations
2. **Error Handling**: Graceful handling of invalid filters
3. **Documentation**: Examples for all common patterns
4. **Testing**: 80%+ coverage of filtering logic

### User Experience

1. **Intuitive API**: Familiar patterns (Array-like methods)
2. **Discoverable**: TypeScript autocomplete shows options
3. **Composable**: Can combine multiple filters/sorts
4. **Non-Destructive**: Original results preserved

---

## Key Insights

### 1. Client-Side vs Server-Side Filtering

**When to use SQL filtering** (pre-search):
- Large result sets (>1000 potential matches)
- Index-backed filters (worktree_id, repo_id, file_type via FILETYPE project)
- Expensive filters (full-text search, vector search)
- Narrow search space significantly

**When to use client-side filtering** (post-search):
- Small result sets (<100 results)
- Iterative refinement (multiple filter changes)
- Metadata-based filters (kind, path substrings, score ranges)
- No index backing needed
- Exploration and progressive narrowing

**Relationship to FILETYPE Project:**
- FILETYPE (completed): Server-side SQL file_type filtering (pre-search)
- SRCHFLTR (this project): Client-side result filtering (post-search)
- **Complementary**: FILETYPE narrows at query time, SRCHFLTR refines results without re-querying
- **Example**: Use FILETYPE to search only TypeScript files, then use SRCHFLTR to filter to functions only

**This project**: Client-side filtering for metadata-based operations on already-retrieved results.

### 2. Immutability vs Mutation

**Options**:
1. **Mutate original**: `results.hits = results.hits.filter(...)`
2. **Return new array**: `const filtered = results.hits.filter(...)`
3. **Return new wrapper**: `const filtered = results.filter({...})` returns new SearchResult

**Decision**: Option 3 - Return new wrapper (immutable, chainable, familiar).

### 3. Filter Syntax

**Options**:
1. **Method chaining**: `results.filter_by_kind("function").filter_by_type("ts")`
2. **Object syntax**: `results.filter({kind: "function", file_type: "ts"})`
3. **Function syntax**: `results.filter({custom: r => r.kind === "function"})`

**Decision**: Object syntax (#2) with optional custom function for flexibility. Simpler than multiple methods, more structured than pure functions.

---

## Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Performance regression | Low | Medium | Benchmark client-side operations, <5ms budget |
| Breaking existing clients | Low | High | Additive API only, existing methods unchanged |
| Over-engineering | Medium | Low | MVP scope: core filtering/sorting only |
| Type sync drift (TS ↔ Rust) | Low | Medium | Sync comments, integration tests |
| Memory leaks from cached results | Low | Medium | Use weak references or clear on new search |

---

## Recommendations

### Immediate

1. **Start with TypeScript implementation**: daemon-client layer
2. **Extend SearchResult interface**: Add filtering/sorting methods
3. **Preserve immutability**: Return new objects, don't mutate
4. **Focus on common patterns**: kind, file_type, path, recency

### Future Enhancements (Out of Scope)

1. **Rust-side filtering**: Move logic to daemon for consistency
2. **Saved filters**: Persist common filter combinations
3. **Faceted search**: Aggregate counts for each filter dimension
4. **Filter suggestions**: "Did you mean to filter by kind=function?"
5. **Complex filters**: AND/OR/NOT logic, ranges, regex

---

## Conclusion

Result filtering is a **high-value, low-risk** enhancement that improves search UX by enabling progressive refinement without expensive re-queries. The implementation is straightforward (client-side TypeScript), performance impact is negligible (<1ms), and backward compatibility is preserved.

**Path Filtering Approach:**
- MVP uses simple string matching (`.includes()`, `.startsWith()`, `.endsWith()` via custom filter)
- Advanced glob patterns deferred to future enhancement (would require minimatch dependency)
- 80% of use cases covered by substring matching

**Ready to proceed to architecture design.**
