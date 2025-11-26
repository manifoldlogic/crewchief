# Ticket: SRCHDUP-1001: Create dedup.rs module with ChunkIdentity and deduplicate()

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create a new deduplication module for the search pipeline that eliminates duplicate search results across worktrees. Implement the `ChunkIdentity` struct for grouping results by their logical identity and the `deduplicate()` function that selects the highest-scoring representative from each group.

## Background

When code is indexed across multiple worktrees (main, feature branches, stale snapshots), the same logical code chunk appears multiple times in search results. A search for "validate_provider" might return 15 identical results from 15 different worktrees, burying unique findings in noise.

This ticket implements the core deduplication logic as specified in `architecture.md` Section 2 (Deduplication Module). The module will be integrated into the search pipeline in subsequent tickets.

**Reference:** plan.md Phase 1, architecture.md Sections 1-2

## Acceptance Criteria

- [ ] New file `crates/maproom/src/search/dedup.rs` exists
- [ ] `ChunkIdentity` struct derives `Hash`, `Eq`, `PartialEq` for HashMap grouping
- [ ] `ChunkIdentity::from_result()` extracts identity from `ChunkSearchResult`
- [ ] `DeduplicationConfig` struct with `enabled` and `strategy` fields
- [ ] `SelectionStrategy` enum with `HighestScore` variant (only MVP variant)
- [ ] `deduplicate()` function groups by identity and returns unique results
- [ ] Module exported via `mod.rs`: `pub mod dedup;`

## Technical Requirements

### ChunkIdentity Struct
```rust
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ChunkIdentity {
    pub relpath: String,
    pub symbol_name: String,  // Empty string if None
    pub start_line: i32,
}
```

### DeduplicationConfig
```rust
#[derive(Debug, Clone)]
pub struct DeduplicationConfig {
    pub enabled: bool,
    pub strategy: SelectionStrategy,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: SelectionStrategy::HighestScore,
        }
    }
}
```

### SelectionStrategy
```rust
#[derive(Debug, Clone, Copy, Default)]
pub enum SelectionStrategy {
    #[default]
    HighestScore,
    // Future: PreferMain (requires worktree_name in ChunkSearchResult)
}
```

### deduplicate() Function
- Accept `Vec<ChunkSearchResult>` and `&DeduplicationConfig`
- Return `Vec<ChunkSearchResult>` (deduplicated)
- Use `HashMap<ChunkIdentity, Vec<ChunkSearchResult>>` for grouping
- Select highest-scoring chunk from each group
- Re-sort final results by score descending
- Return unchanged if `config.enabled == false` or input is empty

## Implementation Notes

1. **Import ChunkSearchResult** from `crate::search::results`
2. **Handle None symbol_name** by converting to empty string in identity
3. **Score comparison** should use `partial_cmp` for f64 ordering
4. **Memory efficiency** - the algorithm is O(n) space, acceptable for typical result sets
5. **No external dependencies** - uses only std::collections::HashMap

### Edge Cases
- Empty input returns empty output
- Single result returns unchanged
- All results have same identity → returns one result
- symbol_name is None → treated as empty string

## Dependencies

- None (this is the first ticket in the project)

## Risk Assessment

- **Risk**: ChunkSearchResult structure may have changed
  - **Mitigation**: Verify field names match current `results.rs` before implementation
- **Risk**: HashMap key collision on identity
  - **Mitigation**: Using composite key (relpath, symbol, line) provides sufficient uniqueness

## Files/Packages Affected

- `crates/maproom/src/search/dedup.rs` (NEW)
- `crates/maproom/src/search/mod.rs` (add `pub mod dedup;`)
