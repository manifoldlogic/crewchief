# Quality Strategy: Maproom Progress UX Enhancement

## Testing Philosophy

**Core principle**: Test the contract, not the implementation.

This is a UX enhancement focused on output formatting. Our tests must verify:
1. **Correct information** is displayed (right counts, percentages, timing)
2. **Appropriate format** is used (minimal vs. verbose)
3. **Performance overhead** is acceptable (<5% slowdown)

We don't need exhaustive unit tests for string formatting. We need **confidence that the UX works** and **won't break existing functionality**.

## Risk Assessment

### High Risk Areas

**HR1: Progress updates slow down indexing**
- **Risk**: Frequent progress prints could add significant overhead
- **Mitigation**: Throttle updates to 200ms minimum, measure performance impact
- **Test**: Benchmark scan with/without progress on large codebase

**HR2: TTY detection fails in edge cases**
- **Risk**: Line overwriting doesn't work in certain terminals/environments
- **Mitigation**: Fallback to non-TTY mode on detection failure
- **Test**: Manual testing in common terminals, automated test in non-TTY

**HR3: Watch mode floods output in pathological cases**
- **Risk**: Mass file changes (e.g., git checkout) create excessive output
- **Mitigation**: Minimal mode already limits to 3 lines per event + debouncing
- **Test**: Trigger watch with 100+ file changes, verify output is reasonable

### Medium Risk Areas

**MR1: Progress percentages calculate incorrectly**
- **Risk**: Division by zero, integer overflow, rounding errors
- **Mitigation**: Unit tests for edge cases (0 files, 1 file, large numbers)
- **Test**: Unit test ProgressTracker calculations

**MR2: Timing measurements are inaccurate**
- **Risk**: Instant::now() overhead, formatting precision issues
- **Mitigation**: Use standard library timing (proven accurate), format to 1 decimal
- **Test**: Verify timing is within 50ms of expected

**MR3: OutputMode flag parsing fails**
- **Risk**: --verbose flag not recognized, wrong mode selected
- **Mitigation**: Use clap's tested parsing, add integration test
- **Test**: Integration test with --verbose flag

### Low Risk Areas

**LR1: Unicode emoji display issues**
- **Risk**: Some terminals don't support emoji
- **Mitigation**: Already using emoji in existing code; no new risk
- **Test**: None (existing behavior)

**LR2: Help text unclear**
- **Risk**: Users don't understand default directory behavior
- **Mitigation**: Clear help text, examples in docs
- **Test**: Manual review of help output

## Test Coverage Strategy

### What to Test

**Tier 1: Critical (Must Have)**
1. Progress calculations are mathematically correct
2. Output format matches expected (minimal vs. verbose)
3. Performance overhead is acceptable
4. TTY vs. non-TTY modes work correctly
5. Existing functionality unchanged (regression)

**Tier 2: Important (Should Have)**
1. Edge cases (0 files, 1 file, 10,000+ files)
2. Concurrent progress updates (parallel scan)
3. Error handling (TTY detection failure, stdout write failure)
4. Watch debouncing with minimal output

**Tier 3: Nice to Have (Could Have)**
1. ETA accuracy validation
2. Terminal width edge cases
3. Unicode emoji rendering

### What NOT to Test

**Don't test**:
- Exact string formatting (too brittle, low value)
- Specific emoji choices (cosmetic)
- stdout/stderr mechanics (trust standard library)
- CLI parsing details (trust clap crate)

**Rationale**: Focus on behavior and correctness, not formatting details.

## Test Implementation Plan

### 1. Unit Tests

**File**: `crates/maproom/src/progress.rs`

**Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_calculation() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);
        tracker.set_totals(100, None);
        tracker.update_files(50);
        assert_eq!(tracker.percentage_files(), 50);
    }

    #[test]
    fn test_zero_total_safe() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);
        tracker.set_totals(0, None);
        // Should not panic
        assert_eq!(tracker.percentage_files(), 0);
    }

    #[test]
    fn test_throttling() {
        let tracker = ProgressTracker::new(OutputMode::Minimal);
        assert!(tracker.should_print()); // First call always true
        assert!(!tracker.should_print()); // Immediate second call false
        std::thread::sleep(Duration::from_millis(250));
        assert!(tracker.should_print()); // After 250ms, true again
    }

    #[test]
    fn test_concurrent_updates() {
        let tracker = Arc::new(ProgressTracker::new(OutputMode::Minimal));
        tracker.set_totals(1000, None);

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let t = Arc::clone(&tracker);
                std::thread::spawn(move || {
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
}
```

**Coverage target**: 80%+ of ProgressTracker logic

### 2. Integration Tests

**File**: `crates/maproom/tests/progress_integration.rs`

**Tests**:
```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_scan_shows_progress() {
    let output = Command::cargo_bin("maproom")
        .unwrap()
        .arg("scan")
        .arg("--path")
        .arg("test-fixtures/small-repo")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Verify progress indicators appear
    assert!(stdout.contains("Processing:"));
    assert!(stdout.contains("files"));
    assert!(stdout.contains("Completed in"));
}

#[test]
fn test_scan_verbose_flag() {
    let output = Command::cargo_bin("maproom")
        .unwrap()
        .arg("scan")
        .arg("--path")
        .arg("test-fixtures/small-repo")
        .arg("--verbose")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Verbose mode should still show progress
    assert!(stdout.contains("Processing:"));
    assert!(stdout.contains("Completed in"));
}

#[test]
fn test_watch_minimal_output() {
    // Create test repo with watcher
    let test_repo = setup_test_repo();

    let mut watch_cmd = Command::cargo_bin("maproom")
        .unwrap()
        .arg("watch")
        .arg("--path")
        .arg(test_repo.path())
        .spawn()
        .unwrap();

    // Wait for watcher to start
    std::thread::sleep(Duration::from_secs(1));

    // Modify a file
    std::fs::write(test_repo.path().join("test.txt"), "changed").unwrap();

    // Wait for re-index
    std::thread::sleep(Duration::from_secs(2));

    // Kill watcher
    watch_cmd.kill().unwrap();

    let output = watch_cmd.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify minimal output
    assert!(stdout.contains("file(s) changed"));
    assert!(stdout.contains("Indexing:"));
    assert!(stdout.contains("Done in"));

    // Verify not verbose
    assert!(!stdout.contains("Re-indexing..."));
}

#[test]
fn test_watch_verbose_output() {
    let test_repo = setup_test_repo();

    let mut watch_cmd = Command::cargo_bin("maproom")
        .unwrap()
        .arg("watch")
        .arg("--path")
        .arg(test_repo.path())
        .arg("--verbose")
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_secs(1));
    std::fs::write(test_repo.path().join("test.txt"), "changed").unwrap();
    std::thread::sleep(Duration::from_secs(2));
    watch_cmd.kill().unwrap();

    let output = watch_cmd.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify verbose output
    assert!(stdout.contains("Detected changes"));
    assert!(stdout.contains("Re-indexing"));
    assert!(stdout.contains("Index updated"));
}
```

**Coverage target**: Core user workflows validated

### 3. Performance Tests

**File**: `crates/maproom/benches/progress_overhead.rs`

**Tests**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_scan_without_progress(c: &mut Criterion) {
    c.bench_function("scan_no_progress", |b| {
        b.iter(|| {
            scan_worktree(
                black_box(&test_repo),
                black_box(&config),
                None, // No progress
            )
        });
    });
}

fn bench_scan_with_progress(c: &mut Criterion) {
    c.bench_function("scan_with_progress", |b| {
        let progress = ProgressTracker::new(OutputMode::Minimal);
        b.iter(|| {
            scan_worktree(
                black_box(&test_repo),
                black_box(&config),
                Some(&progress),
            )
        });
    });
}

criterion_group!(benches, bench_scan_without_progress, bench_scan_with_progress);
criterion_main!(benches);
```

**Success criteria**: <5% overhead with progress tracking enabled

### 4. Manual Testing Checklist

**Test Matrix**:

| Environment | Scan | Watch | Watch Verbose |
|------------|------|-------|---------------|
| iTerm2 (macOS) | ✓ | ✓ | ✓ |
| Terminal.app (macOS) | ✓ | ✓ | ✓ |
| Windows Terminal | ✓ | ✓ | ✓ |
| WSL2 | ✓ | ✓ | ✓ |
| tmux | ✓ | ✓ | ✓ |
| screen | ✓ | ✓ | ✓ |
| VS Code terminal | ✓ | ✓ | ✓ |
| Non-TTY (log file) | ✓ | ✓ | ✓ |

**Per-environment checks**:
- Progress updates appear smoothly
- Line overwriting works (TTY) or periodic updates (non-TTY)
- Emoji render correctly
- Timing information is accurate
- No output corruption or garbling

### 5. Regression Testing

**Existing test suite must pass**:
```bash
cd /workspace
pnpm test
```

**All existing integration tests must pass**:
```bash
cd /workspace/crates/maproom
cargo test
```

**Success criteria**: 100% of existing tests pass (zero regressions)

## Quality Gates

### Pre-merge Requirements

**Gate 1: Unit tests pass**
```bash
cargo test --lib
```

**Gate 2: Integration tests pass**
```bash
cargo test --test '*'
```

**Gate 3: Performance acceptable**
```bash
cargo bench
# Verify <5% overhead
```

**Gate 4: Manual testing completed**
- At least 3 different terminals tested
- Scan and watch both verified
- No visual artifacts or corruption

**Gate 5: Code review**
- Logic is clear and maintainable
- Error handling is appropriate
- No unsafe code introduced

### Post-merge Monitoring

**Monitor for**:
- User reports of progress issues
- Performance degradation in CI
- Terminal compatibility problems

**Rollback trigger**: If progress tracking causes >10% slowdown or breaks in common terminals

## Testing Pragmatism

### What We're NOT Doing

**Not exhaustively testing**:
- Every possible terminal emulator (hundreds exist)
- Every edge case of terminal width, color support, locale
- Precise ETA accuracy (estimate is good enough)
- String formatting details (cosmetic)

**Rationale**: Diminishing returns. Core functionality coverage is more valuable than edge case perfection.

### What We ARE Doing

**Focused testing**:
- Mathematical correctness (percentages, timing)
- Core user workflows (scan, watch, verbose flag)
- Performance impact (benchmark)
- Regression prevention (existing tests)
- Common terminal compatibility (manual spot-check)

**Rationale**: High value, high confidence, reasonable effort.

## Test Data

### Test Fixtures Needed

**Small repo**: 10 files, mix of languages
- Use for fast integration tests
- Located: `crates/maproom/tests/fixtures/small-repo/`

**Medium repo**: 100 files, ~500 chunks
- Use for performance baseline
- Located: `crates/maproom/tests/fixtures/medium-repo/`

**Large repo** (optional): 1000+ files
- Use for stress testing (if needed)
- Clone existing OSS repo (e.g., small Rust project)

### Creating Test Repos

```bash
#!/bin/bash
# tests/fixtures/setup.sh

# Small repo
mkdir -p small-repo/src
echo "fn main() {}" > small-repo/src/main.rs
echo "# Test" > small-repo/README.md
cd small-repo && git init && git add . && git commit -m "init"

# Medium repo
# ... generate 100 files programmatically ...
```

## Confidence Level Assessment

**After implementing this strategy, we will have**:

- **High confidence**: Progress calculations are correct
- **High confidence**: Output formats work in common scenarios
- **High confidence**: No performance regressions
- **High confidence**: Existing functionality intact
- **Medium confidence**: Works in all terminal environments
- **Medium confidence**: ETA estimates are reasonable
- **Low confidence**: Perfect in every edge case (acceptable trade-off)

## Testing Timeline

**Phase 1: Development** (while implementing)
- Write unit tests alongside ProgressTracker
- Run existing test suite frequently

**Phase 2: Integration** (after core implementation)
- Write integration tests for scan/watch
- Set up performance benchmarks

**Phase 3: Manual** (before PR)
- Test in 3-5 different terminals
- Verify TTY and non-TTY behavior
- Check output aesthetics

**Phase 4: Review** (during PR)
- Code review focused on correctness
- Verify test coverage is adequate
- Run full test suite in CI

**Total testing time**: ~20% of development time (pragmatic ratio)

## Success Metrics

**Test suite success criteria**:
- Unit tests: >80% coverage of ProgressTracker
- Integration tests: All core workflows covered
- Performance tests: <5% overhead confirmed
- Manual tests: Works in 3+ terminals
- Regression tests: 100% pass rate

**User acceptance criteria**:
- Users report progress is visible and helpful
- No complaints about performance degradation
- No reports of garbled output in common terminals

## Summary

This is a **pragmatic testing strategy** for a **UX enhancement**. We're not building life-critical software; we're making a developer tool more pleasant to use.

**Key insight**: Test what matters (correctness, performance, compatibility), not what doesn't (exact formatting, exotic terminals, aesthetic perfection).

**Philosophy**: Confidence without ceremony. Tests should prevent real problems, not check boxes.
