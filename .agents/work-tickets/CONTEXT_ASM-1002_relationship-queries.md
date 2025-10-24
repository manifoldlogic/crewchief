# Ticket: CONTEXT_ASM-1002: Relationship Queries

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-context-engineer
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement graph traversal queries for discovering code relationships through chunk_edges. Enable finding test files, callers, callees, and related code chunks via bidirectional edge traversal with depth limiting and relevance scoring.

## Background
The Context Assembly System needs to traverse code relationships to build comprehensive context windows. This includes:
- Finding test files for a given implementation chunk
- Discovering caller/callee relationships
- Building relationship graphs with configurable depth
- Applying relevance decay for distant relationships

This is Phase 1, Week 1, Task 2 from the CONTEXT_ASM planning document. It builds on the chunk_edges table structure to provide semantic code navigation.

## Acceptance Criteria
- [ ] Find related chunks via edges with bidirectional traversal
- [ ] Graph traversal with depth limiting working (configurable max depth)
- [ ] Test file detection functional via test_of edges
- [ ] Callers and callees identified via calls edges
- [ ] Relevance decay factor applied (0.7 per hop as specified in architecture)
- [ ] Query returns results ordered by relevance score
- [ ] Unit tests demonstrate all relationship types working

## Technical Requirements
- Implement recursive CTE for graph traversal as specified in CONTEXT_ASM_ARCHITECTURE.md (lines 34-62)
- Follow chunk_edges bidirectionally (both src_chunk_id and dst_chunk_id)
- Apply relevance decay factor of 0.7 per hop
- Support multiple relationship types:
  - `test_of` - links test files to implementation
  - `calls` - function/method call relationships
  - `imports` - module import relationships
- Depth limiting parameter to prevent unbounded queries
- Return results with relevance scores for ranking
- Handle edge metadata (relationship type, confidence)

## Implementation Notes

### Graph Traversal Architecture
Based on CONTEXT_ASM_ARCHITECTURE.md, implement:

1. **Recursive CTE Pattern**:
   ```sql
   WITH RECURSIVE related AS (
     -- Start with target chunk
     SELECT id, 0 as depth, 1.0 as relevance
     FROM maproom.chunks WHERE id = $1

     UNION ALL

     -- Follow edges up to max depth
     SELECT
       CASE
         WHEN e.src_chunk_id = r.id THEN e.dst_chunk_id
         ELSE e.src_chunk_id
       END as id,
       r.depth + 1,
       r.relevance * 0.7  -- Decay factor
     FROM related r
     JOIN maproom.chunk_edges e ON (
       e.src_chunk_id = r.id OR e.dst_chunk_id = r.id
     )
     WHERE r.depth < $2
   )
   ```

2. **Relationship Type Filtering**:
   - Add WHERE clauses to filter by edge_type
   - Support multiple types in single query
   - Enable directional vs bidirectional traversal

3. **Rust Module Structure**:
   - `crates/maproom/src/context/graph.rs` - Core graph traversal logic
   - `crates/maproom/src/context/relationships.rs` - Relationship-specific queries
   - Use sqlx for parameterized queries
   - Implement async/await pattern with tokio

4. **Query Functions to Implement**:
   - `find_related_chunks(chunk_id, max_depth, edge_types)` - General traversal
   - `find_test_files(chunk_id)` - Specific test_of lookup
   - `find_callers(chunk_id, max_depth)` - Follow calls edges backward
   - `find_callees(chunk_id, max_depth)` - Follow calls edges forward
   - `find_imports(chunk_id)` - Module dependency lookup

5. **Testing Strategy**:
   - Create test fixtures with known relationship graphs
   - Verify depth limiting works correctly
   - Check relevance score calculations
   - Test bidirectional vs unidirectional traversal
   - Validate edge type filtering

### Integration Points
- Connects to PostgreSQL via existing database pool
- Uses chunk_edges table populated by edge extraction
- Feeds into Priority Ranker for context window assembly
- Supports Context Assembler query planning

## Dependencies
- chunk_edges table must be populated with relationship data
- Chunks table must have valid IDs for traversal
- Database connection pool configured
- sqlx query macros available

## Risk Assessment
- **Risk**: Unbounded graph traversal causing performance issues
  - **Mitigation**: Strict depth limiting with sensible defaults (e.g., max_depth=3)

- **Risk**: Circular references causing infinite loops
  - **Mitigation**: Use DISTINCT in CTE and track visited nodes

- **Risk**: Large result sets consuming excessive memory
  - **Mitigation**: Implement result limiting and pagination support

- **Risk**: Complex edge metadata slowing queries
  - **Mitigation**: Index chunk_edges on both src_chunk_id and dst_chunk_id

## Files/Packages Affected
- `crates/maproom/src/context/graph.rs` (new) - Graph traversal queries
- `crates/maproom/src/context/relationships.rs` (new) - Relationship finding logic
- `crates/maproom/src/context/mod.rs` - Module exports
- `crates/maproom/tests/context/graph_test.rs` (new) - Unit tests
- `crates/maproom/tests/context/relationship_test.rs` (new) - Integration tests
- `crates/maproom/src/db/schema.rs` - May need edge type enum definitions

## Planning Document References
- Architecture: `/workspace/crewchief_context/maproom/CONTEXT_ASM/CONTEXT_ASM_ARCHITECTURE.md` (lines 34-86)
- Graph Walker pattern (lines 34-62)
- Priority Ranker weights (lines 64-86)
- Phase 1 Plan: CONTEXT_ASM planning document, Phase 1, Week 1, Task 2
