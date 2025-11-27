# Ticket: SQLIMPL-1004: Migrate Test Files Batch 3 (Incremental)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Migrate incremental indexing test files from PostgreSQL to SQLite. These tests validate file change detection, hash storage, and incremental update functionality.

## Background
Incremental tests verify that maproom can detect file changes and update the index efficiently without full re-indexing. These tests will validate Phase 3 implementations.

This ticket implements Plan Phase 1, Ticket 1004: "Migrate Test Files Batch 3 (Incremental)".

## Acceptance Criteria
- [ ] All 7 `incremental_*.rs` test files compile with SQLite
- [ ] All files pass `cargo test -p crewchief-maproom --no-run`
- [ ] Tests that don't depend on Phase 3 stubs execute successfully
- [ ] Tests requiring Phase 3 work are marked with `#[ignore]`

## Technical Requirements
- Use test helpers from `tests/common/mod.rs`
- Replace PostgreSQL connection setup with SQLite
- Tests may need filesystem fixtures for change detection testing
- Mark tests dependent on unimplemented stubs with `#[ignore]`

## Implementation Notes

### Files to Migrate (7 total)
1. `tests/incremental_cache_test.rs`
2. `tests/incremental_deletions.rs`
3. `tests/incremental_hash_test.rs`
4. `tests/incremental_integration_test.rs`
5. `tests/incremental_processor_test.rs`
6. `tests/incremental_scan_integration.rs`
7. `tests/incremental_update.rs`

### Expected State After Migration
- Tests should **compile** but many won't **pass** until Phase 3 is complete
- Hash storage and retrieval stubs are not yet implemented
- Mark tests as `#[ignore]` with comment: `// Requires Phase 3: incremental implementation`

### Test Patterns
Incremental tests typically:
1. Create initial file state
2. Index files
3. Modify/add/delete files
4. Run incremental update
5. Verify database reflects changes

## Dependencies
- SQLIMPL-1001 (Migrate Test Common Module)

## Risk Assessment
- **Risk**: Tests depend on Phase 3 stub implementations
  - **Mitigation**: Mark as `#[ignore]` during migration
- **Risk**: File system interaction may behave differently in test environment
  - **Mitigation**: Use tempdir for isolated test fixtures

## Files/Packages Affected
- `crates/maproom/tests/incremental_cache_test.rs`
- `crates/maproom/tests/incremental_deletions.rs`
- `crates/maproom/tests/incremental_hash_test.rs`
- `crates/maproom/tests/incremental_integration_test.rs`
- `crates/maproom/tests/incremental_processor_test.rs`
- `crates/maproom/tests/incremental_scan_integration.rs`
- `crates/maproom/tests/incremental_update.rs`
