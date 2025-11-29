# Ticket: EMBCOPY-1002: Add Unit Tests for Embedding Copy Function

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all tests passing
- [ ] **Verified** - by the verify-ticket agent

## Test Execution Report

**Command:** `cargo test --package crewchief-maproom --lib embedding::pipeline::tests::test_copy`

**Results:**
- Total: 3 tests
- Passed: 3
- Failed: 0
- Duration: 0.37s

**Test Status:**
1. **test_copy_existing_embeddings_success**: PASSED ✓
   - Verifies successful copy when cache entry exists with matching blob_sha
   - Confirms embedding population and timestamp updates

2. **test_copy_skips_without_cache**: PASSED ✓
   - Verifies graceful skip when no cache entry exists
   - Confirms embeddings remain NULL, no errors thrown

3. **test_copy_idempotent**: PASSED ✓
   - Verifies idempotent behavior: running on already-populated chunks returns 0
   - Confirms embeddings remain unchanged on repeated calls

**Fix Applied:**
Initial test failures were due to test interference when run in parallel. The `copy_existing_embeddings()` method operates on ALL chunks in the database, so parallel tests would affect each other's results. Fixed by adding `#[serial_test::serial]` attribute to all three tests, ensuring they run sequentially and don't interfere.

**Additional Improvements:**
- Fixed cleanup function to accept blob_sha parameter for proper cache cleanup
- Ensured all tests clean up both chunk data AND code_embeddings entries
- Tests now properly isolated with unique blob_shas per test run

## Implementation Notes

All three tests implemented in `crates/maproom/src/embedding/pipeline.rs` (lines 1105-1257).

**Test Implementation:**
1. `test_copy_existing_embeddings_success` - Tests successful copy when cache entry exists
2. `test_copy_skips_without_cache` - Tests graceful skip when no cache entry
3. `test_copy_idempotent` - Tests idempotent behavior (safe to run multiple times)

**Test Infrastructure:**
- Helper: `create_test_client()` - Database connection
- Helper: `setup_test_chunk()` - Creates unique test data (repo, worktree, file, chunk)
- Helper: `insert_cache_entry()` - Inserts code_embeddings entry
- Helper: `cleanup_test_data()` - Proper cleanup in dependency order
- All tests use `#[serial_test::serial]` to prevent parallel interference

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Write comprehensive unit tests for the `copy_existing_embeddings()` method to ensure it correctly copies embeddings from the cache, skips chunks without cache entries, and handles idempotent operations.

## Background
The embedding copy step (EMBCOPY-1001) is critical infrastructure that must work correctly to avoid slow scans and wasted API costs. Unit tests verify the core behaviors:
1. Successful copy when cache entry exists
2. Graceful skip when no cache entry
3. Idempotent behavior (safe to run multiple times)

These tests provide fast feedback during development and prevent regressions. This ticket implements the testing strategy outlined in `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/quality-strategy.md` (lines 9-40) and fulfills the testing requirements from `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/plan.md` (lines 53-76).

## Acceptance Criteria
- [x] Test `test_copy_existing_embeddings_success` implemented and passing
- [x] Test `test_copy_skips_without_cache` implemented and passing
- [x] Test `test_copy_idempotent` implemented and passing
- [x] All tests pass: `cargo test copy_existing`
- [x] Tests use proper setup/teardown for database state
- [x] Tests verify both chunk updates and return counts

## Technical Requirements

### Test 1: `test_copy_existing_embeddings_success`
- Setup: Insert test chunk with NULL embeddings + matching `code_embeddings` entry with same blob_sha
- Execute: Call `copy_existing_embeddings()`
- Assert:
  - Chunk now has `code_embedding` and `text_embedding` populated
  - Return count equals 1
  - `updated_at` timestamp changed

### Test 2: `test_copy_skips_without_cache`
- Setup: Insert chunk with NULL embeddings, NO matching `code_embeddings` entry
- Execute: Call `copy_existing_embeddings()`
- Assert:
  - Chunk still has NULL embeddings
  - Return count equals 0
  - No errors thrown

### Test 3: `test_copy_idempotent`
- Setup: Insert chunk with embeddings already set
- Execute: Call `copy_existing_embeddings()` twice
- Assert:
  - Embeddings unchanged
  - No errors on second call
  - Return count equals 0 (already has embeddings)

## Implementation Notes

### Test Infrastructure
- Use `#[tokio::test]` for async test support
- Place tests in `crates/maproom/src/embedding/pipeline.rs` under `#[cfg(test)]` module
- Use test database or transactions for isolation
- Create helper functions for test data setup if needed

### Key Test Behaviors
- Test blob_sha matching logic explicitly
- Verify SQL query handles both `code_embedding IS NULL OR text_embedding IS NULL` condition
- Verify proper NULL handling for chunks without cache entries
- Ensure timestamps update correctly
- Test return count accuracy

### Test Data Setup
- Create minimal test chunks with required fields (id, worktree_id, blob_sha, embeddings)
- Create matching code_embeddings entries with same blob_sha
- Use realistic embedding vectors (simple test vectors acceptable)
- Clean up test data after each test

## Dependencies
- EMBCOPY-1001 must be complete (copy function implemented)

## Risk Assessment
- **Risk**: Test database setup complexity
  - **Mitigation**: Use clear setup/teardown helpers and transactions for isolation

- **Risk**: Flaky tests due to timing
  - **Mitigation**: Use proper async handling with tokio::test

- **Risk**: Incomplete coverage
  - **Mitigation**: Test all three core scenarios (success, skip, idempotent)

- **Risk**: Tests may pass locally but fail in CI
  - **Mitigation**: Use consistent test database setup matching CI environment

## Files/Packages Affected
- `crates/maproom/src/embedding/pipeline.rs` (add tests section)

## Planning References
- Plan: `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/plan.md` (lines 53-76)
- Quality Strategy: `.crewchief/projects/EMBCOPY_embedding-inheritance-fix/planning/quality-strategy.md` (lines 9-40)
