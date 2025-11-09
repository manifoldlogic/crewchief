# Ticket: SCHMAFIX-3901: Run Migration Integration Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- This ticket IS the test execution - the primary agent runs tests and reports results
- "Tests pass" checkbox is checked when all 4 integration tests pass (4/4 passing)
- If ANY test fails, this checkbox remains unchecked until tests are fixed and re-run

## Agents
- unit-test-runner (primary - executes tests, reports results only)
- verify-ticket
- commit-ticket

## Summary
Execute the Rust integration tests created in SCHMAFIX-3001 using the unit-test-runner agent. Report test results and confirm all migration scenarios pass without errors. This is a critical quality gate before proceeding with MCP integration (SCHMAFIX-4001) or manual validation (SCHMAFIX-5001).

## Background
After integrating BLOBSHA/BRANCHX migration SQL files (SCHMAFIX-1001), updating the Rust migration runner (SCHMAFIX-2001), and writing comprehensive integration tests (SCHMAFIX-3001), we must now execute those tests to verify that migrations work correctly at the Rust level.

This ticket is a quality gate - we cannot proceed with MCP integration or manual validation until we confirm that:
1. Fresh database migrations work (all 20 migrations applied cleanly)
2. Incremental migration works (v0.17 → v0.21 upgrade path)
3. Migration idempotency works (migrations can safely run twice)
4. Schema validation works (expected tables and columns exist)

The unit-test-runner agent will execute tests and provide a clear pass/fail report WITHOUT attempting to fix any failures. If tests fail, we must return to the appropriate ticket to fix the underlying issue.

Reference: `.agents/projects/SCHMAFIX_schema-migration-integration/planning/plan.md` - Phase 3, "Blocking Dependencies" section

## Acceptance Criteria
- [ ] Command `cargo test migration_integration` executes successfully in `crates/maproom/` directory
- [ ] Test `test_fresh_database_migrations` passes - confirms all 20 migrations apply to empty database
- [ ] Test `test_incremental_migration` passes - confirms v0.17 → v0.21 upgrade path works
- [ ] Test `test_migration_idempotency` passes - confirms migrations can run twice safely
- [ ] Test `test_schema_validation` passes - confirms blob_sha and code_embeddings tables exist with correct schema
- [ ] No panics, no database connection errors, no migration failures in test output
- [ ] Test report confirms 100% pass rate (4 passed; 0 failed; 0 ignored)

## Technical Requirements
- **Test command**: `cargo test migration_integration --test migration_integration`
- **Test location**: `crates/maproom/tests/migration_integration.rs`
- **Database**: Tests should use testcontainers (auto-provisioned PostgreSQL)
- **Environment**: Tests need DATABASE_URL or testcontainer configuration
- **Output requirement**: unit-test-runner agent provides clear pass/fail report
- **Code modification**: NOT ALLOWED - this is observation and reporting only

## Implementation Notes

### Test Execution Process
1. Navigate to `crates/maproom/` directory
2. Run `cargo test migration_integration`
3. Observe test output
4. Report results in clear format:
   - Which tests passed
   - Which tests failed (if any)
   - Error messages for failures
   - Environment issues (containers, database, etc.)

### Expected Success Output
```
running 4 tests
test test_fresh_database_migrations ... ok
test test_incremental_migration ... ok
test test_migration_idempotency ... ok
test test_schema_validation ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Failure Protocol
If ANY tests fail:
1. unit-test-runner reports failure details:
   - Which test failed
   - Error message
   - Stack trace (if applicable)
2. Do NOT attempt to fix code
3. Escalate to appropriate agent based on failure type:
   - Test code issue → return to SCHMAFIX-3001 (rust-indexer-engineer fixes tests)
   - Migration SQL issue → return to SCHMAFIX-1001 (rust-indexer-engineer fixes migrations)
   - Runner code issue → return to SCHMAFIX-2001 (rust-indexer-engineer fixes runner)
4. Ticket remains incomplete until tests pass

### Agent Responsibilities
**unit-test-runner agent MUST**:
- Execute tests and capture output
- Report results clearly (pass/fail for each test)
- Identify environment issues (container startup, database connection)
- Document any unexpected behavior

**unit-test-runner agent MUST NOT**:
- Modify test code
- Modify migration SQL files
- Modify runner code
- Attempt to fix failures
- Mark ticket complete if any test fails

## Dependencies
- **SCHMAFIX-1001** (BLOCKER) - Migration SQL files must exist in `crates/maproom/migrations/`
- **SCHMAFIX-2001** (BLOCKER) - Rust migration runner must be updated to v0.21.0
- **SCHMAFIX-3001** (BLOCKER) - Integration tests must be written and present

## Risk Assessment
- **Risk**: Tests fail due to missing pgvector extension in testcontainer
  - **Mitigation**: Report to user, may need Docker image change to `pgvector/pgvector:pg16`

- **Risk**: Testcontainers doesn't start (Docker not available, permissions issue)
  - **Mitigation**: Report environment issue, may need manual postgres setup or Docker configuration

- **Risk**: Migration 0018 backfill fails (INSERT INTO code_embeddings FROM blob_sha)
  - **Mitigation**: Report to rust-indexer-engineer to fix SQL in SCHMAFIX-1001

- **Risk**: Test hangs or times out
  - **Mitigation**: Report timeout, may indicate database connection issue or infinite loop in migration

## Files/Packages Affected
**Files to Reference** (no modifications):
- `crates/maproom/tests/migration_integration.rs` - Integration test suite created in SCHMAFIX-3001
- `crates/maproom/Cargo.toml` - Verify test dependencies (testcontainers, tokio)
- `crates/maproom/migrations/*.sql` - Migration files being tested
- `crates/maproom/src/migrations.rs` - Migration runner being tested

**No files modified in this ticket** - this is test execution and reporting only.

## Related Planning Documents
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/plan.md` (Phase 3 - Migration Testing)
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/quality-strategy.md` (Critical Path Tests section)

## Estimated Effort
15-30 minutes (test execution and reporting only)

## Success Definition
All 4 integration tests pass with 0 failures. The test report shows:
- `test_fresh_database_migrations ... ok`
- `test_incremental_migration ... ok`
- `test_migration_idempotency ... ok`
- `test_schema_validation ... ok`
- `test result: ok. 4 passed; 0 failed`

If ANY test fails, this ticket is NOT complete until the underlying issue is fixed (in the appropriate ticket) and tests are re-run successfully.
