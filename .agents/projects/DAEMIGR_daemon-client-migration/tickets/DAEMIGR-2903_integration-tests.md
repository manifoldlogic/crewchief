# Ticket: DAEMIGR-2003: Integration Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create end-to-end integration tests for MCP search via daemon using real daemon, real database, and real MCP code to validate daemon integration works correctly.

## Background
With daemon integration complete (DAEMIGR-2001, DAEMIGR-2002), we need comprehensive integration tests to validate the entire search flow works correctly: MCP tool → DaemonClient → Rust daemon → PostgreSQL → results back to MCP.

This is Phase 2 (Integration) testing, ensuring all components work together correctly. The tests must use real components (no mocking) to verify actual system behavior under various conditions including concurrent requests, error scenarios, and daemon lifecycle management.

Context references:
- Test file location: `/workspace/packages/maproom-mcp/tests/search-integration.test.ts`
- Quality requirements: `.agents/projects/DAEMIGR_daemon-client-migration/planning/quality-strategy.md` (lines 159-213)
- Architecture: End-to-end data flow in architecture.md (lines 502-549)

## Acceptance Criteria
- [ ] All integration tests pass (100% success rate)
- [ ] Real daemon, real database, real MCP code (no mocking of core components)
- [ ] Concurrent requests work correctly:
  - [ ] 10 concurrent searches complete without errors
  - [ ] 50 concurrent searches complete without errors
  - [ ] Response IDs match request IDs (no cross-contamination)
- [ ] Error handling verified:
  - [ ] Repo not found returns user-friendly error
  - [ ] Worktree not found returns error
  - [ ] Daemon crash triggers restart and retry
  - [ ] Daemon start failure returns clear error message

## Technical Requirements

### Test Setup
- PostgreSQL test database with sample indexed code
- Known test repository and worktree
- Predictable test queries with expected results
- Test environment variables:
  ```bash
  MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom_test
  ```

### Basic Search Tests
```typescript
it('should return search results via daemon', async () => {
  const results = await mcpSearchTool.execute({
    query: 'test function',
    repo: 'test-repo',
    limit: 10
  })
  expect(results).toHaveProperty('hits')
  expect(results.hits.length).toBeGreaterThan(0)
  expect(results.hits[0]).toHaveProperty('chunk_id')
})
```

### Daemon Lifecycle Tests
- First search starts daemon (verify process spawned)
- Subsequent searches reuse daemon (verify same PID)
- Daemon crash triggers restart (kill daemon, verify new PID)

### Concurrent Request Tests
- Spawn 10/50 searches in parallel with Promise.all()
- Verify all complete successfully
- Verify response IDs match request queries (no cross-contamination)

### Error Scenario Tests
- Invalid repo name → RpcError converted to MCP error
- Daemon binary not found → DaemonStartError with helpful message
- Database connection failure → error propagated correctly

## Implementation Notes

### Test Framework
Use Vitest as test framework (matches daemon-client tests)

### Test Database Setup
1. Create test repo "test-repo" with sample code
2. Index known functions/symbols for predictable results
3. Use test database URL (not production)
4. Consider creating helper: `/workspace/packages/maproom-mcp/tests/helpers/test-database.ts` (optional)

### Test Structure
```typescript
describe('MCP Search Integration', () => {
  beforeAll(async () => {
    // Setup test database with known data
    // Configure test environment variables
  })

  afterAll(async () => {
    // Stop daemon with closeDaemonClient()
    // Clean up test database data
  })

  describe('Basic Search', () => {
    // Basic search functionality tests
  })

  describe('Daemon Lifecycle', () => {
    // Daemon start, reuse, restart tests
  })

  describe('Concurrent Requests', () => {
    // 10 concurrent, 50 concurrent tests
  })

  describe('Error Handling', () => {
    // Error scenario tests
  })
})
```

### Cleanup Strategy
- Stop daemon with closeDaemonClient() in afterAll()
- Clean up test database data
- Prevent daemon process leaks with proper lifecycle management

### Reference Documentation
- See `.agents/projects/DAEMIGR_daemon-client-migration/planning/quality-strategy.md` for complete test case list
- Architecture documentation for data flow validation

## Dependencies
- DAEMIGR-2001 (MCP integration complete)
- DAEMIGR-2002 (singleton management complete)

## Risk Assessment
- **Risk**: Flaky tests due to real database latency
  - **Mitigation**: Generous timeouts, retry on transient failures
- **Risk**: Test data conflicts between test runs
  - **Mitigation**: Use unique test repo name, cleanup after tests
- **Risk**: Daemon process leaks during test failures
  - **Mitigation**: afterAll() hook calls closeDaemonClient(), ensure cleanup runs even on failure
- **Risk**: Concurrent test interference
  - **Mitigation**: Use isolated test data per test suite, proper test database isolation

## Files/Packages Affected
- Create: `/workspace/packages/maproom-mcp/tests/search-integration.test.ts`
- Create (optional): `/workspace/packages/maproom-mcp/tests/helpers/test-database.ts`
- Reference: `.agents/projects/DAEMIGR_daemon-client-migration/planning/quality-strategy.md`
- Reference: `/workspace/packages/maproom-mcp/src/tools/search.ts` (MCP tool being tested)
- Reference: `/workspace/packages/daemon-client/src/index.ts` (daemon client being tested)

## Estimated Effort
1 day (8 hours)

## Phase
2 (Integration)

## Priority
CRITICAL - Validates daemon integration works correctly
