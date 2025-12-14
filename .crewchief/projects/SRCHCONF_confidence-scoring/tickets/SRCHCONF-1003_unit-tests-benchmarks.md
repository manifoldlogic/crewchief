# Ticket: [SRCHCONF-1003]: Confidence Unit Tests and Performance Benchmarks

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive unit tests for confidence computation logic and performance benchmarks to ensure <5ms overhead before integration into the search pipeline.

## Background
The confidence computation module needs rigorous testing to ensure correctness across all edge cases and to validate that performance targets are met. This ticket implements the testing strategy from quality-strategy.md for the confidence module.

Testing happens in Phase 1 (before integration) to catch issues early and validate the MVP design with 3 core signals.

## Acceptance Criteria
- [ ] Minimum 8 unit tests for compute_result_confidence() covering normal and edge cases
- [ ] All unit tests pass (`cargo test -p crewchief-maproom`)
- [ ] Performance benchmark created in `crates/maproom/benches/confidence_overhead.rs`
- [ ] Benchmark shows <5ms total overhead for 20 results
- [ ] Per-result computation verified <1ms
- [ ] Serde serialization roundtrip test passes
- [ ] 100% code coverage for confidence.rs module
- [ ] Zero clippy warnings in test code

## Technical Requirements
**Unit Test Coverage** (minimum 8 tests):

1. **compute_result_confidence - Normal Cases**:
   - Multiple sources (3-4 sources with varied scores)
   - Top result (rank 1, relative_score calculation)
   - Middle result (score gap calculation)

2. **compute_result_confidence - Edge Cases**:
   - Empty source_scores HashMap (source_count = 0)
   - Last result in list (score_gap = 0.0)
   - Single result (no next result for gap)
   - No exact match multiplier (None → is_exact_match = false)
   - Exact match multiplier present (3.0 → is_exact_match = true)

3. **Serialization**:
   - ConfidenceSignals roundtrip: Rust → JSON → Rust
   - Verify all 3 fields serialize correctly

**Performance Benchmarks**:
- Use `criterion` or stdlib `bencher` crate
- Benchmark per-result computation (<1ms target)
- Benchmark batch computation for 20 results (<5ms target)
- Test with realistic FusedResult data

## Implementation Notes
Follow testing strategy from quality-strategy.md:
- Focus on critical paths (confidence math correctness)
- Test edge cases exhaustively
- Use property-based testing if appropriate
- No ceremonial tests - every test must validate actual risk

Test file structure:
```rust
// In crates/maproom/src/search/confidence.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_multiple_sources() { ... }

    #[test]
    fn test_confidence_empty_sources() { ... }

    // ... more tests
}
```

Benchmark file:
```rust
// crates/maproom/benches/confidence_overhead.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_confidence_per_result(c: &mut Criterion) { ... }
fn bench_confidence_batch(c: &mut Criterion) { ... }
```

## Dependencies
- **Prerequisite**: SRCHCONF-1001 (confidence types and module must exist)
- **Prerequisite**: SRCHCONF-1002 (exact_match_multiplier must be available for testing)

## Risk Assessment
- **Risk**: Tests may pass but not cover critical edge cases
  - **Mitigation**: Minimum 8 tests specified, verify-ticket checks coverage. Review quality-strategy.md test cases.
- **Risk**: Performance benchmark may show >5ms overhead
  - **Mitigation**: If detected, optimize before integration. Computation is O(1), should be fast. Worst case: defer complex signals to Phase 2.

## Files/Packages Affected
- `crates/maproom/src/search/confidence.rs` - Add #[cfg(test)] mod tests
- `crates/maproom/benches/confidence_overhead.rs` - NEW benchmark file
- `crates/maproom/Cargo.toml` - Add benchmark configuration if needed

## Verification Notes
The verify-ticket agent should check:
- All 8+ unit tests pass when run via `cargo test`
- Test output shows individual test results
- Benchmark runs successfully and reports timing
- Benchmark shows <5ms for 20 results, <1ms per result
- Serialization roundtrip test confirms JSON encoding works
- Code coverage for confidence.rs is 100% (or explain gaps)
- No test flakiness (run tests 3 times to verify consistency)
