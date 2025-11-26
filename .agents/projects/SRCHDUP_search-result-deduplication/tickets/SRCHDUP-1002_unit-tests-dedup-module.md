# Ticket: SRCHDUP-1002: Unit tests for dedup module

## Status
- [x] **Task completed** - acceptance criteria met (tests included in SRCHDUP-1001)
- [x] **Tests pass** - tests executed and passing (9 tests pass)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create comprehensive unit tests for the deduplication module to verify identity key generation, duplicate detection, score-based selection, and edge case handling. Tests should be embedded in `dedup.rs` using `#[cfg(test)]` module.

## Background

The deduplication module is critical path functionality that affects all search results. Thorough unit testing ensures correctness before pipeline integration. Tests should cover the scenarios outlined in `quality-strategy.md` Level 1 testing.

**Reference:** plan.md Phase 1, quality-strategy.md "Level 1: Unit Tests"

## Acceptance Criteria

- [ ] `#[cfg(test)] mod tests` block in `dedup.rs`
- [ ] Test helper function `make_chunk_result()` for creating test data
- [ ] Test helper function `make_duplicates()` for creating duplicate sets
- [ ] At least 9 unit tests covering all specified scenarios
- [ ] All tests pass: `cargo test --lib search::dedup`
- [ ] Tests complete in <100ms

## Technical Requirements

### Test Helper Functions

```rust
fn make_chunk_result(
    chunk_id: i64,
    relpath: &str,
    symbol_name: Option<&str>,
    start_line: i32,
    score: f64,
) -> ChunkSearchResult

fn make_duplicates(
    count: usize,
    relpath: &str,
    symbol: &str,
    line: i32,
) -> Vec<ChunkSearchResult>
```

### Required Test Cases

1. **test_identity_key_generation**
   - Same (relpath, symbol, line) → same identity
   - Different relpath → different identity
   - Different symbol_name → different identity
   - Different start_line → different identity

2. **test_deduplicate_empty_results**
   - Empty input returns empty output

3. **test_deduplicate_no_duplicates**
   - All unique results → unchanged count

4. **test_deduplicate_all_duplicates**
   - All same identity → returns exactly 1 result

5. **test_deduplicate_mixed**
   - Mix of duplicates and unique → correct count

6. **test_deduplicate_preserves_order**
   - Results sorted by score descending after dedup

7. **test_highest_score_selection**
   - Among duplicates, highest score is selected
   - Verify selected chunk_id matches expected

8. **test_disabled_config**
   - `enabled: false` → returns all results unchanged

9. **test_null_symbol_name_handling**
   - `symbol_name: None` treated as empty string
   - Two chunks with None symbol same file/line are duplicates

## Implementation Notes

1. **Test isolation** - Each test should create its own test data
2. **Deterministic scores** - Use explicit scores, not random
3. **Meaningful assertions** - Check both count and content
4. **Test naming** - Use descriptive names following Rust conventions

### Example Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk_result(...) -> ChunkSearchResult { ... }

    #[test]
    fn test_identity_key_generation() {
        let chunk1 = make_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9);
        let chunk2 = make_chunk_result(2, "src/auth.rs", Some("validate"), 10, 0.8);

        let id1 = ChunkIdentity::from_result(&chunk1);
        let id2 = ChunkIdentity::from_result(&chunk2);

        assert_eq!(id1, id2, "Same file/symbol/line should have same identity");
    }
}
```

## Dependencies

- SRCHDUP-1001 (dedup module must exist)

## Risk Assessment

- **Risk**: ChunkSearchResult fields missing for test construction
  - **Mitigation**: Check actual struct definition, use minimal required fields
- **Risk**: Tests may be flaky due to floating-point comparison
  - **Mitigation**: Use exact scores in tests, avoid floating-point edge cases

## Files/Packages Affected

- `crates/maproom/src/search/dedup.rs` (add test module)
