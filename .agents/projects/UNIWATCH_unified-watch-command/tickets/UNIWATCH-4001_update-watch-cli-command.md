# Ticket: UNIWATCH-4001: Update Commands::Watch to Auto-Detect Branch

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
- rust-indexer-engineer
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Modify the CLI watch command handler to auto-detect the current branch when --worktree flag is not provided, and add deprecation warning when it is provided.

## Background
Currently, the --worktree flag is required for the watch command. With unified watching, we want users to simply run `maproom watch` and have it auto-detect the current branch. The --worktree flag should still work for backward compatibility but emit a deprecation warning.

This ticket implements the CLI interface updates for Phase 4 (CLI Integration & Polish) of the UNIWATCH project, enabling users to use the unified watch command without manual worktree specification.

## Acceptance Criteria
- [ ] Auto-detect branch using `get_current_branch()` when --worktree not provided
- [ ] Log deprecation warning to stderr when --worktree is provided
- [ ] Both usage modes work correctly (with and without --worktree)
- [ ] Help text updated to reflect auto-detection
- [ ] Integration test `test_watch_auto_detects_branch()` passes
- [ ] Backward compatibility test passes

## Technical Requirements
- Location: `crates/maproom/src/main.rs` (Commands::Watch match arm, around line 150-160)
- Approximately 15 lines of modifications
- Use `eprintln!()` for deprecation warning (goes to stderr)
- Use `get_current_branch()` from git module
- Same `watch_worktree()` function call (no API change)
- Ensure error handling for detached HEAD state

## Implementation Notes

The implementation should modify the Commands::Watch match arm as follows:

```rust
// BEFORE:
Commands::Watch { repo, worktree, path, throttle } => {
    let (repo_name, branch_name, _) = get_git_info(&path)?;
    let repo = repo.unwrap_or(repo_name);
    let worktree = worktree.unwrap_or(branch_name);

    indexer::watch_worktree(&client, &repo, &worktree, &path, &throttle).await?;
}

// AFTER:
Commands::Watch { repo, worktree, path, throttle } => {
    let (repo_name, _, _) = get_git_info(&path)?;
    let repo = repo.unwrap_or(repo_name);

    // Auto-detect current branch
    let detected_branch = get_current_branch(&path)?;

    let worktree = if let Some(wt) = worktree {
        eprintln!("Warning: --worktree flag is deprecated and ignored.");
        eprintln!("The watch command now auto-detects branch switches.");
        eprintln!("Using auto-detected branch: {}", detected_branch);
        detected_branch
    } else {
        detected_branch
    };

    indexer::watch_worktree(&client, &repo, &worktree, &path, &throttle).await?;
}
```

**Key considerations:**
- The `get_current_branch()` function should be used from the existing git module
- Error handling should provide clear message if branch detection fails (e.g., detached HEAD)
- Deprecation warning uses stderr to remain visible even when stdout is piped
- The warning is informative and guides users on the new behavior

## Dependencies
- None (CLI change is independent of core implementation)

## Risk Assessment
- **Risk**: `get_current_branch()` might fail in detached HEAD state
  - **Mitigation**: Handle error gracefully with clear error message explaining the state
- **Risk**: Users might not see deprecation warning
  - **Mitigation**: Use stderr (which is visible even when stdout is piped)
- **Risk**: Breaking change for scripts using --worktree
  - **Mitigation**: Maintain backward compatibility - flag still works, just with warning

## Files/Packages Affected
- `crates/maproom/src/main.rs` (~15 line modifications in Commands::Watch match arm)
- Integration tests (execution and validation)
