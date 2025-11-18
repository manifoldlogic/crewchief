# Ticket: IDXCLEAN-3003: End-to-End CLI Integration Tests

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
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create end-to-end tests of CLI command behavior to ensure proper user experience and safety. Verify that the cleanup command behaves correctly from the user's perspective: dry-run by default, confirmation required for deletion, clear error messages, and correct exit codes.

## Background
The CLI is the primary user interface for cleanup operations. These tests verify that the command behaves correctly from the user's perspective, ensuring safety through dry-run defaults and explicit confirmation requirements. This ticket is part of Phase 3: Integration Testing and Safety Validation.

**References:**
- `plan.md`: Phase 3 - Integration Testing and Safety Validation, ticket IDXCLEAN-3003 (lines 470-501)
- `quality-strategy.md`: Path 3 - CLI Usability (lines 186-235)

## Acceptance Criteria
- [ ] Test: Default execution is dry-run (no database changes without --confirm)
- [ ] Test: --confirm flag actually deletes stale worktrees
- [ ] Test: Output format is correct and informative
- [ ] Test: Exit codes are correct (0=success, 1=error, 2=no stale worktrees found)
- [ ] Test: Error handling works (database failure, no stale found)
- [ ] All tests pass

## Technical Requirements
- Test CLI command invocation (not just module functions)
- Capture stdout/stderr for output verification
- Verify exit codes match specification:
  - Exit code 0: Successful cleanup (or dry-run)
  - Exit code 1: Error occurred (database failure, permission issues)
  - Exit code 2: No stale worktrees found (informational, not an error)
- Test with real database + tempfile fixtures
- Verify dry-run makes zero database changes
- Verify --confirm flag triggers actual deletion
- Test error scenarios: database connection failure, no stale worktrees

## Implementation Notes
See `quality-strategy.md` lines 210-234 for complete test examples.

**Key test scenarios:**

1. **Default dry-run behavior:**
```rust
#[tokio::test]
async fn test_cli_default_is_dry_run() {
    let db = setup_test_db().await;
    let stale_id = create_stale_worktree(&db, "/tmp/stale").await;

    let cmd = CleanupStaleCommand { confirm: false, verbose: false };
    cmd.execute(&test_config()).await.unwrap();

    // Verify: worktree still exists (dry-run)
    assert!(db.get_worktree(stale_id).await.is_ok());
}
```

2. **Confirm flag triggers deletion:**
```rust
#[tokio::test]
async fn test_cli_confirm_actually_deletes() {
    let db = setup_test_db().await;
    let stale_id = create_stale_worktree(&db, "/tmp/stale").await;

    let cmd = CleanupStaleCommand { confirm: true, verbose: false };
    cmd.execute(&test_config()).await.unwrap();

    // Verify: worktree deleted
    assert!(db.get_worktree(stale_id).await.is_err());
}
```

3. **Exit codes:**
- Test exit code 0 for successful operations
- Test exit code 1 for errors (database connection failure)
- Test exit code 2 for "no stale worktrees found"

4. **Output format:**
- Capture stdout/stderr
- Verify output includes count of stale worktrees
- Verify output lists paths being deleted/would be deleted
- Verify dry-run includes clear warning message

## Dependencies
- **IDXCLEAN-2002** (CLI execution logic) - Required: CLI command implementation
- **IDXCLEAN-2004** (main.rs integration) - Required: CLI subcommand registration

## Risk Assessment
- **Risk**: Tests may not accurately reflect real CLI invocation behavior
  - **Mitigation**: Test CleanupStaleCommand struct directly (mirrors actual CLI execution path)

- **Risk**: Test database setup may not match production environment
  - **Mitigation**: Use same DatabaseManager and connection pooling as production code

## Files/Packages Affected
**Files Created:**
- `crates/maproom/tests/cleanup_cli_test.rs` (new test file)

**Files Referenced:**
- `crates/maproom/src/db/cleanup/cli.rs` (CleanupStaleCommand implementation)
- `crates/maproom/src/db/cleanup/mod.rs` (StaleWorktreeDetector, WorktreeCleaner)
