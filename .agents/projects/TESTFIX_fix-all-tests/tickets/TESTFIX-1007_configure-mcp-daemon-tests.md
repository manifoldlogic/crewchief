# Ticket: TESTFIX-1007: Configure MCP and Daemon-Client Tests

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
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Document and configure test commands for MCP and daemon-client packages that have special requirements. MCP requires PostgreSQL for full tests; daemon-client performance tests require the daemon binary.

## Background
The MCP package (`packages/maproom-mcp`) and daemon-client package (`packages/daemon-client`) have tests with external dependencies. MCP's `pnpm test` runs `run-blob-sha-tests.cjs` which requires PostgreSQL (`maproom-postgres:5432`). Daemon-client has 42 passing unit tests but 5 failing performance tests that require a running daemon binary. This ticket ensures local-safe test commands are documented and CI handles full test execution. This is Phase 1 (ticket 7) of the TESTFIX project - TypeScript test configuration for packages with external dependencies.

## Acceptance Criteria
- [ ] MCP `pnpm test:connection` passes locally (no database required)
- [ ] MCP `pnpm test:sqlite` passes locally (SQLite fixture tests)
- [ ] Daemon-client unit tests pass: `pnpm test` runs 42 passing tests
- [ ] Performance test skip behavior documented (or tests conditionally skip)
- [ ] README or CLAUDE.md updated with local test commands
- [ ] CI continues to run full test suite with database

## Technical Requirements

**MCP Package:**
- Verify `test:connection` script exists and works without database
- Verify `test:sqlite` script exists for SQLite fixture tests
- Document that `pnpm test` requires PostgreSQL (CI-only)
- Add conditional skip for database tests if running locally

**Daemon-Client Package:**
- Verify unit tests (rpc.test.ts, errors.test.ts) pass: 42 tests
- Document that performance tests (performance.test.ts) require daemon binary
- Consider adding skip condition if daemon not available
- Ensure CI builds daemon before running performance tests

**Documentation:**
- Update package README files with local test commands
- Add note to project CLAUDE.md about test requirements

## Implementation Notes

1. **For MCP:**
   - Run `cd packages/maproom-mcp && pnpm test:connection` to verify
   - Run `cd packages/maproom-mcp && pnpm test:sqlite` to verify
   - Check package.json for test script definitions
   - If scripts don't exist, create them

2. **For daemon-client:**
   - Run `cd packages/daemon-client && pnpm test` to see current state
   - Identify which tests fail and why
   - Add conditional skip logic if daemon binary not found:
   ```typescript
   const daemonAvailable = existsSync(DAEMON_BINARY_PATH)
   describe.skipIf(!daemonAvailable)('performance tests', () => { ... })
   ```

3. **Documentation updates:**
   - Add "Testing" section to package READMEs
   - Document local vs CI test differences
   - Clarify which tests require external dependencies

## Dependencies
- TESTFIX-1001 (clean environment)
- TESTFIX-1002 (baseline documented)

## Risk Assessment
- **Risk**: Test scripts may not exist
  - **Mitigation**: Create them if needed; they're simple script aliases

- **Risk**: CI may break if we change test behavior
  - **Mitigation**: Only add skip conditions, don't change default test command

- **Risk**: Performance tests may be important to run
  - **Mitigation**: Document clearly that CI runs them; local skip is just for convenience

## Files/Packages Affected
- `packages/maproom-mcp/package.json` (verify/add test scripts)
- `packages/maproom-mcp/README.md` (add testing documentation)
- `packages/daemon-client/package.json` (verify test scripts)
- `packages/daemon-client/README.md` (add testing documentation)
- `packages/daemon-client/tests/performance.test.ts` (add skip condition if needed)
