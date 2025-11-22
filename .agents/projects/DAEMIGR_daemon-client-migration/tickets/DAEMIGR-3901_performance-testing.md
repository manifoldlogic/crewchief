# Ticket: DAEMIGR-3901: Performance Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
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
Validate latency targets, resource usage, and connection pool behavior through comprehensive performance benchmarks to ensure 20-50x improvement is realized.

## Background
The primary goal of daemon migration is performance improvement (160-400ms → 10-50ms for warm requests). This ticket validates we meet all performance targets: cold start <600ms, warm requests <60ms, throughput >50 req/s, no memory leaks, and graceful pool exhaustion handling.

This ticket implements the performance testing strategy outlined in `.agents/projects/DAEMIGR_daemon-client-migration/planning/quality-strategy.md` (lines 215-291) and validates the success metrics from `.agents/projects/DAEMIGR_daemon-client-migration/README.md` (lines 117-121).

## Acceptance Criteria
- [ ] Cold start latency < 600ms (first request spawns daemon, connects to DB, executes query)
- [ ] Warm request latency < 60ms median (subsequent requests reuse daemon and pool)
- [ ] Throughput > 50 req/s for concurrent load
- [ ] No memory leaks: Heap growth < 10MB over 1000 requests (measured with forced gc() before/after)
- [ ] Connection pool exhaustion handled gracefully:
  - [ ] Requests queue when pool exhausted (concurrent > pool_size)
  - [ ] Requests timeout if pool held too long (no crashes)
  - [ ] Daemon stays healthy under pool pressure
- [ ] Connection pool sizing documented (formula: pool_size >= concurrent/2)

## Technical Requirements

### Latency Benchmarks

Implement cold start and warm request latency tests:

```typescript
describe('Performance', () => {
  describe('latency', () => {
    it('cold start < 600ms', async () => {
      const start = Date.now()
      await daemon.search({ query: 'test', repo: 'crewchief' })
      expect(Date.now() - start).toBeLessThan(600)
    })

    it('warm requests < 60ms median', async () => {
      // Warmup request
      await daemon.search({ query: 'warmup', repo: 'crewchief' })

      // Measure 100 requests for stable median
      const latencies = []
      for (let i = 0; i < 100; i++) {
        const start = Date.now()
        await daemon.search({ query: `test ${i}`, repo: 'crewchief' })
        latencies.push(Date.now() - start)
      }

      // Calculate median (50th percentile)
      const sorted = latencies.sort((a, b) => a - b)
      const median = sorted[50]
      expect(median).toBeLessThan(60)
    })
  })
})
```

### Memory Leak Test

Implement memory leak detection with forced garbage collection:

```typescript
it('no memory leaks over 1000 requests', async () => {
  // Force GC before baseline measurement
  if (global.gc) {
    global.gc()
  }
  await new Promise(r => setTimeout(r, 100)) // Allow GC to complete

  const initialMem = process.memoryUsage().heapUsed

  // Execute 1000 requests
  for (let i = 0; i < 1000; i++) {
    await daemon.search({ query: `test ${i}`, repo: 'crewchief' })
  }

  // Force GC before final measurement
  if (global.gc) {
    global.gc()
  }
  await new Promise(r => setTimeout(r, 100)) // Allow GC to complete

  const finalMem = process.memoryUsage().heapUsed
  const growth = finalMem - initialMem

  // Assert < 10MB growth
  expect(growth).toBeLessThan(10 * 1024 * 1024)
})
```

**Run with**: `node --expose-gc node_modules/.bin/vitest run performance.test.ts`

### Connection Pool Tests

Validate graceful handling of pool exhaustion:

```typescript
describe('connection pool', () => {
  it('handles pool exhaustion gracefully', async () => {
    // Default pool_size = 5, spawn 10 concurrent requests
    const promises = []
    for (let i = 0; i < 10; i++) {
      promises.push(daemon.search({ query: `test ${i}`, repo: 'crewchief' }))
    }

    // All should complete (some queue, none crash)
    const results = await Promise.all(promises)
    expect(results).toHaveLength(10)
    results.forEach(result => {
      expect(result).toBeDefined()
    })
  })

  it('queues requests when pool exhausted', async () => {
    // Verify requests queue and complete when connections available
    // Pool pressure should not cause failures
  })
})
```

### Throughput Test

Measure requests per second under concurrent load:

```typescript
describe('throughput', () => {
  it('achieves > 50 req/s for concurrent load', async () => {
    const numRequests = 100
    const start = Date.now()

    // Spawn 100 concurrent requests
    const promises = []
    for (let i = 0; i < numRequests; i++) {
      promises.push(daemon.search({ query: `test ${i}`, repo: 'crewchief' }))
    }

    await Promise.all(promises)
    const elapsed = (Date.now() - start) / 1000 // Convert to seconds

    const throughput = numRequests / elapsed
    expect(throughput).toBeGreaterThan(50)
  })
})
```

### Pool Sizing Documentation

Document connection pool sizing recommendations in test comments:

```typescript
/**
 * Connection Pool Sizing Recommendations
 *
 * Formula: pool_size >= concurrent_requests / 2
 *
 * Examples:
 * - 10 concurrent requests: pool_size >= 5
 * - 20 concurrent requests: pool_size >= 10
 * - 50 concurrent requests: pool_size >= 25
 *
 * Pool exhaustion behavior:
 * - Requests queue when all connections busy
 * - Queued requests wait for available connection
 * - Daemon remains healthy under pool pressure
 * - No crashes or connection leaks
 */
```

## Implementation Notes

1. **Run with GC exposed**: Use `node --expose-gc` for memory leak test to enable forced garbage collection
2. **Real database required**: Tests must use actual database and daemon, not mocks
3. **Millisecond precision**: Use `Date.now()` for millisecond-level latency measurement
4. **Multiple iterations**: Run 100+ iterations for stable median calculation
5. **Warm cache first**: For latency tests, run warmup request to prime daemon and connection pool
6. **Pool sizing formula**: Document `pool_size >= concurrent/2` in test comments
7. **GC methodology**: Reference quality-strategy.md lines 291-312 for forced GC rationale

**Why force GC?**
- Node.js garbage collection is non-deterministic
- Without forced GC, unreachable objects may still count toward heap usage
- Forcing GC ensures we measure true leaked objects, not GC-eligible garbage
- More accurate leak detection (fewer false positives)

## Dependencies
- DAEMIGR-2903 (integration tests pass) - Must complete before performance testing

## Risk Assessment
- **Risk**: Database latency affecting benchmarks
  - **Mitigation**: Run on local DB, warm cache with initial request, use median instead of mean
- **Risk**: GC pauses skewing latency measurements
  - **Mitigation**: Use median instead of mean, force gc() before tests, measure over many iterations
- **Risk**: Pool exhaustion causing test failures
  - **Mitigation**: Expected behavior - verify graceful handling, not absence of queuing
- **Risk**: Non-deterministic timing in CI environment
  - **Mitigation**: Use percentiles (median) for stability, allow warmup period

## Files/Packages Affected
- **Create**: `/workspace/packages/daemon-client/tests/performance.test.ts` - Performance test suite
- **Reference**: `.agents/projects/DAEMIGR_daemon-client-migration/planning/quality-strategy.md` - Testing methodology
- **Reference**: `.agents/projects/DAEMIGR_daemon-client-migration/README.md` - Success metrics

## Estimated Effort
1 day (8 hours) - Benchmark implementation, multiple test runs, result analysis

## Phase
3 (Validation)

## Priority
HIGH - Validates core performance improvements (20-50x latency reduction)
