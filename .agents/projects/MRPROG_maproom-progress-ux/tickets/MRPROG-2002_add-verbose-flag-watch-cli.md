# Ticket: MRPROG-2002: Add --verbose flag to watch command CLI

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Add the `--verbose` flag to the watch command in main.rs and wire up OutputMode selection. By default, watch uses minimal output; --verbose restores the detailed multi-line output for debugging.

## Background
The watch_worktree function now supports OutputMode, but the CLI doesn't expose it yet. This ticket adds the flag and wires up the mode selection, completing the watch minimal output feature.

This mirrors the pattern from Phase 1 (scan command) and provides users control over output verbosity.

References:
- Architecture: `.agents/projects/MRPROG_maproom-progress-ux/planning/architecture.md` (CLI Argument Parsing section)
- Plan: `.agents/projects/MRPROG_maproom-progress-ux/planning/plan.md` (Phase 2, task 2)
- Similar pattern: MRPROG-1003 (scan --verbose flag)

## Acceptance Criteria
- [ ] `--verbose` flag added to `Commands::Watch` in main.rs
- [ ] OutputMode determined from flag: false → Minimal (default), true → Verbose
- [ ] OutputMode passed to `watch_worktree()` call
- [ ] `maproom watch` shows minimal output by default
- [ ] `maproom watch --verbose` shows detailed output
- [ ] Help text documents the --verbose flag
- [ ] Manual test confirms both modes work correctly

## Technical Requirements

### CLI Changes in main.rs

1. **Add flag to Commands::Watch:**
```rust
Watch {
    // ... existing fields ...

    #[arg(long, help = "Show detailed output (file-by-file listing)")]
    verbose: bool,
}
```

2. **Update watch command handler:**
```rust
Commands::Watch {
    path,
    repo,
    worktree,
    debounce,
    exclude,
    verbose,  // NEW
    // ... other fields
} => {
    // Determine output mode
    let output_mode = if verbose {
        OutputMode::Verbose
    } else {
        OutputMode::Minimal  // DEFAULT for watch
    };

    // Call watch_worktree with output_mode
    indexer::watch_worktree(
        &pool,
        &root,
        &repo,
        &worktree,
        debounce,
        exclude.as_deref(),
        output_mode,  // NEW PARAMETER
    )
    .await?;

    Ok(())
}
```

3. **Import OutputMode if not already:**
```rust
use crate::progress::OutputMode;
```

### Help Text
- Verbose flag help: "Show detailed output (file-by-file listing)"
- Makes it clear verbose provides more detail, minimal is default

## Implementation Notes

1. Find `Commands::Watch` enum variant (around line 503+ in main.rs)
2. Add verbose field with #[arg] attribute
3. Find watch command handler (around line 503-522)
4. Import OutputMode if needed
5. Determine mode from flag
6. Pass output_mode to watch_worktree
7. Test both modes manually

### Manual Test Procedure
1. Build: `cargo build`
2. Test minimal (default): `maproom watch`
   - Modify a file
   - Verify: "🔄 N file(s) changed", "Indexing: ...", "✅ Done in X.Xs"
3. Test verbose: `maproom watch --verbose`
   - Modify a file
   - Verify: "Detected changes...", file-by-file listing, "Index updated"
4. Test help: `maproom watch --help`
   - Verify --verbose flag documented

## Dependencies
- BLOCKED BY: MRPROG-2001 (needs watch_worktree to support OutputMode)

## Risk Assessment
- **Risk**: None - straightforward CLI wiring, same pattern as scan command
  - **Mitigation**: N/A

## Files/Packages Affected
- MODIFY: `crates/maproom/src/main.rs` (Commands enum and watch handler)
