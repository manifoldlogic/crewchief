# Ticket: HYBRID_SEARCH-6001: MCP Integration Update

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update the maproom MCP server to expose the new hybrid search system parameters, including search mode selection (fts/vector/hybrid), filter parameters, and debugging options. This work integrates the completed hybrid search backend (Phase 4) with the MCP tool interface, enabling AI assistants to leverage vector similarity and graph signals alongside full-text search.

## Background
The hybrid search system (Phases 1-4) has implemented a state-of-the-art retrieval system combining full-text search (FTS), vector similarity, and graph signals. The current MCP search tool (in `/workspace/packages/maproom-mcp/src/index.ts`) only exposes basic FTS parameters: `repo`, `worktree`, `query`, `k`, and `filter`.

To make the hybrid search capabilities accessible to AI assistants via MCP, we need to:
1. Add search mode selection to allow users to choose between FTS-only, vector-only, or hybrid search
2. Expose filter parameters for more precise result targeting (repo_id, worktree_id, file_type, recency)
3. Add debug mode to return score breakdowns and explanations
4. Maintain backward compatibility with existing MCP clients that use the current simple interface

This is the final user-facing component of Phase 6 (Production Rollout) that makes the hybrid search system accessible through Claude, Cursor, and other MCP-compatible AI tools.

## Acceptance Criteria
- [ ] MCP search tool updated with new parameters in tool schema
- [ ] Mode parameter implemented: "fts", "vector", "hybrid" (default: "hybrid")
- [ ] Filter parameters working: repo_id, worktree_id, file_type, recency_threshold
- [ ] Debug parameter returns score breakdown with explanations
- [ ] Backward compatibility maintained - existing calls without new parameters work as before
- [ ] Tool description updated with guidance on when to use each mode
- [ ] README documentation updated with parameter examples
- [ ] Tests added for new parameters and modes
- [ ] Error messages provide helpful guidance when parameters are invalid

## Technical Requirements
- **Update tool schema** in `handleSearch()` and `toolSchemas` array
  - Add `mode` parameter: enum ["fts", "vector", "hybrid"], default "hybrid"
  - Add `filters` object parameter with optional: repo_id, worktree_id, file_type, recency_threshold
  - Add `debug` boolean parameter, default false
- **Modify search execution** to pass mode parameter to backend
  - Query construction should vary based on mode selection
  - FTS mode: existing ts_rank_cd query (lines 346-383)
  - Vector mode: cosine similarity on embedding vectors
  - Hybrid mode: RRF combination of FTS and vector results
- **Implement filter parameter handling**
  - Extend WHERE clauses based on provided filters
  - repo_id and worktree_id filters already partially implemented
  - file_type should map to file extensions
  - recency_threshold filters by file modification time
- **Add score explanation in debug mode**
  - Return breakdown of FTS score, vector score, graph signals
  - Include weight contributions and fusion method used
  - Provide interpretation hints for scores
- **Maintain backward compatibility**
  - All new parameters must be optional with sensible defaults
  - Existing search calls should behave identically
  - Default to "hybrid" mode for best results
- **Update tool descriptions and documentation**
  - Add usage guidance to tool schema description
  - Document when to use each mode (FTS for exact matches, vector for semantic)
  - Update README with comprehensive examples

## Implementation Notes

### Current Search Implementation
The existing MCP search tool (lines 306-512 in `index.ts`) implements:
- Repository and worktree filtering
- Full-text search using PostgreSQL ts_vectors
- ts_rank_cd scoring with boost factors for headings
- File type filtering (code/docs/config)
- Comprehensive error handling and suggestions

### Changes Required
1. **Schema Extension** (lines 115-135)
   - Add mode, filters, and debug to inputSchema properties
   - Update description with mode selection guidance

2. **Query Modification** (lines 339-383)
   - Create separate query builders for each mode
   - FTS: current implementation (reuse existing logic)
   - Vector: `SELECT ... ORDER BY embedding <-> query_embedding LIMIT k`
   - Hybrid: UNION ALL with RRF fusion in application layer or use CTE

