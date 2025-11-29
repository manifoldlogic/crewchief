# Ticket: TESTFIX-1004: Run and Verify Rust Tests

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
- unit-test-runner
- rust-indexer-engineer (if fixes needed)
- verify-ticket
- commit-ticket

## Summary
Execute the Rust test suite after compilation fixes and fix any runtime failures. Verify SQLite tests pass locally and PostgreSQL tests compile for CI.

## Background
After TESTFIX-1003 fixes all compilation errors, tests need to be executed to verify they actually pass. Some tests may compile but fail at runtime due to logic errors, missing test fixtures, or environment requirements. This is Phase 3 of the TESTFIX project - Rust test execution. This ticket implements the test execution and verification phase described in the TESTFIX project plan, ensuring all Rust tests are functional and pass locally.

## Acceptance Criteria
- [ ] `cargo test --features sqlite` passes (all SQLite tests green)
- [ ] `cargo check --tests --features postgres` compiles successfully
- [ ] Any runtime failures are fixed (not just skipped)
- [ ] Test output shows pass counts matching expected coverage

## Technical Requirements

**SQLite Tests (Primary - Must Pass):**
```bash
cargo test --features sqlite
```
- All unit tests must pass
- All integration tests must pass
- No test should be `#[ignore]`d without documented reason

**PostgreSQL Tests (Compilation Only):**
```bash
cargo check --tests --features postgres
```
- Tests must compile (runtime requires PostgreSQL connection)
- CI handles actual PostgreSQL test execution

**If runtime failures occur:**
- Fix logic errors in test assertions
- Add missing test fixtures or setup
- Document any tests requiring external dependencies
- Do NOT skip tests without justification

## Implementation Notes

1. **Run SQLite tests with output capture:**
   ```bash
   cargo test --features sqlite 2>&1 | tee /tmp/rust-test-output.txt
   ```

2. **Analyze failures by category:**
   - **Assertion failures**: Logic issues in test expectations or implementation
   - **Setup failures**: Missing test fixtures, database schema, or configuration
   - **Timeout failures**: Performance issues or blocking operations
   - **Panic failures**: Unwrap/expect calls on None/Err values

3. **Fix each category systematically:**
   - Address assertion failures first (these indicate test/code logic bugs)
   - Add missing test fixtures to `tests/fixtures/`
   - Adjust timeouts for legitimate long-running operations
   - Replace unsafe unwraps with proper error handling

4. **Re-run tests after each fix batch** to verify fixes don't introduce new failures

5. **Final verification:**
   - All SQLite tests pass
   - PostgreSQL tests compile: `cargo check --tests --features postgres`
   - Document test output showing pass counts

## Dependencies
- TESTFIX-1003 (Rust tests must compile before they can be executed)

## Risk Assessment

- **Risk**: Tests require external services (Ollama, databases)
  - **Mitigation**: SQLite tests should be self-contained; document any external dependencies clearly. PostgreSQL tests only need to compile locally (CI runs them with real database).

- **Risk**: Flaky tests due to timing or race conditions
  - **Mitigation**: Add appropriate timeouts; use deterministic test data; avoid time-based assertions where possible.

- **Risk**: Tests fail due to missing test fixtures
  - **Mitigation**: Check `tests/fixtures/` directory; create missing fixtures if needed; ensure fixture paths are correct for test execution context.

- **Risk**: Tests uncover real bugs in production code
  - **Mitigation**: This is actually desirable! Fix production code bugs exposed by tests - don't modify tests to pass broken code.

## Files/Packages Affected
- `crates/maproom/tests/` (all test files - may need runtime fixes for assertions/setup)
- `crates/maproom/tests/fixtures/` (may need new test fixtures or data files)
- `crates/maproom/src/` (only if production code has bugs exposed by tests - fix the bugs, not the tests)
