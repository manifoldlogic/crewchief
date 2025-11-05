# Ticket: MRPROG-1005: Add performance benchmarks for scan progress tracking

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create performance benchmarks using Criterion to measure the overhead of progress tracking. Verify that adding progress updates to scan operations causes <5% performance degradation.

## Background
Progress tracking adds atomic counter updates and periodic output operations to the indexing loop. We need to verify this overhead is acceptable (<5%) to ensure the UX improvement doesn't significantly slow down scan operations.

This is a pragmatic benchmark focused on real-world impact, not micro-optimizations. We're measuring actual scan time with/without progress on a realistic test repository.

This ticket implements the Performance Tests section from the quality-strategy.md planning document, validating that progress tracking maintains acceptable performance characteristics.

## Acceptance Criteria
- [ ] Benchmark file created: `crates/maproom/benches/progress_overhead.rs`
- [ ] Benchmark for scan without progress (baseline)
- [ ] Benchmark for scan with progress (ProgressTracker enabled)
- [ ] Test fixture: small test repository (~50-100 files)
- [ ] Benchmarks run successfully: `cargo bench`
- [ ] Overhead measured and documented: <5% slowdown
- [ ] Results documented in benchmark output

## Technical Requirements

### Benchmark Setup

1. **Add Criterion dependency to Cargo.toml:**
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "progress_overhead"
harness = false
```

2. **Create benchmark file:**
```rust
// crates/maproom/benches/progress_overhead.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use maproom::indexer;
use maproom::progress::{ProgressTracker, OutputMode};
use std::path::PathBuf;

fn bench_scan_without_progress(c: &mut Criterion) {
    let test_repo = setup_test_repo(); // Helper to create/use test fixture

    c.bench_function("scan_no_progress", |b| {
        b.iter(|| {
            // Run scan_worktree with progress=None
            scan_worktree(
                black_box(&pool),
                black_box(&test_repo),
                black_box("test-repo"),
                black_box("main"),
                black_box("HEAD"),
                None, None, false, 4,
                None,  // No progress
            )
        });
    });
}

fn bench_scan_with_progress(c: &mut Criterion) {
    let test_repo = setup_test_repo();

    c.bench_function("scan_with_progress", |b| {
        let progress = ProgressTracker::new(OutputMode::Minimal);

        b.iter(|| {
            scan_worktree(
                black_box(&pool),
                black_box(&test_repo),
                black_box("test-repo"),
                black_box("main"),
                black_box("HEAD"),
                None, None, false, 4,
                Some(&progress),  // With progress
            )
        });
    });
}

criterion_group!(benches, bench_scan_without_progress, bench_scan_with_progress);
criterion_main!(benches);
```

### Test Fixture
- Create small test repository in `crates/maproom/benches/fixtures/test-repo/`
- ~50-100 code files (TypeScript, Rust, Python mix)
- Commit to git: `git init && git add . && git commit -m "test"`

### Success Criteria
- Measure baseline (no progress)
- Measure with progress
- Calculate overhead: `(with_progress - baseline) / baseline * 100%`
- Verify overhead < 5%

## Implementation Notes

1. Create benches directory if it doesn't exist: `crates/maproom/benches/`
2. Add dev-dependency for criterion in Cargo.toml
3. Create progress_overhead.rs benchmark file with two benchmark functions
4. Create test fixture repository with representative code files
5. Run benchmarks: `cargo bench`
6. Review HTML report in `target/criterion/`
7. Document results showing <5% overhead

**Note**: The benchmark is pragmatic and measures real-world impact. We're not micro-optimizing but validating that the UX enhancement doesn't significantly degrade performance.

**Expected Results:**
```
scan_no_progress       time:   [450.12 ms 455.23 ms 460.89 ms]
scan_with_progress     time:   [460.34 ms 465.87 ms 471.22 ms]

Overhead: ~2.3% (well under 5% target)
```

## Dependencies
- **BLOCKED BY**: MRPROG-1002 (needs scan_worktree integration)
- **BLOCKED BY**: MRPROG-1003 (needs CLI wiring for realistic test)

## Risk Assessment
- **Risk**: Benchmark might be noisy on different machines
  - **Mitigation**: Run multiple iterations (Criterion default), focus on relative overhead not absolute time
- **Risk**: Test fixture size affects results
  - **Mitigation**: Use ~50-100 files (realistic small repo), document fixture size in results

## Files/Packages Affected
- CREATE: `crates/maproom/benches/progress_overhead.rs`
- CREATE: `crates/maproom/benches/fixtures/test-repo/` (test data)
- MODIFY: `crates/maproom/Cargo.toml` (add criterion dev-dependency, bench config)

## References
- Quality strategy: `.agents/projects/MRPROG_maproom-progress-ux/planning/quality-strategy.md` (Performance Tests section)
- Architecture: `.agents/projects/MRPROG_maproom-progress-ux/planning/architecture.md` (Performance Considerations)

## Estimated Effort
2-3 hours
