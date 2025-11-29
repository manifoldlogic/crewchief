# Ticket: IDXSIZE-2001: Create automated test suite

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 30/30 tests passed (see Test Execution section)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive automated test suite (`test_index_migration.sh`) that validates the migration works correctly on empty databases and with test data across L1-L3 testing levels.

## Background
Following the quality strategy pyramid, we need automated tests covering SQL syntax (L1), empty database (L2), and data population (L3). This test suite must run in CI/CD and provide fast feedback before manual production-clone testing.

This ticket implements Step 2.1 from `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` which defines the comprehensive testing approach needed to validate that the migration correctly handles the index size limit issue where previews exceeding 2704 bytes previously caused INSERT failures.

The quality strategy defines a three-level test pyramid:
- **L1 (SQL Syntax)**: Validates SQL correctness without execution
- **L2 (Empty Database)**: Verifies schema changes on clean database
- **L3 (Data Population)**: Tests real-world scenarios with varying preview sizes

## Acceptance Criteria
- [x] Test script created at `/workspace/crates/maproom/tests/test_index_migration.sh`
- [x] L1: SQL syntax validation executes and passes (9/9 checks)
- [x] L2: Empty database test creates indexes successfully (12/12 checks)
- [x] L3: Data population test handles small previews (<2000 bytes)
- [x] L3: Data population test handles large previews (>2704 bytes) without errors - CRITICAL TEST PASSED
- [x] All test levels execute with captured output
- [x] Script completes in <5 minutes (actual: 3 seconds)
- [x] Script exits with code 0 on success, non-zero on failure

## Technical Requirements
- Create bash test script with `set -e` for fail-fast behavior
- L1: Run `psql --dry-run` or syntax validation on migration SQL
- L2: Spin up test PostgreSQL container, run init.sql + migration, verify 3 indexes exist
- L3: Insert test data with varying preview sizes:
  - Small: 50-500 bytes (baseline - should work before and after migration)
  - Large: 2000-5000 bytes (critical test - would fail before migration)
  - Extreme: 10KB+ (edge case validation)
