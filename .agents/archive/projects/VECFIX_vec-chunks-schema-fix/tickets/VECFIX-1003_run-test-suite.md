# Ticket: VECFIX-1003: Run test suite and fix failures

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- unit-test-runner
- rust-indexer-engineer (if fixes needed)
- verify-ticket
- commit-ticket

## Summary

Execute the full test suite for the maproom crate and fix any test failures resulting from the vec_chunks code removal in VECFIX-1001 and VECFIX-1002.

## Background

After removing the deprecated `vec_chunks` code (VECFIX-1001) and cleaning up the schema (VECFIX-1002), we need to verify all existing tests pass. The `embeddings.rs` module has comprehensive tests that should continue to work, but some tests might have referenced the old `vec_chunks` table or the deprecated functions that were removed.

This ticket implements the testing verification described in:
- `plan.md` Phase 2: VECFIX-1003
- `quality-strategy.md` Section 1: Existing Test Suite

## Acceptance Criteria
- [x] All tests in `cargo test -p crewchief-maproom` pass
- [x] No tests reference `vec_chunks` table
- [x] No clippy warnings from `cargo clippy -p crewchief-maproom`
- [x] Test output shows all 8 key embedding tests passing (listed below)

## Technical Requirements

- Run the full test suite: `cargo test -p crewchief-maproom`
- Fix any failing tests that broke due to code removal
- Update any tests that reference the deprecated `vec_chunks` table
- Verify all 8 key embedding tests pass:
  - `test_vector_table_sync`
  - `test_vector_table_sync_update`
  - `test_sync_all_embeddings_to_vec`
  - `test_768_dim_embedding_storage`
  - `test_768_dim_vector_table_sync`
  - `test_mixed_dimensions_storage`
  - `test_sync_all_mixed_dimensions`
  - `test_unsupported_dimension`
- Run clippy to ensure no new warnings: `cargo clippy -p crewchief-maproom`

## Implementation Notes

**Test Execution Strategy**:

1. **Initial test run**: Execute `cargo test -p crewchief-maproom` to identify failures
2. **Failure analysis**: Determine if failures are due to:
   - Tests referencing removed `upsert_embeddings()` or `batch_upsert_embeddings()` functions
   - Tests referencing the `vec_chunks` table
   - Tests that need to use the correct `embeddings.rs` module functions
3. **Fix approach**:
   - If tests reference deprecated functions, update to use `store.upsert_embedding()` (singular)
   - If tests reference `vec_chunks`, update to use `code_embeddings` table
   - Ensure tests still verify the same behavior, just using the correct code path

**Expected Test Behavior**:

The existing `embeddings.rs` tests should all pass without modification since they already use the correct implementation. Any failures likely indicate:
- Tests in other modules that called the deprecated code
- Mock or stub code that referenced `vec_chunks`

**Search for problematic tests**:
```bash
rg "vec_chunks" crates/maproom/src --type rust
rg "batch_upsert_embeddings|upsert_embeddings\(" crates/maproom/src --type rust
```

## Dependencies

**Prerequisite Tickets**:
- VECFIX-1001: Remove vec_chunks code and migrate callers (MUST be complete)
- VECFIX-1002: Remove vec_chunks from schema.rs (MUST be complete)

**External Dependencies**:
- None

## Risk Assessment

- **Risk**: Tests may fail due to undetected callers of deprecated functions
  - **Mitigation**: Thorough search with `rg` before removal; compilation errors would have caught most issues

- **Risk**: Tests may have fragile mocks or stubs that reference `vec_chunks`
  - **Mitigation**: Update tests to use correct `code_embeddings` table and `embeddings.rs` module

- **Risk**: Integration tests may fail if pipeline changes broke embedding storage
  - **Mitigation**: This is caught by unit tests first; E2E verification in VECFIX-1004

## Files/Packages Affected

- `crates/maproom/src/**/*.rs` - Any test files that reference `vec_chunks` or deprecated functions
- Primary test locations to check:
  - `crates/maproom/src/db/sqlite/embeddings.rs` - Core embedding tests
  - `crates/maproom/src/db/sqlite/mod.rs` - Tests in main module
  - `crates/maproom/src/embedding/pipeline.rs` - Pipeline tests
  - Any other test modules that interact with embedding storage
