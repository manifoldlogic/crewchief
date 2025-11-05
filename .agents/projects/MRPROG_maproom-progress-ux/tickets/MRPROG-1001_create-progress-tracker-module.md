# Ticket: MRPROG-1001: Create ProgressTracker module

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
Create the core `ProgressTracker` module in `crates/maproom/src/progress.rs` that will handle progress reporting for scan operations. This module provides the foundation for real-time progress updates during indexing.

## Background
The current maproom scan command goes silent during indexing, creating anxiety for users scanning large codebases. This ticket implements the core progress tracking infrastructure that will enable real-time feedback.

The ProgressTracker module is designed as an optional dependency that can be injected into the indexer without modifying core indexing logic. It uses atomic counters for thread safety and throttling to avoid output flooding.

This ticket implements the foundational component from the MRPROG project's architecture document, specifically the ProgressTracker module described in the Component Design section.

## Acceptance Criteria
- [ ] `crates/maproom/src/progress.rs` module exists with complete implementation
- [ ] `OutputMode` enum defined with `Minimal` and `Verbose` variants
- [ ] `ProgressTracker` struct implemented with:
  - TTY detection (using `atty` crate)
  - Atomic counters for files and chunks
  - Throttling logic (200ms minimum between updates)
  - Methods: `new()`, `set_totals()`, `update_files()`, `update_chunks()`, `should_print()`, `print_progress()`, `finish()`
- [ ] Progress output formats correctly for both TTY (line overwriting with `\r`) and non-TTY (periodic updates)
- [ ] Module compiles without errors and integrates into crates/maproom/src/lib.rs

## Technical Requirements

### Module Structure
The module must implement the following API:

```rust
// crates/maproom/src/progress.rs

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub enum OutputMode {
    Minimal,
    Verbose,
}

pub struct ProgressTracker {
    mode: OutputMode,
    is_tty: bool,
    start_time: Instant,
    total_files: Option<usize>,
    total_chunks: Option<usize>,
    processed_files: AtomicUsize,
    processed_chunks: AtomicUsize,
    last_update: Mutex<Instant>,
}

impl ProgressTracker {
    pub fn new(mode: OutputMode) -> Self;
    pub fn set_totals(&self, files: usize, chunks: Option<usize>);
    pub fn update_files(&self, count: usize);
    pub fn update_chunks(&self, count: usize);
    pub fn should_print(&self) -> bool;  // Returns true if >200ms since last print
    pub fn print_progress(&self);        // Prints current progress
    pub fn finish(&self);                // Prints final timing
    fn percentage_files(&self) -> usize;
    fn percentage_chunks(&self) -> usize;
}
```

### TTY Detection
- Use `atty::is(atty::Stream::Stdout)` to detect if stdout is a TTY
- Fallback to `false` if detection fails or crate is unavailable
- Store result in `is_tty` field during construction

### Output Formats
- **TTY mode**: Use `\r` to overwrite line: `"Processing: 450/1200 files (37%) | Embeddings: 2500/6000 (42%)"`
- **Non-TTY mode**: Print new line every 10% progress increment

### Throttling Logic
- Use `Mutex<Instant>` to track last update time
- Only print if elapsed time since last update > 200ms
- Prevents output flooding during rapid updates

### Dependencies
- Add `atty = "0.2"` to `crates/maproom/Cargo.toml`

## Implementation Notes

### Module Integration
1. Create the module file at `crates/maproom/src/progress.rs`
2. Add to `crates/maproom/src/lib.rs`:
   ```rust
   pub mod progress;
   ```

### Thread Safety Considerations
- Use `AtomicUsize` for counters (thread-safe without locks)
- Use `Mutex<Instant>` for last update time (infrequent access, acceptable lock overhead)
- All methods take `&self` (can be shared across threads)

### Implementation Details
1. Implement TTY detection with safe fallback to non-TTY mode
2. Use `AtomicUsize::fetch_add()` for increment operations
3. Implement percentage calculation with zero-division safety:
   ```rust
   fn percentage_files(&self) -> usize {
       if let Some(total) = self.total_files {
           if total > 0 {
               return (self.processed_files.load(Ordering::Relaxed) * 100) / total;
           }
       }
       0
   }
   ```
4. Format output to fit typical terminal width (80+ characters)
5. In `finish()`, print final summary with total elapsed time

### Output Format Examples
```
TTY mode (overwrite):
Processing: 450/1200 files (37%) | Embeddings: 2500/6000 (42%)

Non-TTY mode (newlines):
Progress: 10% complete (120/1200 files)
Progress: 20% complete (240/1200 files)
...
```

## Dependencies
None - this is the first ticket in Phase 1 (Progress Tracking Foundation)

## Risk Assessment
- **Risk**: TTY detection might fail on some terminal emulators or environments
  - **Mitigation**: Safe fallback to non-TTY mode if detection fails; tested behavior ensures graceful degradation

- **Risk**: Atomic operations might have performance overhead
  - **Mitigation**: Atomic operations are very fast for simple counters; benchmarked in later ticket (MRPROG-1005)

- **Risk**: Mutex contention on `last_update` in high-concurrency scenarios
  - **Mitigation**: Throttling at 200ms means lock is acquired max 5 times/second; negligible overhead

## Files/Packages Affected
- **CREATE**: `crates/maproom/src/progress.rs` (new module, ~150-200 lines)
- **MODIFY**: `crates/maproom/src/lib.rs` (add `pub mod progress;`)
- **MODIFY**: `crates/maproom/Cargo.toml` (add `atty = "0.2"` dependency)

## References
- Architecture: `.agents/projects/MRPROG_maproom-progress-ux/planning/architecture.md` (Component Design section)
- Quality Strategy: `.agents/projects/MRPROG_maproom-progress-ux/planning/quality-strategy.md`
- Project Plan: `.agents/projects/MRPROG_maproom-progress-ux/planning/plan.md` (Phase 1)

## Estimated Effort
2-4 hours
