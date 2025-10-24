# Ticket: CONTEXT_ASM-2001: Importance Scoring

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (24/24: 13 unit + 11 integration)
- [x] **Verified** - by the verify-ticket agent

## Agents
- mcp-context-engineer
- database-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement importance scoring system for chunks in context assembly. Calculate relevance scores based on relationship types, distance decay, and chunk metadata (importance, recency, churn). This enables intelligent prioritization of related code chunks when assembling context bundles within token budgets.

## Background
The context assembly engine needs to prioritize which related chunks to include when operating under token budget constraints. A naive approach that treats all relationships equally will produce suboptimal context bundles. We need a sophisticated scoring system that considers:

1. **Relationship semantics**: Tests are more important than imports
2. **Graph distance**: Closer chunks are more relevant than distant ones
3. **Code importance**: Central/frequently-called code matters more
4. **Recency**: Recently changed code may be more relevant
5. **Churn rate**: High-churn areas indicate active development

This ticket implements the Priority Ranker component described in the architecture document (lines 64-86), which provides the scoring foundation for intelligent context assembly.

## Acceptance Criteria
- [x] Base relevance scoring implemented with configurable starting score (default 1.0)
- [x] Relationship type weighting applied correctly (test_of=1.5, calls=1.2, imports=1.1)
- [x] Distance decay factor implemented (score *= 0.7^distance)
- [x] Chunk importance_score multiplier integrated from database
- [x] Same directory bonus applied (score *= 1.3 for co-located chunks)
- [x] Recency score considered in final ranking
- [x] Churn score integrated for active development areas
- [x] Unit tests cover all scoring factors independently
- [x] Integration tests verify combined scoring behavior
- [x] Performance benchmarks show scoring completes within 10ms for 1000 chunks

## Technical Requirements

### Core Scoring Function
- Implement `calculate_importance_score(chunk, relationship, target_chunk) -> f64`
- Support configurable weights for relationship types
- Apply multiplicative decay based on graph distance
- Normalize scores to 0.0-1.0 range for consistency

### Relationship Type Weights
```rust
pub const RELATIONSHIP_WEIGHTS: &[(RelationshipType, f64)] = &[
    (RelationshipType::TestOf, 1.5),
    (RelationshipType::Calls, 1.2),
    (RelationshipType::Imports, 1.1),
    (RelationshipType::Uses, 1.0),
];
```

### Distance Decay
- Exponential decay: `score *= decay_factor.powf(distance as f64)`
- Default decay factor: 0.7
- Configurable via context assembly options

### Metadata Integration
- Pull `importance_score`, `recency_score`, `churn_score` from chunks table
- Apply as multiplicative factors: `score *= chunk.importance_score * chunk.recency_score * chunk.churn_score`
- Handle missing/null scores gracefully (default to 1.0)

### Directory Co-location Bonus
- Extract directory paths from file paths
- Apply 1.3x multiplier when `chunk.file.directory == target_chunk.file.directory`
- Optional: graduated bonus (same dir=1.3, parent/child dir=1.15)

### Rust Implementation Structure
```rust
pub struct ImportanceScorer {
    config: ScoringConfig,
}

pub struct ScoringConfig {
    pub base_score: f64,
    pub decay_factor: f64,
    pub relationship_weights: HashMap<RelationshipType, f64>,
    pub directory_bonus: f64,
}

impl ImportanceScorer {
    pub fn score(
        &self,
        chunk: &Chunk,
        relationship: &Relationship,
        target: &Chunk,
    ) -> f64 {
        // Implementation
    }

    fn apply_relationship_weight(&self, score: f64, rel_type: RelationshipType) -> f64;
    fn apply_distance_decay(&self, score: f64, distance: u32) -> f64;
    fn apply_metadata_scores(&self, score: f64, chunk: &Chunk) -> f64;
    fn apply_directory_bonus(&self, score: f64, chunk: &Chunk, target: &Chunk) -> f64;
}
```

## Implementation Notes

