# Ticket: HYBRID_SEARCH-2003: Initial Search Integration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- mcp-tools-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate query processing and parallel search executors into a unified SearchPipeline that performs basic score combination, result deduplication, simple ranking, and exposes an API endpoint for hybrid search queries.

## Background
This is the third ticket in Phase 2 (Week 2, Task 3) of the Hybrid Search implementation. With query processing (HYBRID_SEARCH-2001) and parallel search execution (HYBRID_SEARCH-2002) complete, we now need to integrate these components into a cohesive search pipeline. This ticket creates the orchestration layer that combines full-text search, vector search, and graph-enhanced results into a single ranked result set, exposed through an API endpoint (or MCP tool integration point).

The initial implementation uses simple weighted score combination as a placeholder for the sophisticated fusion algorithms to be implemented in Phase 3.

## Acceptance Criteria
- [ ] SearchPipeline struct created that orchestrates query processing, search execution, and result fusion
- [ ] Basic score fusion implemented using simple weighted average combination (placeholder for Phase 3 improvements)
- [ ] Result deduplication logic implemented to handle the same chunk appearing in multiple search results (deduplicate by chunk_id)
- [ ] SearchResults assembly implemented with complete metadata (query details, timing, result counts)
- [ ] Simple ranking pipeline functional - results ordered by combined score
- [ ] API endpoint created and accessible (HTTP endpoint or MCP tool handler)
- [ ] End-to-end search flow works: query input → processing → execution → fusion → ranked results
- [ ] Basic integration tests verify pipeline with sample queries

## Technical Requirements
- Create `SearchPipeline` struct integrating:
  - `QueryProcessor` from HYBRID_SEARCH-2001
  - `SearchExecutors` from HYBRID_SEARCH-2002
  - Basic `ScoreFusion` trait and simple implementation
  - Optional reranker placeholder (None initially)
- Implement basic score combination:
  - Simple weighted average: `fts_weight * fts_score + vector_weight * vector_score + graph_weight * graph_score`
  - Default weights: FTS=0.4, Vector=0.4, Graph=0.2 (configurable)
  - Normalize scores to 0.0-1.0 range before combination
- Result deduplication:
  - Track chunk IDs across all search result streams
  - When same chunk appears in multiple results, use highest individual score or sum weighted scores
  - Preserve source metadata (which search types found the chunk)
- SearchResults structure:
  - Query string
  - Array of ranked results with chunk details
  - Metadata: query processing details, timing, result counts per search type
- API endpoint requirements:
  - Accept query string and search options (limit, filters, weights)
  - Return JSON with SearchResults
  - Handle errors gracefully
  - Consider MCP tool integration as alternative/addition to HTTP endpoint
- Follow architecture specification in `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md` lines 296-340 (Query Pipeline section)

## Implementation Notes

### Architecture Reference
The complete search pipeline flow is documented in the architecture document at lines 296-340. Key implementation details:

**SearchPipeline Structure** (from architecture):
```rust
pub struct SearchPipeline {
    processor: QueryProcessor,
    executors: SearchExecutors,
    fusion: Box<dyn ScoreFusion>,
    reranker: Option<CrossEncoder>,
}
```

**Pipeline Stages**:
1. **Stage 1: Process query** - Use QueryProcessor from HYBRID_SEARCH-2001
2. **Stage 2: Execute parallel searches** - Use tokio::join! for concurrent FTS, vector, graph, and signal searches
3. **Stage 3: Fuse scores** - Combine results using basic weighted fusion
4. **Stage 4: Optional reranking** - Placeholder (None) for Phase 3 cross-encoder reranking

**Score Fusion Strategy** (Phase 2 - Simple):
- Implement `ScoreFusion` trait with basic weighted average method
- Normalize all scores to 0.0-1.0 before fusion
- Apply configurable weights (default: FTS=0.4, Vector=0.4, Graph=0.2)
- More sophisticated fusion (RRF, learned weights) deferred to Phase 3

