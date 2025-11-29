# Ticket: MRPROG-3003: Performance validation on realistic codebase

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
Run performance benchmarks on a realistic large codebase to validate that the <5% overhead claim holds in real-world usage. Document results in a performance report that can be referenced in documentation.

## Background
MRPROG-1005 created benchmarks using a small test repository. This ticket validates performance on a real, large codebase (1000+ files) to ensure the overhead claim is accurate for production use.

This is pragmatic performance validation: verify the feature works efficiently at scale, document results, not endless micro-optimization.

**Reference:** Phase 3 (Polish & Documentation), Task 5 from `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md`

## Acceptance Criteria
- [ ] Identified realistic test codebase (1000+ files, mix of languages)
- [ ] Ran baseline scan (without progress): recorded time
- [ ] Ran scan with progress: recorded time
- [ ] Calculated overhead percentage: `(with_progress - baseline) / baseline * 100%`
- [ ] Verified overhead <5%
- [ ] Performance report created with results
- [ ] Report includes: test repo details, machine specs, baseline time, progress time, overhead %
- [ ] Any issues or optimizations documented

## Technical Requirements

### Test Codebase Selection

**Option 1: Use existing large OSS repo**
- Clone popular Rust/TypeScript repo (>1000 files)
- Examples: tokio, rust-lang/rust (subset), vscode (subset), next.js

**Option 2: Use CrewChief itself**
- CrewChief codebase with multiple packages
- Realistic mix of TS and Rust

**Option 3: Create synthetic large repo**
- Generate 1000+ files programmatically
- Mix of languages (TS, Rust, Python)

### Benchmarking Procedure

```bash
# 1. Setup test repository
cd /path/to/large/repo
git status  # Verify it's a git repo

# 2. Run baseline (no progress)
# Temporarily modify code to pass None for progress
time maproom scan > /dev/null 2>&1
# Record: Baseline time = X seconds

# 3. Run with progress (default behavior)
time maproom scan > /dev/null 2>&1
# Record: Progress time = Y seconds

# 4. Calculate overhead
# Overhead = (Y - X) / X * 100%

# 5. Run multiple iterations for accuracy
for i in {1..5}; do
    time maproom scan > /dev/null 2>&1
done
# Average the results
```

### Performance Report Format

Create: `.crewchief/projects/MRPROG_maproom-progress-ux/testing/performance-validation-report.md`

```markdown
# Performance Validation Report

## Test Environment

**Date:** YYYY-MM-DD
**Machine:** MacBook Pro M1, 16GB RAM (or your specs)
**OS:** macOS 14.0 / Ubuntu 22.04
**Rust:** 1.75.0
**Database:** PostgreSQL 15

## Test Repository

**Repository:** tokio/tokio (or your choice)
**Total files:** 1,247
**Total size:** 15.2 MB
**Languages:** Rust (95%), TOML (3%), Markdown (2%)
**Commit:** abc123de

## Benchmark Results

### Baseline (No Progress)

| Run | Time (seconds) |
|-----|----------------|
| 1   | 45.2           |
| 2   | 44.8           |
| 3   | 45.5           |
| 4   | 44.9           |
| 5   | 45.1           |
| **Average** | **45.1** |

### With Progress

| Run | Time (seconds) |
|-----|----------------|
| 1   | 46.3           |
| 2   | 46.1           |
| 3   | 46.5           |
| 4   | 46.2           |
| 5   | 46.4           |
| **Average** | **46.3** |

### Overhead Analysis

- **Baseline:** 45.1 seconds
- **With Progress:** 46.3 seconds
- **Overhead:** 1.2 seconds
- **Percentage:** **2.7%** ✅ (well under 5% target)

## Observations

- Progress updates appeared every ~300ms
- TTY mode worked correctly (line overwriting)
- Final summary accurate
- No visible performance impact during normal use
- Memory usage stable

## Conclusion

✅ **Performance target met:** Progress tracking adds 2.7% overhead, well under the 5% target. The UX improvement justifies this minimal cost.

## Recommendations

- Ship with confidence
- Document <5% overhead claim in README
- No optimizations needed at this time
```

## Implementation Notes

1. Select or create large test repository
2. Run multiple iterations for statistical validity
3. Use `time` command for accurate measurement
4. Redirect output to /dev/null to isolate scan performance
5. Document machine specs and test conditions
6. Create performance report with results
7. If overhead >5%, investigate and optimize

**Testing Directory:**
- Ensure `.crewchief/projects/MRPROG_maproom-progress-ux/testing/` directory exists
- Create `performance-validation-report.md` with benchmark results

## Dependencies
- **BLOCKED BY**: MRPROG-1005 (benchmark infrastructure must exist)
- **BLOCKED BY**: MRPROG-1007 (Phase 1 complete and validated)

## Risk Assessment
- **Risk**: Performance might vary significantly across machines
  - **Mitigation**: Document test environment in detail
- **Risk**: Overhead might exceed 5% on slow machines
  - **Mitigation**: Throttling can be adjusted if needed
- **Risk**: Different repos might show different overhead
  - **Mitigation**: Test on realistic repo similar to actual usage

## Files/Packages Affected
- CREATE: `.crewchief/projects/MRPROG_maproom-progress-ux/testing/performance-validation-report.md`
- MODIFY (temporarily): `crates/maproom/src/scan.rs` (to create baseline without progress)