### Algorithm Flow
1. Start with base score (1.0)
2. Apply relationship type weight
3. Apply distance decay
4. Multiply by chunk.importance_score (from DB)
5. Multiply by chunk.recency_score (from DB)
6. Multiply by chunk.churn_score (from DB)
7. Apply directory bonus if applicable
8. Clamp/normalize if needed

### Performance Considerations
- Scoring happens frequently during graph traversal
- Pre-load chunk metadata in batch queries to avoid N+1 problems
- Cache directory path extractions
- Consider SIMD for batch scoring operations if needed

### Testing Strategy
- **Unit tests**: Each scoring factor in isolation
  - Relationship weights applied correctly
  - Distance decay follows exponential curve
  - Directory bonus only applies when paths match
  - Metadata multipliers handle nulls gracefully
- **Integration tests**: Combined scoring scenarios
  - Test suite chunk scores higher than random import
  - Nearby chunks score higher than distant ones
  - Recent/high-churn code gets appropriate boost
- **Property tests**: Invariants hold
  - Scores are always non-negative
  - Closer distance always yields higher score (all else equal)
  - Stronger relationship type yields higher score (all else equal)

### Configuration Options
Expose via `ExpandOptions` or separate `ScoringOptions`:
```rust
pub struct ScoringOptions {
    pub base_score: Option<f64>,
    pub decay_factor: Option<f64>,
    pub relationship_weights: Option<HashMap<RelationshipType, f64>>,
    pub directory_bonus: Option<f64>,
    pub include_recency: bool,
    pub include_churn: bool,
}
```

### Integration Points
- Used by graph walker during traversal (CONTEXT_ASM-1002)
- Consumed by priority queue in assembler (CONTEXT_ASM-1001)
- Feeds into budget manager decisions (CONTEXT_ASM-1003)

### Edge Cases
- **Missing metadata**: Default to 1.0 multiplier
- **Zero distance**: Handle specially (target chunk itself)
- **Negative scores**: Should never happen, but clamp to 0.0 if detected
- **Infinite scores**: Cap at reasonable maximum (e.g., 100.0)
- **Circular relationships**: Distance prevents infinite loops

## Dependencies
- **CONTEXT_ASM-1002** (relationship queries) - Must be complete to provide relationship data
- **Database schema**: Requires `importance_score`, `recency_score`, `churn_score` columns on chunks table
- **Graph traversal**: Distance information must be available from recursive CTE

## Risk Assessment
- **Risk**: Scoring weights are arbitrary and may not reflect actual importance
  - **Mitigation**: Make weights configurable; gather usage data to tune empirically; consider ML-based scoring in future

- **Risk**: Performance degradation with large graphs (1000+ related chunks)
  - **Mitigation**: Batch metadata loading; profile and optimize hot paths; consider pre-computing scores for common patterns

- **Risk**: Recency/churn data may be stale or missing for some repositories
  - **Mitigation**: Graceful defaults (1.0); document data requirements; add warnings when metadata is sparse

- **Risk**: Complex scoring logic makes debugging context assembly difficult
  - **Mitigation**: Comprehensive logging of score calculations; debug mode that shows score breakdown; unit tests for each factor

## Files/Packages Affected
- **New files**:
  - `crates/maproom/src/context/importance.rs` - Core importance scoring implementation
  - `crates/maproom/src/context/ranker.rs` - Priority ranking using scores
  - `crates/maproom/tests/context/importance_test.rs` - Unit tests for scoring
  - `crates/maproom/tests/context/integration/scoring_test.rs` - Integration tests

- **Modified files**:
  - `crates/maproom/src/context/graph.rs` - Include importance scores in graph queries
  - `crates/maproom/src/context/mod.rs` - Export new scoring modules
  - `crates/maproom/src/context/assembler.rs` - Use scores for prioritization
  - `crates/maproom/src/db/schema.rs` - Ensure metadata fields are accessible

- **Database queries**:
  - Extend graph traversal CTE to include chunk metadata
  - Add batch loading for chunk importance/recency/churn scores