**Deduplication Logic**:
- Use HashMap<chunk_id, SearchResult> to track unique chunks
- When duplicate found, decide merge strategy:
  - Option A: Keep highest individual score
  - Option B: Sum weighted scores from different search types
  - Track which search types found each chunk (for debugging/metadata)

**SearchResults Assembly**:
```rust
pub struct SearchResults {
    query: String,
    results: Vec<ScoredChunk>,
    metadata: SearchMetadata,
}

pub struct SearchMetadata {
    query_processing: QueryProcessingDetails,
    result_counts: HashMap<SearchType, usize>,
    timing: SearchTiming,
}
```

**API Endpoint Options**:
- HTTP endpoint: `POST /api/search` with JSON body
- MCP tool handler: Integrate with existing MCP server infrastructure
- Consider both options for maximum flexibility

**Error Handling**:
- Gracefully handle partial search failures (e.g., vector search fails but FTS succeeds)
- Log search failures but continue with available results
- Return error metadata in SearchResults

**Testing Approach**:
- Unit tests for score fusion logic with mock search results
- Unit tests for deduplication with overlapping result sets
- Integration tests with end-to-end query flow
- Test error scenarios (empty results, partial failures)

### Files to Create
- `crates/maproom/src/search/pipeline.rs` - SearchPipeline orchestration
- `crates/maproom/src/search/fusion.rs` - ScoreFusion trait and basic implementation
- `crates/maproom/src/search/results.rs` - SearchResults and metadata structures
- `crates/maproom/src/api/search.rs` - API endpoint or MCP tool handler (or both)

### Files to Modify
- `crates/maproom/src/search/mod.rs` - Export new modules
- `crates/maproom/src/api/mod.rs` - Register new endpoint (if HTTP)
- `crates/maproom/src/mcp/mod.rs` - Register new tool (if MCP)

### Key Design Decisions
1. **Simple fusion first**: Phase 2 uses basic weighted average; sophisticated algorithms (RRF, learned weights) in Phase 3
2. **Deduplication strategy**: Recommend sum of weighted scores to preserve signal from multiple search types
3. **API vs MCP**: Both options valid - coordinate with mcp-tools-engineer for integration point preference
4. **Error resilience**: Partial results better than complete failure
5. **Extensibility**: Design ScoreFusion as trait to easily swap implementations in Phase 3

## Dependencies
- **HYBRID_SEARCH-2001** (query processing) - Required for QueryProcessor integration
- **HYBRID_SEARCH-2002** (parallel search execution) - Required for SearchExecutors integration
- Both must be complete before this ticket can begin

## Risk Assessment
- **Risk**: Score normalization may not work well across different search types with different score ranges
  - **Mitigation**: Use percentile-based normalization or min-max scaling per search type; monitor score distributions in testing

- **Risk**: Simple weighted average may produce poor ranking compared to existing search
  - **Mitigation**: This is expected for Phase 2; document baseline metrics for comparison with Phase 3 improvements; ensure weights are configurable for tuning

- **Risk**: Deduplication logic may lose important signal when merging results
  - **Mitigation**: Preserve source metadata (which search types found chunk); consider sum of weighted scores rather than max score

- **Risk**: API endpoint design may not align with MCP tool requirements
  - **Mitigation**: Coordinate with mcp-tools-engineer early; consider supporting both HTTP and MCP interfaces

- **Risk**: Pipeline orchestration complexity may introduce latency
  - **Mitigation**: Use parallel execution (tokio::join!); measure and log timing for each stage; optimize in Phase 3 if needed

## Files/Packages Affected
- `crates/maproom/src/search/pipeline.rs` (new)
- `crates/maproom/src/search/fusion.rs` (new)
- `crates/maproom/src/search/results.rs` (new)
- `crates/maproom/src/api/search.rs` (new)
- `crates/maproom/src/search/mod.rs` (modified)
- `crates/maproom/src/api/mod.rs` (modified, if HTTP endpoint)
- `crates/maproom/src/mcp/mod.rs` (modified, if MCP tool)
- Integration tests: `crates/maproom/tests/search_pipeline_integration_test.rs` (new)
