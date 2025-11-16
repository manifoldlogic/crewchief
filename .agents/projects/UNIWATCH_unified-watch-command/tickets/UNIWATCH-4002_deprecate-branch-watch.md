# Ticket: UNIWATCH-4002: Add Deprecation Warning to branch-watch Command

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (trivial change, no tests required)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Add a deprecation warning to the branch-watch command directing users to use the unified watch command instead, while keeping the command functional.

## Background
The branch-watch command is now redundant since watch handles branch switches automatically. We want to deprecate it gracefully - keep it working but warn users to switch to the unified watch command.

This ticket implements the deprecation strategy for Phase 4 (CLI Integration & Polish) of the UNIWATCH project, guiding users toward the unified watch command while maintaining backward compatibility.

## Acceptance Criteria
- [ ] Deprecation warning logged when branch-watch is invoked
- [ ] Warning includes clear migration guidance
- [ ] Command still works (no functional change to behavior)
- [ ] Warning goes to stderr (not stdout)
- [ ] Warning is user-friendly and actionable

## Technical Requirements
- Location: `crates/maproom/src/main.rs` (Commands::BranchWatch match arm)
- Approximately 5 lines of modifications
- Use `eprintln!()` for warning message
- Clear, actionable guidance for users
- No change to existing branch-watch functionality

## Implementation Notes

Add deprecation warning at the start of the Commands::BranchWatch match arm:

```rust
Commands::BranchWatch { repo } => {
    eprintln!("⚠️  Warning: 'branch-watch' is deprecated.");
    eprintln!("The 'watch' command now handles branch switches automatically.");
    eprintln!("Use 'maproom watch' instead for unified file and branch watching.");
    eprintln!();

    // Existing branch-watch implementation continues unchanged
    let mut watcher = BranchWatcher::new(repo, client)?;
    watcher.start(None).await?;
}
```

**Key considerations:**
- Warning emoji (⚠️) provides visual indication
- Clear explanation of why it's deprecated
- Specific migration path provided
- Empty line after warning for readability
- Existing functionality remains completely intact
- Uses stderr to avoid interfering with any output parsing

## Dependencies
- None

## Risk Assessment
- **Risk**: None significant (non-breaking change)
  - **Mitigation**: N/A
- **Risk**: Users might miss the warning
  - **Mitigation**: Use clear emoji and formatting, stderr ensures visibility

## Files/Packages Affected
- `crates/maproom/src/main.rs` (~5 line additions to Commands::BranchWatch match arm)
