# Ticket: MRPROG-1004: Write ProgressTracker unit tests

## Status
- [x] **Task completed** - acceptance criteria met (tests created in MRPROG-1001)
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Note
Tests were created as part of MRPROG-1001 module implementation. All 11 tests exist in progress.rs lines 228-374 and pass. This ticket is marked complete as the work was delivered earlier.

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Write comprehensive unit tests for the ProgressTracker module to verify percentage calculations, throttling logic, concurrent updates, and TTY/non-TTY formatting. Target 80%+ coverage of ProgressTracker code.

## Background
The ProgressTracker module handles mathematical calculations (percentages, ETAs) and state management (atomic counters, mutex for throttling). Unit tests ensure correctness across edge cases like zero files, division by zero, concurrent access, and timing accuracy.

This is pragmatic testing focused on correctness, not exhaustive coverage. We're testing the critical math and concurrency logic, not cosmetic string formatting. This ticket is part of Phase 1 (Progress Tracking Foundation) of the MRPROG project and implements the Unit Tests section of the quality strategy.

**Reference**: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/quality-strategy.md` (Unit Tests section)

## Acceptance Criteria
- [ ] Test file created: `crates/maproom/src/progress.rs` (in #[cfg(test)] module)
- [ ] Tests for percentage calculations (0 files, 1 file, 100 files, 10,000 files)
- [ ] Tests for zero-division safety (total=0, processed>total)
- [ ] Tests for throttling logic (immediate second call returns false, >200ms returns true)
- [ ] Tests for concurrent updates (10 threads updating concurrently, verify final count)
- [ ] Tests for both OutputMode variants (Minimal, Verbose)
- [ ] All tests pass: `cargo test --lib progress`
- [ ] Code coverage >80% for ProgressTracker module

## Technical Requirements

**Test Module Structure:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_percentage_calculation() { }

    #[test]
    fn test_percentage_calculation_edge_cases() { }

    #[test]
    fn test_zero_total_safe() { }

    #[test]
    fn test_throttling() { }

    #[test]
    fn test_throttling_timing() { }

    #[test]
    fn test_concurrent_updates() { }

    #[test]
    fn test_output_mode_minimal() { }

    #[test]
    fn test_output_mode_verbose() { }

    #[test]
    fn test_set_totals_updates() { }
}
```

**Key Test Cases:**

1. **Percentage Calculation:**
```rust
#[test]
fn test_percentage_calculation() {
    let tracker = ProgressTracker::new(OutputMode::Minimal);
    tracker.set_totals(100, None);
    tracker.update_files(50);
    assert_eq!(tracker.percentage_files(), 50);

    tracker.update_files(75);
    assert_eq!(tracker.percentage_files(), 75);
}
```

2. **Zero Division Safety:**
```rust
#[test]
fn test_zero_total_safe() {
    let tracker = ProgressTracker::new(OutputMode::Minimal);
    tracker.set_totals(0, None);
    // Should not panic
    assert_eq!(tracker.percentage_files(), 0);
}
```

3. **Throttling:**
```rust
#[test]
fn test_throttling() {
    let tracker = ProgressTracker::new(OutputMode::Minimal);
    assert!(tracker.should_print()); // First call always true
    assert!(!tracker.should_print()); // Immediate second call false

    std::thread::sleep(Duration::from_millis(250));
    assert!(tracker.should_print()); // After 250ms, true again
}
```

4. **Concurrent Updates:**
```rust
#[test]
fn test_concurrent_updates() {
    let tracker = Arc::new(ProgressTracker::new(OutputMode::Minimal));
    tracker.set_totals(1000, None);

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let t = Arc::clone(&tracker);
            thread::spawn(move || {
                for _ in 0..100 {
                    t.update_files(1);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(tracker.processed_files.load(Ordering::Relaxed), 1000);
}
```

## Implementation Notes

1. Add tests inside `crates/maproom/src/progress.rs` at the bottom
2. Use `#[cfg(test)]` to only compile during testing
3. Test both files and chunks tracking
4. Use `std::thread::sleep` for timing tests (acceptable for unit tests)
5. Use `Arc` for testing concurrent access
6. Don't test exact output strings (too brittle), test behavior

**Testing Approach:**
- Focus on mathematical correctness (percentage calculations)
- Verify concurrency safety (atomic operations work correctly)
- Test edge cases (zero totals, overflow scenarios)
- Validate throttling timing logic
- Test both OutputMode::Minimal and OutputMode::Verbose behavior

**Coverage Strategy:**
Run tests with: `cargo test --lib progress`

Use `cargo tarpaulin` or `cargo llvm-cov` to verify >80% coverage:
```bash
cargo tarpaulin --lib --packages maproom --exclude-files '**/tests/*' --out Stdout
```

## Dependencies
- **BLOCKED BY**: MRPROG-1001 (needs ProgressTracker module to test)
- **BLOCKS**: None - this is a testing ticket that validates implementation

## Risk Assessment
- **Risk**: Timing tests might be flaky on slow CI
  - **Mitigation**: Use generous margins (250ms instead of 201ms) to account for CI variability

- **Risk**: Coverage tools might not be available in all environments
  - **Mitigation**: Coverage verification is optional; passing tests are mandatory

- **Risk**: Concurrent test might occasionally fail due to scheduler timing
  - **Mitigation**: Use high iteration counts (10 threads × 100 updates = 1000 total) to make race conditions statistically unlikely

## Files/Packages Affected
- **MODIFY**: `crates/maproom/src/progress.rs` (add #[cfg(test)] mod tests section at end)

## Estimated Effort
2-3 hours

## References
- Quality strategy: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/quality-strategy.md` (Unit Tests section)
- Architecture: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/architecture.md`
- Phase 1 Plan: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md` (Testing section)