- Verify all INSERT statements succeed (the critical test - large previews should work now)
- Check query planner behavior with EXPLAIN for both small and large previews
- Include cleanup (docker rm -f test containers)
- Test script should be executable and include proper shebang
- All SQL queries should be parameterized or properly escaped
- Test output should be clear and diagnostic (show what's being tested at each step)

## Implementation Notes
Follow the test structure from `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` Step 2.1 (lines 154-219) and quality-strategy.md (lines 15-127). The critical validation is that INSERT of chunks with preview > 2704 bytes succeeds, which would have failed before this migration.

Use test data generation patterns from quality-strategy.md lines 336-377.

**Test Data Generation Approach**:
- Generate preview text deterministically (e.g., repeat "x" * N to reach target size)
- Include variety: code snippets, prose, special characters, Unicode
- Ensure byte counts are exact (important for boundary testing at 2704 bytes)

**Expected Behavior**:
- Before migration: INSERT with preview >2704 bytes fails with index row size error
- After migration: INSERT with preview of any size succeeds (preview is not indexed)
- Query performance should remain acceptable for both exact and fuzzy searches

**Container Management**:
- Use unique container names (e.g., `maproom-test-$$` with process ID)
- Always cleanup containers in trap handler (handles script interruption)
- Use PostgreSQL 15 or 16 (match production environment)
- Wait for PostgreSQL readiness before running tests (use pg_isready or connection polling)

**Success Criteria**:
The script should clearly indicate which test level passed/failed and provide enough diagnostic information to troubleshoot failures without requiring code reading.

## Dependencies
- IDXSIZE-1001 (migration SQL must exist at `/workspace/crates/maproom/migrations/003_index_size_limits.sql`)
- IDXSIZE-1004 (Phase 1 validation passed - confirms migration SQL is ready)
- PostgreSQL init.sql schema (at `/workspace/crates/maproom/migrations/001_init.sql` and `002_chunk_hashes.sql`)

## Risk Assessment
- **Risk**: Test script requires unavailable PostgreSQL tools
  - **Mitigation**: Use Docker containers for isolated testing; no host dependencies needed
- **Risk**: Tests hang or run too slow
  - **Mitigation**: Set timeouts on docker operations, use small dataset for L3 (10-20 test rows)
- **Risk**: Tests pass but miss edge cases
  - **Mitigation**: Include extreme preview sizes (10KB+) in test data, test exact boundary (2704 bytes)
- **Risk**: Test environment differs from production
  - **Mitigation**: Use same PostgreSQL version, same init.sql schema, same migration SQL

## Files/Packages Affected
- `/workspace/crates/maproom/tests/test_index_migration.sh` (new file)
- Uses temporary Docker containers (no persistent changes)
- Reads migration SQL from `/workspace/crates/maproom/migrations/003_index_size_limits.sql`
- Reads schema from `/workspace/crates/maproom/migrations/001_init.sql` and `002_chunk_hashes.sql`

## Planning References
- `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` - Step 2.1 (lines 149-224)
- `.crewchief/projects/IDXSIZE_index-size-limits/planning/quality-strategy.md` - Test pyramid L1-L3 (lines 14-127)

## Test Execution

**Test Execution Date**: 2025-11-09
**Test Script**: `/workspace/crates/maproom/tests/test_index_migration.sh`
**Overall Status**: ✅ ALL TESTS PASSED (30/30)
**Duration**: 3 seconds

### Test Results Summary

| Level | Test Category | Tests | Passed | Failed |
|-------|---------------|-------|--------|--------|
| L1 | SQL Syntax Validation | 9 | 9 | 0 |
| L2 | Empty Database Test | 12 | 12 | 0 |
| L3 | Data Population Test | 9 | 9 | 0 |
| **TOTAL** | | **30** | **30** | **0** |

### L1: SQL Syntax Validation (9/9 PASSED)

- ✅ Migration file exists: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- ✅ Migration uses safe DROP IF EXISTS pattern
- ✅ Migration uses CREATE INDEX CONCURRENTLY for non-blocking index creation
- ✅ Found idx_chunks_search_small_preview definition
- ✅ Found idx_chunks_search_basic definition
- ✅ Partial covering index uses INCLUDE clause for index-only scans
- ✅ Partial index correctly filters by preview size (WHERE LENGTH(preview) <= 2000)
- ✅ Migration includes ANALYZE for query planner statistics
- ✅ Migration sets statement timeout for safety

### L2: Empty Database Test (12/12 PASSED)

**Test Environment**:
- Container: `test_migration_0017_42222`
- PostgreSQL: `ankane/pgvector:v0.5.1`
- Database: `maproom`

**Tests**:
- ✅ PostgreSQL container started successfully
- ✅ PostgreSQL is ready (pg_isready check)
- ✅ Created maproom database
- ✅ Migration files copied to container
- ✅ Base schema initialized successfully (0001_init.sql)
- ✅ Old covering index created (simulating pre-migration state)
- ✅ Old covering index exists before migration (count: 1)
- ✅ Migration 0017 executed successfully
- ✅ Old covering index dropped successfully (count: 0)
- ✅ Partial covering index exists (idx_chunks_search_small_preview, count: 1)
- ✅ Basic fallback index exists (idx_chunks_search_basic, count: 1)
- ✅ Index comments exist (count: 1)

### L3: Data Population Test (9/9 PASSED) - CRITICAL

**Test Setup**:
- Test repository and worktree created (repo_id: 1, file_id: 1)

**Test Cases**:
1. ✅ **Small preview (500 bytes)**: INSERT succeeded
2. ✅ **Medium preview (2000 bytes)**: INSERT succeeded (boundary test)
3. ✅ **Large preview (3000 bytes)**: INSERT succeeded - **CRITICAL TEST PASSED**
   - This would FAIL before migration (exceeds 2704-byte B-tree limit)
   - Validates the core fix for index size limit errors
4. ✅ **Extreme preview (10KB)**: INSERT succeeded (edge case validated)
5. ✅ **All chunks inserted**: Verified all 4 test chunks exist in database
6. ✅ **Query planner (small previews)**: Uses sequential scan (expected for small tables)
7. ✅ **Query planner (large previews)**: Uses sequential scan (expected for small tables)
8. ✅ **Index statistics**: Retrieved successfully
9. ✅ **Cleanup**: Container removed successfully

**Index Usage Statistics**:
```
Index Name                      | Scans | Tuples Read | Tuples Fetched
idx_chunks_search_basic         |     0 |           0 |              0
idx_chunks_search_small_preview |     0 |           0 |              0
```
*Note: Seq scans used for small test dataset (<100 rows) - expected PostgreSQL behavior*

### Critical Validation Confirmed

**The Key Fix Works**: Chunks with preview text >2704 bytes now INSERT successfully. Before migration 0017, the old covering index (`idx_chunks_search_covering`) would fail with "index row size exceeds btree version 4 maximum 2704" error.

Migration 0017's two-index strategy eliminates this error:
- **Partial covering index** (`idx_chunks_search_small_preview`): For preview ≤ 2000 bytes (95%+ of data)
- **Basic fallback index** (`idx_chunks_search_basic`): For all preview sizes (100% of data)

### Test Script Features

- **Fail-fast**: Uses `set -euo pipefail` for immediate error detection
- **Automatic cleanup**: Trap handler ensures container cleanup on success/failure/interrupt
- **Unique naming**: Container name includes process ID to avoid conflicts
- **Non-conflicting port**: Uses 15432 instead of 5432
- **Color-coded output**: PASS (green), FAIL (red), WARN (yellow), INFO (blue)
- **Diagnostic**: Clear progress reporting at each test step
- **Duration**: Completes in ~3 seconds
- **Exit codes**: 0 on success, non-zero on failure
