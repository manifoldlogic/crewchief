# Ticket: DAEMIGR-3902: Stress Testing

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
Validate daemon-client stability under extreme load conditions beyond normal usage patterns. Ensure no memory leaks, crashes, or resource exhaustion under stress. Verify graceful degradation and recovery from crash scenarios.

## Background
This is Phase 3 validation work for the DAEMIGR project (Daemon Client Migration). After basic functionality (Phase 1) and performance optimization (Phase 2), we need to validate that the daemon client can handle extreme conditions that may occur in production:
- High request volumes (10,000+ sequential operations)
- Concurrent request bursts (1,000+ simultaneous)
- Daemon process crashes and recovery
- Resource pool exhaustion scenarios
- Sustained load over extended periods

This ticket implements the stress testing strategy defined in quality-strategy.md (lines 313-356) to ensure the system is production-ready.

## Acceptance Criteria
- [ ] 10,000 sequential requests complete without crash (heap growth < 50MB after gc)
- [ ] 1,000 concurrent requests complete successfully (some may queue, none fail)
- [ ] Daemon auto-restarts after crash (circuit breaker doesn't trigger inappropriately)
- [ ] Connection pool exhaustion handled gracefully (requests queue and complete when pool available)
- [ ] No resource leaks detected (connections released, memory freed, file handles closed)
- [ ] Crash during active request handled correctly (client receives error, daemon restarts cleanly)
- [ ] Sustained load test (1 hour, 100 req/min) shows stable resource usage (no drift in memory/connections)
- [ ] Circuit breaker triggers after rapid repeated crashes (5 crashes in short period)

## Technical Requirements
- Create comprehensive stress test suite at `/workspace/packages/daemon-client/tests/stress.test.ts`
- Test must measure and assert on:
  - Heap memory growth (using `process.memoryUsage()` before/after with gc)
  - CPU usage patterns
  - Connection pool utilization
  - Daemon restart count
  - Request success/failure rates
- Test scenarios must include:
  - **Sequential load**: 10k requests with heap monitoring
  - **Concurrent burst**: 1k simultaneous requests
  - **Crash recovery**: Kill daemon during request, verify error handling and auto-restart
  - **Pool saturation**: Spawn more concurrent requests than pool size, verify queueing behavior
  - **Sustained load**: 1 hour at 100 req/min with resource monitoring
  - **Circuit breaker**: Trigger 5 rapid crashes, verify circuit opens
- Use appropriate test timeouts for long-running tests
- Include clear assertions with meaningful error messages
- Use test hooks to properly clean up resources between scenarios

## Implementation Notes

### Memory Leak Detection
```typescript
// Force garbage collection before/after test
if (global.gc) {
  global.gc();
}
const heapBefore = process.memoryUsage().heapUsed;
// ... run 10k requests ...
if (global.gc) {
  global.gc();
}
const heapAfter = process.memoryUsage().heapUsed;
const growth = (heapAfter - heapBefore) / 1024 / 1024; // MB
expect(growth).toBeLessThan(50);
```

### Concurrent Request Burst
```typescript
// Create 1000 concurrent requests
const promises = Array.from({ length: 1000 }, () =>
  client.someOperation()
);
const results = await Promise.allSettled(promises);
const succeeded = results.filter(r => r.status === 'fulfilled').length;
expect(succeeded).toBe(1000);
```

### Crash Recovery Testing
```typescript
// Kill daemon process during active request
const requestPromise = client.someOperation();
// Wait a bit to ensure request is in flight
await sleep(100);
// Kill daemon process
daemonProcess.kill('SIGKILL');
// Verify client handles error gracefully
await expect(requestPromise).rejects.toThrow();
// Verify daemon restarts
await waitForDaemonReady();
```

### Pool Saturation
```typescript
// Assuming pool size is 10, spawn 50 concurrent requests
// Some should queue, all should eventually complete
const poolSize = 10;
const requestCount = poolSize * 5;
const startTime = Date.now();
const promises = Array.from({ length: requestCount }, () =>
  client.someOperation()
);
await Promise.all(promises);
const duration = Date.now() - startTime;
// Should take longer due to queueing (not parallel)
expect(duration).toBeGreaterThan(expectedParallelTime);
```

### Sustained Load Test
```typescript
// Run for 1 hour at 100 req/min
const durationMs = 60 * 60 * 1000; // 1 hour
const requestsPerMinute = 100;
const interval = 60000 / requestsPerMinute; // 600ms

const samples = [];
const endTime = Date.now() + durationMs;

while (Date.now() < endTime) {
  await client.someOperation();
  samples.push({
    heap: process.memoryUsage().heapUsed,
    connections: getConnectionCount(),
  });
  await sleep(interval);
}

// Verify no resource drift (linear regression slope near zero)
const heapTrend = calculateTrend(samples.map(s => s.heap));
expect(Math.abs(heapTrend)).toBeLessThan(threshold);
```

### Test Organization
- Use `describe` blocks to group scenarios
- Use `beforeEach`/`afterEach` for resource cleanup
- Set appropriate timeouts (e.g., `{ timeout: 60000 }` for 1-hour test)
- Consider using test flags to skip long-running tests in CI
- Log progress for long operations to prevent timeout confusion

## Dependencies
- DAEMIGR-3901 (Performance Testing) - Must complete first to establish baseline metrics
- Daemon client implementation from Phase 1
- Connection pool configuration from Phase 2

## Risk Assessment
- **Risk**: Long-running tests may timeout in CI environment
  - **Mitigation**: Use appropriate test timeouts, consider separating long tests with test flags, run locally or in dedicated stress test environment

- **Risk**: Stress tests may destabilize development environment
  - **Mitigation**: Use resource limits, run in isolated environment, include proper cleanup in afterEach hooks

- **Risk**: Memory leak detection may produce false positives due to GC timing
  - **Mitigation**: Force gc before measurements (require --expose-gc flag), use multiple samples, allow reasonable threshold (50MB for 10k requests)

- **Risk**: Crash recovery tests may leave orphaned processes
  - **Mitigation**: Track all spawned processes, use try/finally for cleanup, verify all processes terminated in afterAll hook

- **Risk**: 1-hour sustained test is impractical for regular CI runs
  - **Mitigation**: Make configurable via environment variable (e.g., STRESS_TEST_DURATION), default to shorter duration in CI (5 minutes), document how to run full suite locally

## Files/Packages Affected
- **Create**: `/workspace/packages/daemon-client/tests/stress.test.ts` - Comprehensive stress test suite
- **Modify**: `/workspace/packages/daemon-client/package.json` - May need to add test script with --expose-gc flag
- **Reference**: `/workspace/packages/daemon-client/tests/performance.test.ts` - From DAEMIGR-3901 for baseline comparison
