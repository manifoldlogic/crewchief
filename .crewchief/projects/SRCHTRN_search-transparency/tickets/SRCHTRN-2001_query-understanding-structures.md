# SRCHTRN-2001: Query Understanding Structures

## Title
Create Rust structures for query understanding metadata

## Status
- [x] **Implementation Complete**
- [x] **Tests Passing**
- [x] **Verified**
- [x] **Committed**

## Agents
- **Primary**: rust-engineer
- **unit-test-runner**: Execute tests
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Extend `crates/maproom/src/search/results.rs` with `QueryUnderstanding`, `QueryFilters`, and `TimingBreakdown` structures. Add optional `understanding` field to `SearchMetadata` for successful search responses.

## Background
Phase 1 established structured error diagnostics. Phase 2 adds query understanding feedback to successful searches, showing users how queries are interpreted, what filters were applied, and timing breakdown. This ticket creates the Rust data structures that will be populated in SRCHTRN-2002.

**Design Principle**: Expose existing data, don't compute new data. Query understanding information already exists in `ProcessedQuery` and `SearchMetadata`.

## Acceptance Criteria
- [ ] `QueryUnderstanding` struct created with mode, tokens, expanded_terms, filters, fusion_strategy, timing fields
- [ ] `QueryFilters` struct created with repo_id, worktree_id, file_types, recency_threshold fields
- [ ] `TimingBreakdown` struct created with query_processing_ms, search_execution_ms, score_fusion_ms, result_assembly_ms, total_ms fields
- [ ] `SearchMetadata` extended with optional `understanding` field
- [ ] All structs derive `Serialize`, `Deserialize` for JSON-RPC
- [ ] Unit tests validate structure creation and serialization
- [ ] Type sync comments added linking to future TypeScript types
- [ ] All tests passing

## Technical Requirements

### Extend `crates/maproom/src/search/results.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryUnderstanding {
    pub mode: SearchMode,
    pub tokens: Vec<String>,
    pub expanded_terms: Vec<String>,
    pub filters: QueryFilters,
    pub fusion_strategy: String,
    pub timing: TimingBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilters {
    pub repo_id: i64,
    pub worktree_id: Option<i64>,
    pub file_types: Vec<String>,
    pub recency_threshold: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingBreakdown {
    pub query_processing_ms: f64,
    pub search_execution_ms: f64,
    pub score_fusion_ms: f64,
    pub result_assembly_ms: f64,
    pub total_ms: f64,
}

// Extend existing SearchMetadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    // ... existing fields ...

    /// Query understanding metadata (added in Phase 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub understanding: Option<QueryUnderstanding>,
}
```

### Structure Creation Helpers
```rust
impl QueryUnderstanding {
    /// Create from ProcessedQuery and timing data
    pub fn from_processed_query(
        query: &ProcessedQuery,
        filters: QueryFilters,
        timing: TimingBreakdown,
    ) -> Self {
        Self {
            mode: query.mode,
            tokens: query.tokens.clone(),
            expanded_terms: query.expanded_terms.clone(),
            filters,
            fusion_strategy: query.fusion_strategy.clone(),
            timing,
        }
    }
}

impl TimingBreakdown {
    /// Create from timing measurements
    pub fn new(
        query_processing_ms: f64,
        search_execution_ms: f64,
        score_fusion_ms: f64,
        result_assembly_ms: f64,
    ) -> Self {
        let total_ms = query_processing_ms
            + search_execution_ms
            + score_fusion_ms
            + result_assembly_ms;

        Self {
            query_processing_ms,
            search_execution_ms,
            score_fusion_ms,
            result_assembly_ms,
            total_ms,
        }
    }
}
```

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_breakdown_total_calculation() {
        let timing = TimingBreakdown::new(4.2, 35.8, 2.1, 6.4);

        assert_eq!(timing.total_ms, 48.5);
        assert_eq!(timing.query_processing_ms, 4.2);
    }

    #[test]
    fn test_query_understanding_serialization() {
        let understanding = QueryUnderstanding {
            mode: SearchMode::Auto,
            tokens: vec!["authenticate".to_string(), "user".to_string()],
            expanded_terms: vec!["auth".to_string(), "login".to_string()],
            filters: QueryFilters {
                repo_id: 1,
                worktree_id: Some(2),
                file_types: vec![],
                recency_threshold: None,
            },
            fusion_strategy: "reciprocal_rank_fusion".to_string(),
            timing: TimingBreakdown::new(4.2, 35.8, 2.1, 6.4),
        };

        let json = serde_json::to_string(&understanding).unwrap();
        assert!(json.contains("authenticate"));
        assert!(json.contains("reciprocal_rank_fusion"));
    }

    #[test]
    fn test_optional_understanding_field_serialization() {
        let metadata = SearchMetadata {
            // ... existing fields ...
            understanding: None,
        };

        let json = serde_json::to_value(&metadata).unwrap();
        // When None, field should be omitted (skip_serializing_if)
        assert!(json.get("understanding").is_none());

        let metadata_with_understanding = SearchMetadata {
            // ... existing fields ...
            understanding: Some(QueryUnderstanding {
                mode: SearchMode::Code,
                tokens: vec!["test".to_string()],
                expanded_terms: vec![],
                filters: QueryFilters {
                    repo_id: 1,
                    worktree_id: None,
                    file_types: vec![],
                    recency_threshold: None,
                },
                fusion_strategy: "linear".to_string(),
                timing: TimingBreakdown::new(1.0, 2.0, 3.0, 4.0),
            }),
        };

        let json = serde_json::to_value(&metadata_with_understanding).unwrap();
        assert!(json.get("understanding").is_some());
    }
}
```

## Implementation Notes
1. Locate `crates/maproom/src/search/results.rs` (or wherever `SearchMetadata` is defined)
2. Add new structs near `SearchMetadata` definition
3. Add `understanding: Option<QueryUnderstanding>` to `SearchMetadata`
4. Use `#[serde(skip_serializing_if = "Option::is_none")]` to omit field when None (backward compatibility)
5. Add helper constructors for ergonomic creation
6. Write unit tests for serialization and total_ms calculation

**Existing Data Sources**:
- `mode`, `tokens`, `expanded_terms`, `fusion_strategy` → Already in `ProcessedQuery`
- Timing data → Already tracked in search pipeline
- Filters → Already in `SearchOptions`

**No New Computation**: All fields copy from existing in-memory data.

## Dependencies
**Phase 1 Complete**: Phase 1 establishes type sync patterns that Phase 2 follows

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Breaking existing serialization
- Adding performance overhead

**Mitigations**:
- Optional field with `skip_serializing_if` (backward compatible)
- No new computation (just data assembly)
- Unit tests validate serialization

## Files/Packages Affected
- **Modified**: `crates/maproom/src/search/results.rs` (~60 lines added)
- **Reference**: `crates/maproom/src/search/pipeline.rs` (ProcessedQuery definition)

## Estimated Effort
3-4 hours

**Breakdown**:
- Struct definitions: 1 hour
- Helper constructors: 1 hour
- Unit tests: 1-2 hours

## Planning References
- [plan.md](../planning/plan.md) - Phase 2 ticket breakdown
- [architecture.md](../planning/architecture.md) - Query understanding metadata design
- [quality-strategy.md](../planning/quality-strategy.md) - Unit testing approach
