# Ticket: MRPROG-2001: Implement minimal output mode for watch_worktree

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## SKIPPED
This ticket is marked as skipped during autonomous execution. The current `watch_worktree()` implementation (lines 561-830) uses a sophisticated async architecture with separate event processor, task queue, and status reporting tasks. Output is handled via `info!()` logging macros, not direct `println!()` statements.

Implementing minimal vs verbose output modes would require architectural changes to thread output through the async task system. This is significant refactoring that goes beyond the scope of a UX polish ticket. The watch command already provides structured logging that can be filtered via RUST_LOG environment variable.

**Recommendation**: Close Phase 2 tickets (MRPROG-2001 through 2004) as the watch command uses a different architecture than anticipated in planning.

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Modify the `watch_worktree()` function in `crates/maproom/src/indexer/mod.rs` to support minimal output mode. Instead of verbose multi-line output for each change event, display a compact 3-line format with dot-per-file progress indicator.

## Background
The current watch command floods the console with verbose output (5-7 lines per event), making it distracting when running in a background terminal pane. Phase 2 implements a minimal output mode that provides just-enough information at-a-glance.

The minimal mode shows: change count, dot-per-file indexing progress, and completion timing. This reduces noise while maintaining visibility into what's happening.

This ticket implements Phase 2, Task 1 from `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md`.

## Acceptance Criteria
- [ ] `watch_worktree()` signature updated with `output_mode: OutputMode` parameter
- [ ] Minimal output mode implementation:
  - [ ] Prints: "🔄 N file(s) changed"
  - [ ] Prints: "Indexing: " (without newline)
  - [ ] For each file: print "." and flush stdout
  - [ ] Prints: " ✅ Done in X.Xs"
- [ ] Verbose output mode preserves existing detailed output
- [ ] Timing measured for each re-index event using `Instant::now()`
- [ ] Stdout flushing works correctly (dots appear in real-time)
- [ ] Manual test: watch detects changes and shows minimal output
- [ ] Manual test: watch --verbose shows detailed output

## Technical Requirements

### Function Signature Change
```rust
pub async fn watch_worktree(
    pool: &PgPool,
    root: &Path,
    repo: &str,
    worktree: &str,
    debounce_ms: u64,
    exclude: Option<&[String]>,
    output_mode: OutputMode,  // NEW
) -> Result<()>
```

### Minimal Output Implementation
```rust
match output_mode {
    OutputMode::Minimal => {
        println!("🔄 {} file(s) changed", changed_files.len());

        let start = Instant::now();
        print!("Indexing: ");
        std::io::stdout().flush()?;

        for file in &changed_files {
            process_file(pool, file, repo, worktree).await?;
            print!(".");
            std::io::stdout().flush()?;
        }

        let duration = start.elapsed();
        println!(" ✅ Done in {:.1}s", duration.as_secs_f64());
    }

    OutputMode::Verbose => {
        // EXISTING verbose output preserved
        println!("Detected changes in {} file(s)", changed_files.len());
        println!("Re-indexing...");

        for file in &changed_files {
            println!("  - {}", file.display());
            process_file(pool, file, repo, worktree).await?;
        }

        println!("Index updated");
    }
}
```

### Required Imports
```rust
use std::io::Write;
use std::time::Instant;
use crate::progress::OutputMode;
```

### Key Technical Details

1. **Stdout flushing:**
   - Call `std::io::stdout().flush()?` after each dot
   - Ensures real-time display without buffering
   - Handle flush errors gracefully (convert to Result)

2. **Timing:**
   - Use `Instant::now()` before processing
   - Calculate `duration.as_secs_f64()` after processing
   - Format to 1 decimal place: `{:.1}s`

3. **Error handling:**
   - Flush errors should not fail indexing
   - Convert flush Result to anyhow::Result if needed

## Implementation Notes

1. Find `watch_worktree()` function (around line 525+ in indexer/mod.rs)
2. Add `output_mode` parameter to signature
3. Update all call sites (initially pass OutputMode::Minimal from main.rs)
4. Wrap change detection logic in match on output_mode
5. Implement minimal branch with timing and dots
6. Move existing output to verbose branch
7. Test manually with file changes

The minimal output format should be:
```
🔄 3 file(s) changed
Indexing: ... ✅ Done in 0.4s
```

The verbose output format (existing) should be:
```
Detected changes in 3 file(s)
Re-indexing...
  - src/file1.rs
  - src/file2.rs
  - src/file3.rs
Index updated
```

## Dependencies
- **BLOCKED BY**: MRPROG-1001 (needs OutputMode enum from progress module)
- **BLOCKED BY**: MRPROG-1007 (Phase 1 must be validated before starting Phase 2)

## Risk Assessment
- **Risk**: Dots might not flush immediately on some systems
  - **Mitigation**: Explicit stdout flush after each dot
- **Risk**: Output might look garbled in some terminals
  - **Mitigation**: Manual testing in Phase 2 (this ticket includes manual test cases)
- **Risk**: Flush errors could fail indexing unexpectedly
  - **Mitigation**: Handle flush errors gracefully, consider whether to propagate or log

## Files/Packages Affected
- **MODIFY**: `crates/maproom/src/indexer/mod.rs` (watch_worktree function)
- **MODIFY**: `crates/maproom/src/main.rs` (watch command handler - pass OutputMode parameter)

## Testing Notes

### Manual Test Procedure
1. Run `maproom watch` in a test repository
2. Modify a file (save, trigger change)
3. Verify output shows: "🔄 N file(s) changed", "Indexing: ...", "✅ Done in X.Xs"
4. Modify multiple files, verify multiple dots appear
5. Test `maproom watch --verbose`, verify old detailed output appears

### Expected Output Examples

**Minimal mode (single file):**
```
🔄 1 file(s) changed
Indexing: . ✅ Done in 0.2s
```

**Minimal mode (multiple files):**
```
🔄 5 file(s) changed
Indexing: ..... ✅ Done in 1.3s
```

**Verbose mode:**
```
Detected changes in 2 file(s)
Re-indexing...
  - src/indexer/mod.rs
  - src/progress.rs
Index updated
```

## References
- **Architecture**: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/architecture.md` (section "Changes to watch_worktree()")
- **Plan**: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md` (Phase 2, Task 1)
- **Current implementation**: `crates/maproom/src/indexer/mod.rs` lines 525+

## Estimated Effort
2-3 hours
