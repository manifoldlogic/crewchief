# Ticket: MRPROG-1003: Add --verbose flag and wire up ProgressTracker in scan command

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
Add the `--verbose` flag to the scan command CLI and create a ProgressTracker instance in the scan handler, passing it to `scan_worktree()`. This completes the integration, making progress visible to users.

## Background
The ProgressTracker module exists (MRPROG-1001) and is integrated into `scan_worktree()` (MRPROG-1002), but currently nothing creates or uses it. This ticket adds the CLI flag to control output mode and creates the tracker in the command handler.

By default, users get progress updates (Minimal mode). The `--verbose` flag provides future extensibility for more detailed output if needed.

This ticket implements the CLI Argument Parsing section of the architecture document, completing Phase 1 of the Maproom Progress UX Enhancement project.

## Acceptance Criteria
- [ ] `--verbose` flag added to `Commands::Scan` in main.rs
- [ ] OutputMode determined from --verbose flag (false → Minimal, true → Verbose)
- [ ] ProgressTracker created in scan command handler
- [ ] ProgressTracker passed to `scan_worktree()` call
- [ ] `maproom scan` shows real-time progress during indexing
- [ ] `maproom scan --verbose` works (same output for now, future-proofing)
- [ ] Help text updated to document --verbose flag

## Technical Requirements

### CLI Changes in main.rs

**1. Add flag to Commands::Scan:**
```rust
Scan {
    // ... existing fields ...

    #[arg(long, help = "Show detailed output")]
    verbose: bool,
}
```

**2. Update scan command handler:**
```rust
Commands::Scan {
    path,
    repo,
    worktree,
    commit,
    verbose,
    // ... other fields
} => {
    // Determine output mode
    let mode = if verbose {
        OutputMode::Verbose
    } else {
        OutputMode::Minimal
    };

    // Create progress tracker
    let progress = ProgressTracker::new(mode);

    // Call scan_worktree with progress
    let stats = indexer::scan_worktree(
        &pool,
        &root,
        &repo,
        &worktree,
        &commit,
        languages.as_deref(),
        exclude.as_deref(),
        parallel,
        concurrency,
        Some(&progress),  // NOW PASSING TRACKER
    )
    .await?;

    Ok(())
}
```

**3. Import OutputMode:**
```rust
use crate::progress::{ProgressTracker, OutputMode};
```

### Help Text
- The `--verbose` flag help text: "Show detailed output"
- Document in command description that scan shows progress by default

## Implementation Notes

### Implementation Steps
1. Find the Commands::Scan enum variant (around line 387 in main.rs)
2. Add the verbose field with #[arg] attribute
3. Find the scan command handler (around line 387-476)
4. Import ProgressTracker and OutputMode
5. Create tracker and pass to scan_worktree
6. Test manually: `cargo build && ./target/debug/maproom scan`

### Key Decisions
- Default to Minimal mode (progress shown) rather than silent mode
- --verbose flag future-proofs for more detailed output modes
- ProgressTracker is created once per scan and passed by reference
- OutputMode determines behavior at creation time

### Testing Approach
Manual testing is sufficient for this CLI wiring ticket:
- Run scan on test repository (e.g., CrewChief itself)
- Verify progress updates appear every 200-500ms
- Verify final "Completed in X.Xs" message appears
- Verify --verbose flag works (should show same output for now)

## Dependencies
- **BLOCKED BY**: MRPROG-1001 (needs ProgressTracker module)
- **BLOCKED BY**: MRPROG-1002 (needs scan_worktree integration)
- **BLOCKS**: None (completes Phase 1)

## Risk Assessment
- **Risk**: None identified
  - **Mitigation**: This is straightforward CLI wiring with no complex logic

- **Risk**: User confusion about --verbose flag behavior
  - **Mitigation**: Help text explains that progress is shown by default, --verbose is for future detailed output

## Files/Packages Affected
- **MODIFY**: `/workspace/crates/maproom/src/main.rs` (Commands enum and scan handler)
  - Add verbose field to Commands::Scan enum
  - Import ProgressTracker and OutputMode
  - Create ProgressTracker in scan handler
  - Pass tracker to scan_worktree call
