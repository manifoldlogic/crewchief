# Ticket: HYBRID_SEARCH-2002: Parallel Search Execution

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- search-quality-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement parallel execution of FTS, vector similarity, graph signal, and recency/churn queries using tokio::join! for sub-100ms search latency. Each search type produces ranked results that will be fused by the score fusion engine.

## Background
The hybrid search system requires multiple search strategies to run concurrently to achieve target latency under 100ms. This ticket implements the four core search executors (FTS, vector, graph, signals) that will run in parallel as Stage 2 of the query pipeline. Each executor must return ranked results with normalized scores that can be combined by the RRF fusion algorithm.

From the architecture plan (Phase 2, Week 2, Task 2), this is the core search execution layer that sits between query processing (HYBRID_SEARCH-2001) and score fusion (HYBRID_SEARCH-2003).

## Acceptance Criteria
- [ ] FTS search executor returns ranked results with ts_rank_cd scoring
- [ ] Vector similarity search supports code/text/hybrid modes with pgvector
- [ ] Graph signal queries calculate importance from edge counts (callers, importers, tests)
- [ ] Recency and churn scores integrated from chunks table
- [ ] All four search types execute in parallel using tokio::join!
- [ ] Total parallel execution completes under 100ms for typical queries
- [ ] Each executor returns results in consistent RankedResults format
- [ ] Integration tests verify parallel execution and result formats

## Technical Requirements

### FTS Search (`crates/maproom/src/search/fts.rs`)
- Use PostgreSQL full-text search with ts_rank_cd ranking function
- Apply proximity boost (flag 32) for phrase matching
- Add exact match bonus (0.2) when query appears in symbol_name
- Support repository filtering via $repo_id parameter
- Return results sorted by (fts_score + exact_bonus) DESC
- Over-fetch by 3x limit for fusion (limit * 3)

### Vector Search (`crates/maproom/src/search/vector.rs`)
- Implement pgvector cosine distance operator (<=>)
- Support three search modes: 'code', 'text', 'hybrid'
- Calculate similarity as 1 - distance for normalized scores
- Hybrid mode: 60% code similarity + 40% text similarity
- Use ORDER BY distance for efficient index utilization
- Over-fetch by 3x limit for fusion (limit * 3)

### Graph Queries (`crates/maproom/src/search/graph.rs`)
- Calculate PageRank-like importance from chunk_edges table
- Count incoming edges by type: calls (0.3 weight), imports (0.2), tests (0.1)
- Use logarithmic scaling: LOG(2 + count) to dampen extreme values
- Handle chunks with no edges (COALESCE to 0)
- Over-fetch by 2x limit for fusion (limit * 2)

### Signal Integration (`crates/maproom/src/search/signals.rs`)
- Query recency_score and churn_score from chunks table
- Combine signals with configurable weights
- Default: recency 0.3, churn 0.2
- Return all chunks with signal scores (no limit)

### Parallel Execution (`crates/maproom/src/search/executors.rs`)
- Implement SearchExecutors struct with database connection pool
- Use tokio::join! to execute all four queries concurrently
- Handle errors from each executor independently
- Return RankedResults from each executor with:
  - chunk_id: i64
  - score: f32 (normalized 0.0-1.0)
  - rank: usize (1-based ranking)
- Total execution time target: < 100ms

### Database Schema Requirements
- Depends on vector columns (HYBRID_SEARCH-1002)
- Depends on populated embeddings (HYBRID_SEARCH-1003)
- Uses existing ts_doc column and GIN index
- Uses existing chunk_edges table
- Uses recency_score and churn_score columns

## Implementation Notes

### Architecture Reference
See `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`:
- Full-Text Search SQL (lines 121-148)
- Vector Search SQL (lines 150-180)
- Graph-Enhanced Ranking SQL (lines 182-205)
- Query Pipeline parallel execution (lines 313-319)

### Performance Considerations
- Use prepared statements for all queries to reduce planning overhead
- Leverage existing indexes: GIN for FTS, IVFFlat for vectors, B-tree for edges
- Over-fetch results to provide fusion engine with more candidates
- Parallel execution should complete in ~60-80ms under typical load
- Monitor query execution plans to ensure index usage

### RankedResults Format
```rust
pub struct RankedResults {
    pub results: Vec<RankedResult>,
    pub source: SearchSource,  // FTS, Vector, Graph, Signals
}

pub struct RankedResult {
    pub chunk_id: i64,
    pub score: f32,      // 0.0-1.0 normalized
    pub rank: usize,     // 1-based position
}

pub enum SearchSource {
    FTS,
    Vector,
    Graph,
    Signals,
}
```

### Error Handling
- Each executor must handle SQL errors gracefully
- Missing embeddings for vector search should not fail the query
- Empty result sets are valid (return empty Vec)
- Log execution times for each search type for monitoring
- Propagate errors up to query pipeline for proper handling

### Testing Strategy
- Unit tests for each executor with mock database
- Integration tests with real PostgreSQL database
- Benchmark tests to verify <100ms parallel execution
- Test edge cases: no results, missing embeddings, empty graph
- Verify result format consistency across all executors

## Dependencies
- **HYBRID_SEARCH-1002**: Database vector preparation (vector columns and IVFFlat indices)
- **HYBRID_SEARCH-1003**: Embedding generation pipeline (populated embeddings)
- **HYBRID_SEARCH-2001**: Query processor (generates query embeddings for vector search)

## Risk Assessment
- **Risk**: Parallel execution may exceed 100ms under high load or with large result sets
  - **Mitigation**: Implement query timeouts, monitor execution times, optimize indices if needed

- **Risk**: Vector search may be slow without proper IVFFlat index tuning
  - **Mitigation**: Benchmark with different ivfflat.probes settings, document optimal configuration

- **Risk**: Missing embeddings could cause vector search to return no results
  - **Mitigation**: Handle NULL embeddings gracefully, log warnings for chunks without embeddings

- **Risk**: Graph queries may be expensive for heavily connected chunks
  - **Mitigation**: Use COUNT FILTER for efficient edge type counting, add index on dst_chunk_id if needed

## Files/Packages Affected
- `crates/maproom/src/search/executors.rs` - Main SearchExecutors implementation with tokio::join!
- `crates/maproom/src/search/fts.rs` - Full-text search executor
- `crates/maproom/src/search/vector.rs` - Vector similarity search executor
- `crates/maproom/src/search/graph.rs` - Graph signal queries executor
- `crates/maproom/src/search/signals.rs` - Recency/churn score integration
- `crates/maproom/src/search/types.rs` - RankedResults and SearchSource types
- `crates/maproom/tests/search/executors_test.rs` - Integration tests for parallel execution
