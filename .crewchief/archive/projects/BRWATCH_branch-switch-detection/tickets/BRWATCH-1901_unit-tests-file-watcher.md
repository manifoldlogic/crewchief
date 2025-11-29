# Ticket: BRWATCH-1901: Unit tests for file watcher

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute comprehensive unit tests for Phase 1 file watcher implementation, validating branch name parsing, file event detection, and error handling.

## Background
This is a critical path test ticket for Phase 1. From quality-strategy.md, unit tests represent 50% of the test pyramid and focus on branch parsing and event handling.

The unit tests validate:
1. Branch name parsing from various .git/HEAD formats
2. File watcher event handling
3. Error scenarios (missing files, invalid formats)

This ticket executes tests created in BRWATCH-1003 and validates BRWATCH-1002 functionality.

**Planning Reference**: `/workspace/.crewchief/projects/BRWATCH_branch-switch-detection/planning/quality-strategy.md` - Lines 26-82 (Unit Tests)

## Acceptance Criteria
- [x] All branch parsing tests pass (test_parse_branch_ref, test_parse_feature_branch, test_parse_detached_head)
- [x] Error handling tests pass (test_parse_invalid_format, test_parse_empty_sha)
- [x] File watcher event detection test passes (if implemented in BRWATCH-1002)
- [x] Test execution completes without panics
- [x] All tests documented with clear failure messages
- [x] No test failures or compilation errors

## Technical Requirements
- Run tests with: `cargo test --lib watcher -- --nocapture`
- Execute all #[test] functions in src/watcher.rs
- Verify test coverage includes:
  - Standard branch refs (main, develop, etc.)
  - Feature branches with slashes (feature/auth-system)
  - Detached HEAD state (commit SHA)
  - Invalid formats
  - Edge cases (empty content, short SHA)
- Report pass/fail status for each test
- Document any failures with reproduction steps

## Implementation Notes

### Test Execution Strategy
```bash
# Run watcher unit tests
cargo test --lib watcher -- --nocapture

# Expected output:
# test watcher::tests::test_parse_branch_ref ... ok
# test watcher::tests::test_parse_feature_branch ... ok
# test watcher::tests::test_parse_detached_head ... ok
# test watcher::tests::test_parse_invalid_format ... ok
# test watcher::tests::test_parse_empty_sha ... ok
#
# test result: ok. 5 passed; 0 failed
```

### Validation Checklist
From quality-strategy.md:
- ✅ `test_parse_branch_ref` - Standard branch parsing
- ✅ `test_parse_feature_branch` - Feature branch with slash
- ✅ `test_parse_detached_head` - Detached HEAD (SHA)
- ✅ `test_parse_invalid_format` - Error handling

### Success Criteria
All tests must pass. Any failures indicate:
- Implementation bug in get_current_branch()
- Missing edge case handling
- Incorrect error handling

**Do NOT modify code** - this is a test-runner ticket. Report failures to rust-indexer-engineer for fixes.

## Dependencies
- BRWATCH-1001 complete (dependencies added)
- BRWATCH-1002 complete (BranchWatcher struct implemented)
- BRWATCH-1003 complete (get_current_branch function with tests)

## Risk Assessment
- **Risk**: Tests pass locally but fail in CI
  - **Mitigation**: Run tests multiple times, check for flakiness
- **Risk**: File watcher tests require actual .git/HEAD file
  - **Mitigation**: Use test helpers that parse strings directly (parse_head_content)
- **Risk**: Platform-specific test failures
  - **Mitigation**: Run on Linux (primary target), document platform-specific issues

## Files/Packages Affected
- `/workspace/crates/maproom/src/watcher.rs` (test execution only, no modifications)
