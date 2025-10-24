# Ticket: MCP_CORE-2001: End-to-End Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- mcp-tools-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement comprehensive end-to-end testing suite for the MCP server implementation, covering complete workflow scenarios, error handling, performance benchmarks, and load testing. This Phase 2 task ensures the MCP tools work correctly in real-world usage patterns and meet performance requirements.

## Background
With individual MCP tools implemented in Phase 1 (context, open, upsert, explain), we need to validate that they work correctly together in real-world workflows. E2E testing will verify:
- Complete user workflows (search → open → context chains)
- Error handling across tool boundaries
- Performance characteristics under various loads
- Database connection pooling and resource management
- All Zod schema validations

This testing phase is critical before moving to Phase 3 (client libraries) and ensures the MCP server is production-ready.

## Acceptance Criteria
- [ ] E2E workflow tests passing for all tool combinations (search → open, search → context, full chains)
- [ ] All error scenarios documented and tested (invalid params, missing data, database errors, timeouts)
- [ ] Performance benchmarks documented with p50, p95, p99 response times (target <100ms p95)
- [ ] Load testing completed with concurrent request scenarios (10, 50, 100 concurrent clients)
- [ ] All Zod schema validations tested with valid and invalid inputs
- [ ] Database connection pooling tested under load
- [ ] Test coverage report generated showing >80% coverage for tool implementations

## Technical Requirements
- Use Vitest as testing framework (consistent with codebase)
- Implement realistic workflow scenarios matching actual usage patterns
- Test all error paths defined in ErrorHandler (ValidationError, DatabaseError, timeout errors)
- Benchmark tool response times: search <50ms, open <30ms, context <100ms, upsert variable, explain <200ms
- Load test with concurrent requests: 10, 50, 100 concurrent clients
- Validate all Zod schemas from architecture (SearchSchema, ContextSchema, OpenSchema, UpsertSchema, ExplainSchema)
- Test database connection pooling (pool_size: 10, connection_timeout: 5000ms)
- Mock external dependencies (filesystem, git commands) for deterministic testing
- Generate performance reports with charts/graphs for analysis

## Implementation Notes

### Test Structure
```
packages/maproom-mcp/tests/e2e/
├── workflow_test.ts          # Complete workflow scenarios
├── error_scenarios_test.ts   # Error handling tests
├── performance_test.ts       # Response time benchmarks
└── load_test.ts              # Concurrent request testing
```

### Workflow Tests (workflow_test.ts)
Test complete user workflows:
1. **Search → Open workflow**: Search for code → open specific file
2. **Search → Context workflow**: Search for symbol → get full context
3. **Search → Open → Context chain**: Full exploration workflow
4. **Upsert → Search workflow**: Index new files → verify searchable
5. **Explain workflow**: Get symbol card for chunk

### Error Scenario Tests (error_scenarios_test.ts)
Cover all error paths from architecture:
- Invalid parameters (empty query, negative k, invalid scope)
- Missing data (non-existent chunk_id, invalid relpath)
- Database errors (connection failure, timeout, query errors)
- Timeout scenarios (slow queries, large file reads)
- Validation errors (Zod schema violations)
- Rate limiting errors
- Path traversal attempts (security)

### Performance Benchmarks (performance_test.ts)
Measure and document:
- Search tool: p50, p95, p99 response times (target <50ms p95)
- Context tool: p50, p95, p99 (target <100ms p95)
- Open tool: p50, p95, p99 (target <30ms p95)
- Upsert tool: throughput (files/sec, chunks/sec)
- Explain tool: p50, p95, p99 (target <200ms p95 if enabled)

Generate report with:
- Response time distributions
- Throughput metrics
- Resource usage (memory, CPU)
- Database query performance

### Load Testing (load_test.ts)
Test concurrent scenarios:
- 10 concurrent clients (baseline)
- 50 concurrent clients (moderate load)
- 100 concurrent clients (high load)

Measure:
- Response time degradation under load
- Error rates
- Database connection pool saturation
- Memory/resource leaks
- Request queuing behavior

### Key Test Data
- Use realistic code samples from codebase
- Test with various file sizes (small: <1KB, medium: 1-10KB, large: >10KB)
- Test with different languages (TypeScript, JavaScript, Rust, Markdown)
- Include edge cases (empty files, binary files, very long lines)

### Architecture References
All tool implementations are defined in `/workspace/crewchief_context/maproom/MCP_CORE/MCP_CORE_ARCHITECTURE.md`:
- Search tool (lines 37-64)
- Context tool (lines 67-78)
- Open tool (lines 81-104)
- Upsert tool (lines 106-127)
- Explain tool (lines 129-148)
- Error handling (lines 166-192)

## Dependencies
- **MCP_CORE-1001**: Context tool implementation (required for context workflow tests)
- **MCP_CORE-1002**: Open tool implementation (required for open workflow tests)
- **MCP_CORE-1003**: Upsert tool implementation (required for indexing tests)
- **MCP_CORE-1004**: Explain tool implementation (required for explain workflow tests)
- **Vitest testing framework**: Already available in codebase
- **Database test fixtures**: Test database with sample indexed data

## Risk Assessment
- **Risk**: Performance benchmarks may reveal slow queries requiring optimization
  - **Mitigation**: Identify slow queries early, add database indexes, optimize query plans before Phase 3

- **Risk**: Load testing may uncover connection pool exhaustion
  - **Mitigation**: Test various pool sizes, implement proper connection error handling, add connection pool monitoring

- **Risk**: E2E tests may be flaky due to timing issues or external dependencies
  - **Mitigation**: Mock external dependencies (filesystem, git), use deterministic test data, implement proper test isolation

- **Risk**: Test execution time may be long with comprehensive load testing
  - **Mitigation**: Separate load tests from unit tests, run load tests in CI only on main branch, implement parallel test execution

## Files/Packages Affected
- `packages/maproom-mcp/tests/e2e/workflow_test.ts` (new)
- `packages/maproom-mcp/tests/e2e/error_scenarios_test.ts` (new)
- `packages/maproom-mcp/tests/e2e/performance_test.ts` (new)
- `packages/maproom-mcp/tests/e2e/load_test.ts` (new)
- `packages/maproom-mcp/tests/fixtures/` (new - test data fixtures)
- `packages/maproom-mcp/tests/helpers/` (new - test utilities)
- `packages/maproom-mcp/package.json` (update test scripts)
- `packages/maproom-mcp/vitest.config.ts` (configure e2e test suite)
