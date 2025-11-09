# Ticket: BLOBSHA-1901: Execute Phase 1 Test Suite

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute comprehensive test suite for Phase 1 (Blob SHA Foundation) to verify Rust implementation, PostgreSQL migration, and cross-implementation compatibility. Report all test results without modifying code.

## Background
This ticket implements Step 1.5 from the BLOBSHA project plan (planning/plan.md, lines 138-151). After implementing Rust blob SHA computation (BLOBSHA-1001) and database migration (BLOBSHA-1002), we must verify correctness before proceeding to Phase 2. The unit-test-runner agent executes tests and reports results, enabling implementation agents to fix any failures.

This verification ensures:
- Rust implementation correctly computes Git-compatible blob SHAs
- PostgreSQL migration successfully adds and populates blob_sha column
- Cross-implementation compatibility between Rust and SQL functions
- Foundation is solid before implementing deduplication logic in Phase 2

## Acceptance Criteria
- [x] All Rust unit tests pass:
  - `test_blob_sha_deterministic` - Same content produces same SHA
  - `test_blob_sha_git_compatibility` - Output matches Git hash-object
  - `test_blob_sha_different_content` - Different content produces different SHA
  - `test_blob_sha_whitespace_sensitive` - Whitespace changes SHA
  - `test_blob_sha_empty_content` - Handles empty strings
  - `test_blob_sha_unicode` - Handles Unicode correctly
- [x] Integration test passes:
  - `test_blob_sha_rust_sql_match` - PostgreSQL function matches Rust output
- [x] Migration test passes:
  - `test_migration_001_success` - Migration executes without errors
  - `test_migration_001_no_nulls` - All chunks have blob_sha values
  - `test_migration_001_dedup_metrics` - Deduplication percentage calculated
- [x] Test report generated with pass/fail status for each test
- [x] If any tests fail, detailed error output provided for debugging

## Technical Requirements
- Execute Rust tests via: `cd crates/maproom && cargo test content_hash`
- Execute integration tests via: `cd packages/maproom-mcp && npm test blob-sha-migration.test.ts`
- Generate test report showing:
  - Total tests run
  - Passed count
  - Failed count (with error messages)
  - Test execution time
- Use `--nocapture` flag for Rust tests to show all output
- Use `--verbose` flag for npm tests
- Execute tests in clean database state (migrations applied)
- Capture stdout/stderr for diagnostic purposes

## Implementation Notes
The unit-test-runner agent should NOT modify any code. This is a read-only verification step. If tests fail:

1. Report which specific tests failed
2. Include full error messages and stack traces
3. Indicate which acceptance criterion failed
4. Document expected vs actual behavior
5. Return ticket to implementation agents (rust-indexer-engineer or general-purpose) for fixes

Success criteria from planning/plan.md lines 148-151:
- All Phase 1 tests passing
- Migration script validated
- Metrics showing deduplication potential (should be 0% for initial scan, 70-90% for branch overlaps)

Reference test implementations:
- Unit tests: planning/quality-strategy.md lines 87-138
- Migration tests: planning/quality-strategy.md lines 233-268

**Test Execution Order**:
1. Rust unit tests first (verify core logic)
2. Integration tests second (verify Rust-SQL compatibility)
3. Migration tests last (verify schema changes and data migration)

**Reporting Format**:
```
=== RUST UNIT TESTS ===
✓ test_blob_sha_deterministic
✗ test_blob_sha_git_compatibility
  Error: assertion failed...

=== INTEGRATION TESTS ===
✓ test_blob_sha_rust_sql_match

=== MIGRATION TESTS ===
✗ test_migration_001_no_nulls
  Error: found 5 NULL values in blob_sha column

SUMMARY: 7/10 tests passed (3 failures)
```

## Dependencies
- BLOBSHA-1001 (Rust implementation must be complete)
- BLOBSHA-1002 (Migration must be complete and run)
- Test database available with migrated schema
- Maproom PostgreSQL container running (`maproom-postgres:5432`)
- Rust toolchain installed (cargo available)
- Node.js environment configured (npm available)

## Risk Assessment
- **Risk**: Tests pass but logic is incorrect (false positive)
  - **Mitigation**: Git compatibility test verifies against known-good implementation using `git hash-object`
- **Risk**: Flaky tests due to timing or database state
  - **Mitigation**: Run tests 3 times, require all passes for acceptance
- **Risk**: Environment-specific failures (local vs CI)
  - **Mitigation**: Document environment details (OS, Rust version, Node version) in test report
- **Risk**: Migration tests fail due to existing data
  - **Mitigation**: Tests should use isolated test database or transaction rollbacks

## Files/Packages Affected
- READ: `crates/maproom/src/content_hash.rs` (unit tests section)
- READ: `packages/maproom-mcp/tests/blob-sha-migration.test.ts` (integration tests)
- READ: Test database schema and data (for migration validation)
- READ: `crates/maproom/migrations/001_add_blob_sha.sql` (migration script being tested)
