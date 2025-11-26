# Ticket: SRCHDUP-2001: Extend SearchOptions with deduplicate flag

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (123 search tests pass)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Add a `deduplicate` boolean field to `SearchOptions` struct that controls whether search results are deduplicated. Default should be `true` (enabled) so users benefit immediately. Provide a builder method `without_dedup()` for opting out.

## Background

The `SearchOptions` struct is the configuration passed through the search pipeline. Adding the `deduplicate` flag here allows deduplication to be controlled at the API level without breaking existing callers (new field with default value).

**Reference:** plan.md Phase 2, architecture.md Section 3 "SearchOptions Extension"

## Acceptance Criteria

- [x] `SearchOptions` has `pub deduplicate: bool` field
- [x] `SearchOptions::new()` sets `deduplicate: true` by default
- [x] `SearchOptions::without_dedup()` builder method returns self with `deduplicate: false`
- [x] `SearchOptions::with_deduplicate(bool)` builder method for explicit control
- [x] All existing tests pass (no regression)
- [x] No breaking changes to existing SearchOptions usage

## Technical Requirements

### Modified SearchOptions Struct
```rust
pub struct SearchOptions {
    pub repo_id: i64,
    pub worktree_id: Option<i64>,
    pub limit: usize,
    pub mode: SearchMode,
    pub fusion_weights: Option<FusionWeights>,
    pub skip_vector: bool,
    pub skip_graph: bool,
    pub skip_signals: bool,
    pub deduplicate: bool,  // NEW: default true
}
```

### Builder Methods
```rust
impl SearchOptions {
    pub fn new(repo_id: i64, worktree_id: Option<i64>, limit: usize) -> Self {
        Self {
            // ... existing fields ...
            deduplicate: true,  // Enable by default
        }
    }

    /// Disable deduplication
    pub fn without_dedup(mut self) -> Self {
        self.deduplicate = false;
        self
    }

    /// Set deduplication explicitly
    pub fn with_deduplicate(mut self, deduplicate: bool) -> Self {
        self.deduplicate = deduplicate;
        self
    }
}
```

### Default Implementation
If `SearchOptions` implements `Default`, ensure `deduplicate: true` is included.

## Implementation Notes

1. **Backward compatibility** - Existing code using `SearchOptions::new()` gets dedup enabled
2. **Builder pattern** - Match existing methods like `with_mode()`, `with_limit()`
3. **Field placement** - Add at end to minimize merge conflicts
4. **Documentation** - Add doc comments explaining the field's purpose

### Verification Steps
1. Run `cargo build` to ensure no compilation errors
2. Run `cargo test` to ensure existing tests pass
3. Verify no callers of `SearchOptions::new()` break

## Dependencies

- SRCHDUP-1001 (dedup module exists, but not strictly required for this change)

## Risk Assessment

- **Risk**: Other code constructs SearchOptions with struct literal syntax
  - **Mitigation**: Search for `SearchOptions {` to find all instantiation sites
- **Risk**: Serialization/deserialization affected
  - **Mitigation**: Check if `#[serde(...)]` attributes needed

## Files/Packages Affected

- `crates/maproom/src/search/results.rs` (modify SearchOptions)
