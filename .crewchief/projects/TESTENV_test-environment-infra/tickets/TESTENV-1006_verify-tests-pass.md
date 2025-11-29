# Ticket: TESTENV-1006: Verify all fixture-compatible tests pass

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: This IS the test verification ticket - must run and document test results.

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute the complete test suite and verify that all 397 integration tests pass using the new fixture-based approach. Document test results and performance metrics.

## Background
This ticket is the Phase 1 verification gate. All previous Phase 1 work (corpus design, fixture generation, test setup integration, test updates) culminates here. Success means all tests pass without requiring the Rust daemon for standard test runs.

Reference: [plan.md](../planning/plan.md) - Phase 1, Success Criteria

## Acceptance Criteria
- [ ] Run `pnpm test` in `packages/maproom-mcp` - all tests pass
- [ ] Test count is 397 (or matches current expected count)
- [ ] Test suite completes in <30 seconds
- [ ] Fixture load time logged and is <50ms
- [ ] No daemon process is spawned during test run
- [ ] CI pipeline passes (if accessible)
- [ ] Document test results with output

## Technical Requirements

### Test Execution
```bash
cd packages/maproom-mcp

# Ensure clean state
docker compose -p crewchief-dev-env down
docker compose -p crewchief-dev-env up -d postgres-test

# Run tests with timing
time pnpm test

# Expected output:
# ✓ All tests pass
# Tests: 397 passed
# Time: <30s
```

### Verification Checklist

| Check | Command | Expected Result |
|-------|---------|-----------------|
| Tests pass | `pnpm test` | 0 failures |
| Test count | Check vitest output | ~397 tests |
| Duration | Check vitest output | <30 seconds |
| Fixture load | Check console output | "Fixtures loaded in Xms" where X < 50 |
| No daemon | `ps aux \| grep crewchief-maproom` | No running processes |

### Performance Metrics to Capture

Document the following in the ticket completion:
```
Test Results:
- Total tests: [X]
- Passed: [X]
- Failed: [X]
- Skipped: [X]
- Duration: [X]s

Fixture Performance:
- Load time: [X]ms
- Chunk count: [X]

Environment:
- Node version: [X]
- pnpm version: [X]
- Docker version: [X]
```

## Implementation Notes

1. **Run tests multiple times** - Verify no flakiness (run 3x minimum)

2. **Check for daemon spawning** - Grep logs for daemon-related messages

3. **Compare with baseline** - Previous state was 392 passing, 5 failing

4. **Document any skipped tests** - If tests are skipped, explain why

5. **CI verification** - If CI is accessible, trigger a test run there too

6. **Clean environment** - Start fresh to ensure reproducibility:
   ```bash
   # Full reset
   docker compose -p crewchief-dev-env down -v
   docker compose -p crewchief-dev-env up -d postgres-test
   pnpm test
   ```

## Dependencies
- TESTENV-1001 (test corpus)
- TESTENV-1002 (fixture script)
- TESTENV-1003 (generated fixtures)
- TESTENV-1004 (fixture integration)
- TESTENV-1005 (test updates)

## Risk Assessment
- **Risk**: Some tests still fail
  - **Mitigation**: Debug and fix; may need to revisit previous tickets
- **Risk**: Flaky tests
  - **Mitigation**: Run multiple times; identify and fix flaky tests
- **Risk**: Performance regression
  - **Mitigation**: If tests take >30s, investigate slow tests

## Files/Packages Affected
- No code changes in this ticket (verification only)
- May update test files if issues discovered
