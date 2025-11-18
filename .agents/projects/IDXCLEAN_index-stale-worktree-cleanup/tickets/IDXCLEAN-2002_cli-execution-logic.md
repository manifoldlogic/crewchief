# Ticket: IDXCLEAN-2002: Implement CLI Command Execution Logic

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
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the three-phase CLI command execution flow for manual stale worktree cleanup: detection → report → deletion (with confirmation). This is the core user workflow that orchestrates the detection and deletion modules created in Phase 1.

## Background
With the CLI structure in place (IDXCLEAN-2001), this ticket implements the execution logic that brings together the StaleWorktreeDetector (IDXCLEAN-1001) and WorktreeCleaner (IDXCLEAN-1002) into a cohesive user experience. The command must default to dry-run mode for safety, requiring explicit `--confirm` flag to actually delete stale worktrees.

This implements **Phase 2 - CLI Command Interface** from the project plan, specifically the execution flow described in lines 261-298.

## Acceptance Criteria
- [ ] Execute detection phase using StaleWorktreeDetector from IDXCLEAN-1001
- [ ] Display formatted findings to user with clear information about stale worktrees
- [ ] Execute deletion phase using WorktreeCleaner if --confirm flag is provided
- [ ] Show progress indicators during each phase (using emojis: 🔍 📊 🗑️ ✅)
- [ ] Handle errors gracefully with user-friendly messages (not raw stack traces)
- [ ] Implement correct exit codes: 0 = success, 1 = error, 2 = no stale worktrees found
- [ ] Integration test: Verify dry-run mode doesn't delete anything
- [ ] Integration test: Verify --confirm flag actually deletes stale worktrees

## Technical Requirements
- Use `StaleWorktreeDetector` from IDXCLEAN-1001 for detection phase
- Use `WorktreeCleaner` from IDXCLEAN-1002 for deletion phase
- Default behavior MUST be dry-run (no deletion without --confirm)
- Progress indicators should use clear emoji markers: 🔍 (detection), 📊 (report), 🗑️ (deletion), ✅ (success)
- Error handling must convert technical errors into user-friendly messages
- Exit codes must follow standard CLI conventions:
  - 0: Successful execution (with or without deletions)
  - 1: Error occurred during execution
  - 2: No stale worktrees found (informational)

## Implementation Notes
Follow the execution flow pattern from plan.md lines 273-290:

```rust
impl CleanupStaleCommand {
    pub async fn execute(&self, cfg: &Config) -> Result<()> {
        // Phase 1: Detection
        println!("🔍 Detecting stale worktrees...");
        let detector = StaleWorktreeDetector::new(cfg);
        let stale = detector.detect().await?;

        // Phase 2: Report
        if stale.is_empty() {
            println!("✅ No stale worktrees found!");
            return Ok(());
        }

        println!("📊 Found {} stale worktrees:", stale.len());
        for entry in &stale {
            println!("  - {} (indexed: {}, disk: {})",
                entry.worktree_name,
                if entry.in_database { "yes" } else { "no" },
                if entry.on_disk { "yes" } else { "no" }
            );
        }

        // Phase 3: Deletion (if confirmed)
        if self.confirm {
            println!("🗑️  Deleting stale worktrees...");
            let cleaner = WorktreeCleaner::new(cfg);
            let results = cleaner.cleanup(&stale).await?;
            println!("✅ Cleanup complete: {} deleted, {} errors",
                results.deleted_count, results.error_count);
        } else {
            println!("⚠️  This was a dry-run. Use --confirm to actually delete.");
        }

        Ok(())
    }
}
```

**Error Handling Strategy:**
- Convert `anyhow::Error` into user-friendly messages
- Show technical details only with `--verbose` flag
- Always indicate what the user should do to fix issues

**Progress Indicators:**
- Keep output clean and scannable
- Use consistent emoji markers
- Show summary statistics for each phase

## Dependencies
- **IDXCLEAN-2001**: CLI structure and command definition (not yet created)
- **IDXCLEAN-1001**: StaleWorktreeDetector module
- **IDXCLEAN-1002**: WorktreeCleaner module

## Risk Assessment
- **Risk**: User accidentally deletes worktrees without understanding consequences
  - **Mitigation**: Default to dry-run mode, require explicit --confirm flag, show clear summary before deletion

- **Risk**: Errors during deletion leave database in inconsistent state
  - **Mitigation**: WorktreeCleaner handles partial failures gracefully (from IDXCLEAN-1002)

- **Risk**: Poor error messages confuse users
  - **Mitigation**: Wrap all technical errors with context about what operation failed and suggested remediation

## Files/Packages Affected
- `crates/maproom/src/cli/commands/db.rs` - Implement execute method for CleanupStaleCommand
- Integration test file (to be created during implementation)
