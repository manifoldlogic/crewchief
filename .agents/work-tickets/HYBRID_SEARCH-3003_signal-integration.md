# Ticket: HYBRID_SEARCH-3003: Graph and Temporal Signal Integration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate graph importance scores, recency decay, and churn score normalization into the hybrid search weighted fusion system. This completes the signal integration layer by adding graph-based, temporal, and code stability signals to the search ranking algorithm.

## Background
Phase 3 of the hybrid search system builds on the weighted fusion framework (HYBRID_SEARCH-3002) by adding additional ranking signals beyond FTS and vector search. Graph signals capture code importance through relationships (callers, importers, tests), recency decay prioritizes recently modified code, and churn scores help identify stable vs volatile code. These signals provide a more comprehensive understanding of code relevance beyond just textual and semantic matching.

This work implements the signal integration strategy defined in the HYBRID_SEARCH_ARCHITECTURE.md document, specifically the Graph-Enhanced Ranking (lines 182-205) and Weighted Linear Combination (lines 251-258) components.

## Acceptance Criteria
- [ ] Graph importance calculation implemented using LOG-based PageRank-like scoring
- [ ] Recency decay calculation working with exponential decay over time
- [ ] Churn score normalization implemented (1.0 / (1.0 + churn_score))
- [ ] All signals integrated into weighted fusion framework from HYBRID_SEARCH-3002
- [ ] Signal-specific debugging utilities available for score explanation
- [ ] Unit tests for each signal calculation
- [ ] Integration tests showing all signals contributing to final ranking
- [ ] Documentation explaining signal weights and tuning parameters

## Technical Requirements

### Graph Importance Signal
- Calculate graph score from chunk_edges table
- Formula: LOG(2 + callers) * 0.3 + LOG(2 + importers) * 0.2 + LOG(2 + tests) * 0.1
- Handle chunks with no edges (score = 0.0)
- Use PostgreSQL LEFT JOIN with edge_counts CTE pattern from architecture

### Recency Decay Signal
- Implement exponential decay: score = exp(-lambda * days_since_update)
- Default lambda parameter for decay rate tuning
- Calculate days_since_update from chunks.updated_at timestamp
- Handle NULL timestamps gracefully

### Churn Score Normalization
- Normalize churn using: 1.0 / (1.0 + churn_score)
- Fetch raw churn_score from chunks table
- Ensure normalized score is in [0, 1] range
- Lower churn (stable code) should yield higher scores

### Signal Integration
- Extend SearchSignals struct to include graph_score, recency_score, churn_score
- Update WeightedFusion to apply FusionWeights from architecture:
  - fts: 0.4
  - vector: 0.35
  - graph: 0.1
  - recency: 0.1
  - churn: 0.05
- Ensure all signals are normalized to comparable ranges before fusion

### Signal Debugging
- Create signal explanation format showing individual signal contributions
- Add debug mode that logs raw and normalized signal values
- Provide per-result signal breakdown in search responses
- Include weight application details for troubleshooting

## Implementation Notes

### Module Structure
Create new signal modules in `crates/maproom/src/search/signals/`:
- `graph.rs` - Graph importance calculation
- `recency.rs` - Recency decay calculation
- `churn.rs` - Churn score normalization
- `debug.rs` - Signal debugging and explanation utilities
- `mod.rs` - Module exports and shared types

### Architecture Alignment
Follow the SQL pattern from HYBRID_SEARCH_ARCHITECTURE.md lines 182-205:
```sql
WITH edge_counts AS (
  SELECT
    dst_chunk_id as chunk_id,
    COUNT(*) FILTER (WHERE type = 'calls') as callers,
    COUNT(*) FILTER (WHERE type = 'imports') as importers,
    COUNT(*) FILTER (WHERE type = 'test_of') as tests
  FROM maproom.chunk_edges
  GROUP BY dst_chunk_id
)
```

Translate this to Rust using sqlx queries or integrate into existing search queries.

### Integration with Weighted Fusion
Update `crates/maproom/src/search/fusion/weighted.rs` to:
1. Accept expanded SearchSignals with all signal types
2. Apply weights from FusionWeights struct
3. Return FusedResult with score explanation metadata
4. Support signal debugging mode via configuration

### Testing Strategy
- Unit tests for each signal calculation with known inputs
- Test edge cases: NULL values, zero edges, extreme dates
- Integration test with real chunk data showing signal contributions
- Performance test ensuring signal calculation doesn't slow queries
- Verification that weights sum correctly and scores are normalized

### Performance Considerations
- Graph signal may require JOINs across large chunk_edges table
- Consider query optimization with appropriate indexes
- May need to cache edge counts if recalculation is expensive
- Recency and churn are simple calculations on chunk columns (fast)

## Dependencies
- HYBRID_SEARCH-3002 (weighted fusion framework) - MUST be completed first
- Existing chunk_edges table with graph relationship data
- chunks.updated_at and chunks.churn_score columns must exist

## Risk Assessment
- **Risk**: Graph signal queries may be slow on large codebases with many edges
  - **Mitigation**: Add indexes on chunk_edges (dst_chunk_id, type), consider materialized views for edge counts if performance issues arise

- **Risk**: Signal weights (0.4, 0.35, 0.1, 0.1, 0.05) may not be optimal for all codebases
  - **Mitigation**: Make weights configurable via search parameters, document tuning guidance, plan for future A/B testing framework

- **Risk**: Recency decay may overly prioritize recent but low-quality code
  - **Mitigation**: Keep recency weight low (0.1), allow lambda parameter tuning, combine with other quality signals

- **Risk**: Churn score interpretation may vary by codebase maturity
  - **Mitigation**: Normalize churn score, use low weight (0.05), document that stable code gets higher scores

## Files/Packages Affected
- `crates/maproom/src/search/signals/graph.rs` (NEW)
- `crates/maproom/src/search/signals/recency.rs` (NEW)
- `crates/maproom/src/search/signals/churn.rs` (NEW)
- `crates/maproom/src/search/signals/debug.rs` (NEW)
- `crates/maproom/src/search/signals/mod.rs` (NEW or MODIFIED)
- `crates/maproom/src/search/fusion/weighted.rs` (MODIFIED - integrate signals)
- `crates/maproom/src/search/mod.rs` (MODIFIED - wire up signal modules)
- `crates/maproom/src/search/types.rs` (MODIFIED - extend SearchSignals struct)