3. **Filter Implementation**
   - file_type: extend existing filter logic (lines 369-378)
   - recency_threshold: add `WHERE f.modified_at > NOW() - INTERVAL '$threshold'`
   - Additional repo/worktree filters already exist (lines 364-367)

4. **Debug Output Structure**
   ```typescript
   debug: {
     mode: 'hybrid',
     fts_score: 0.45,
     vector_score: 0.82,
     graph_score: 0.15,
     recency_score: 0.23,
     churn_score: 0.10,
     final_score: 0.67,
     fusion_method: 'rrf',
     weights: { fts: 0.4, vector: 0.35, graph: 0.1, recency: 0.1, churn: 0.05 }
   }
   ```

5. **Error Handling**
   - Invalid mode: suggest valid options
   - Missing embeddings for vector mode: fallback to FTS with warning
   - Invalid filters: descriptive error messages

### Architecture References
- **HYBRID_SEARCH_ARCHITECTURE.md** (lines 264-294): Configuration schema for fusion weights and parameters
- **HYBRID_SEARCH_PLAN.md** (lines 151-179): Phase 6 requirements and acceptance criteria
- Current MCP implementation follows JSON-RPC 2.0 over stdio with content-length framing

### Testing Strategy
- Unit tests for parameter validation
- Integration tests for each mode (fts, vector, hybrid)
- Test filter combinations (file_type + recency, etc.)
- Test debug output structure
- Test backward compatibility (existing calls work unchanged)
- Test error cases (invalid mode, missing embeddings, etc.)

### Performance Considerations
- Vector searches may be slower than FTS - document expected latency
- Hybrid mode combines results, adding overhead - target <100ms p95
- Cache embeddings for repeated queries
- Consider query timeout parameter for long-running searches

## Dependencies
- **HYBRID_SEARCH-4003** (or equivalent Phase 4 completion ticket): Complete hybrid search system with RRF fusion, vector search, and graph signals
- Embeddings must be generated for vector and hybrid modes to work
- PostgreSQL pgvector extension must be installed and configured
- Existing MCP server infrastructure (already present in `packages/maproom-mcp`)

## Risk Assessment
- **Risk**: Breaking changes to existing MCP clients
  - **Mitigation**: Make all new parameters optional with backward-compatible defaults. Test extensively with current search queries.

- **Risk**: Vector search returns no results if embeddings not generated
  - **Mitigation**: Detect missing embeddings and fall back to FTS with informative warning message. Document embedding generation requirement.

- **Risk**: Performance degradation with complex hybrid queries
  - **Mitigation**: Implement query timeouts, optimize SQL queries, add performance monitoring. Document expected latency ranges.

- **Risk**: Confusion about when to use which mode
  - **Mitigation**: Provide clear guidance in tool description and README. Default to "hybrid" for best overall results.

- **Risk**: Debug mode exposes internal implementation details
  - **Mitigation**: Document that debug output is for development/tuning only. Structure may change in future versions.

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/src/index.ts` - Main MCP server implementation
  - Update `toolSchemas` array (lines 115-135)
  - Modify `handleSearch()` function (lines 306-512)
  - Add helper functions for mode-specific query building
- `/workspace/packages/maproom-mcp/README.md` - Documentation
  - Add section on search modes
  - Document new parameters with examples
  - Add troubleshooting guide for vector mode
- `/workspace/packages/maproom-mcp/tests/search_tool_test.ts` (create if doesn't exist)
  - Unit tests for parameter validation
  - Integration tests for each search mode
  - Backward compatibility tests
- `/workspace/packages/maproom-mcp/package.json` - May need version bump
- Database queries will access:
  - `maproom.chunks` table (existing FTS and vector columns)
  - `maproom.files` table (for filters and metadata)
  - `maproom.worktrees` and `maproom.repos` tables (existing)
