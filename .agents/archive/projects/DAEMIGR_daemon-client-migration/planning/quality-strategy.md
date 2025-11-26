# DAEMIGR Quality Strategy

## Testing Philosophy

**Principle:** Tests should provide confidence, not ceremonial coverage.

**Focus Areas:**
1. **Correctness:** Core functionality works as specified
2. **Reliability:** System handles failures gracefully
3. **Performance:** Latency targets are met
4. **Integration:** Components work together correctly

**Non-Goals:**
- 100% line coverage (diminishing returns)
- Testing framework internals (trust dependencies)
- Over-specified behavior (brittle tests)

## Test Categories

### 1. Unit Tests (daemon-client package)

**Scope:** Individual modules in isolation

**Coverage Target:** > 80% for critical paths

#### `client.test.ts` - DaemonClient Core Logic

**Test Cases:**

```typescript
describe('DaemonClient', () => {
  describe('initialization', () => {
    it('should not start daemon on construction (lazy init)')
    it('should validate config on construction')
    it('should throw on invalid binary path')
  })

  describe('lifecycle', () => {
    it('should start daemon on first search request')
    it('should reuse daemon for subsequent requests')
    it('should restart daemon on crash')
    it('should stop daemon gracefully on client.stop()')
    it('should kill daemon after shutdown timeout')
    it('should cleanup resources on stop (streams, listeners)')
  })

  describe('request handling', () => {
    it('should generate sequential request IDs (1, 2, 3...)')
    it('should match responses to requests by ID')
    it('should handle concurrent requests correctly')
    it('should timeout on slow responses')
    it('should reject on daemon crash during request')
  })

  describe('health checking', () => {
    it('should ping daemon before request (if configured)')
    it('should restart daemon on failed health check')
    it('should skip health check if recently successful')
  })

  describe('error handling', () => {
    it('should throw DaemonStartError on spawn failure')
    it('should throw RpcError on daemon error response')
    it('should throw RpcTimeoutError on request timeout')
    it('should propagate daemon stderr to logger')
  })
})
```

**Mocking Strategy:**
- Mock `child_process.spawn` (control daemon behavior)
- Mock process stdin/stdout (simulate RPC)
- Mock timers (test timeouts without delay)

#### `lifecycle.test.ts` - Process Lifecycle Management

**Test Cases:**

```typescript
describe('DaemonLifecycle', () => {
  describe('start', () => {
    it('should spawn daemon with correct args and env')
    it('should wait for daemon readiness (first stdout)')
    it('should timeout if daemon does not respond')
    it('should throw on spawn error (ENOENT, EACCES)')
  })

  describe('stop', () => {
    it('should send SIGTERM for graceful shutdown')
    it('should wait up to shutdownTimeout for exit')
    it('should send SIGKILL if still running after timeout')
    it('should close streams and remove listeners')
    it('should be idempotent (multiple calls safe)')
  })

  describe('restart', () => {
    it('should stop old daemon before starting new')
    it('should increment restart attempt counter')
    it('should record last restart time')
    it('should return new process instance')
  })

  describe('crash recovery', () => {
    it('should detect crash via exit event')
    it('should restart if attempts < maxAttempts')
    it('should apply exponential backoff (1s, 2s, 4s...)')
    it('should not restart if attempts >= maxAttempts')
    it('should reset attempts after success window (60s)')
  })
})
```

**Edge Cases:**
- Daemon exits immediately (exit code 1)
- Daemon killed by signal (SIGKILL, SIGSEGV)
- Multiple crashes in quick succession
- Restart during shutdown (race condition)

#### `rpc.test.ts` - JSON-RPC Protocol Handling

**Test Cases:**

```typescript
describe('RpcProtocol', () => {
  describe('createRequest', () => {
    it('should create valid JSON-RPC 2.0 request')
    it('should include method, params, id')
    it('should set jsonrpc field to "2.0"')
  })

  describe('parseResponse', () => {
    it('should parse valid success response')
    it('should parse valid error response')
    it('should detect malformed JSON')
    it('should detect missing required fields')
    it('should detect invalid jsonrpc version')
  })

  describe('isError', () => {
    it('should return true if response has error field')
    it('should return false if response has result field')
    it('should handle null id (notification)')
  })

  describe('createError', () => {
    it('should create valid JSON-RPC error object')
    it('should include code, message, optional data')
  })
})
```

**Invalid Input Cases:**
- Malformed JSON (parse error)
- Missing `jsonrpc` field
- Missing `id` field
- Both `result` and `error` present
- Neither `result` nor `error` present

### 2. Integration Tests (maproom-mcp package)

**Scope:** End-to-end search via daemon

**Coverage Target:** All critical user flows

#### `search-integration.test.ts` - MCP Search Tool

**Test Cases:**

```typescript
describe('MCP Search Tool (with daemon)', () => {
  describe('basic search', () => {
    it('should return search results via daemon')
    it('should include chunk IDs in results')
    it('should respect limit parameter')
    it('should filter by repo and worktree')
  })

  describe('daemon lifecycle', () => {
    it('should start daemon on first search')
    it('should reuse daemon for subsequent searches')
    it('should restart daemon if crashed')
  })

  describe('concurrent requests', () => {
    it('should handle 10 concurrent searches correctly')
    it('should handle 50 concurrent searches correctly')
    it('should not mix up responses (ID matching)')
  })

  describe('error scenarios', () => {
    it('should return error if repo not found')
    it('should return error if worktree not found')
    it('should return error if query invalid')
    it('should return error if daemon crashes during search')
    it('should return error if daemon fails to start')
  })

  describe('timeout handling', () => {
    it('should timeout on slow daemon (> 30s)')
    it('should restart daemon after timeout')
  })
})
```

**Test Environment:**
- Real PostgreSQL database (test data)
- Real Rust daemon binary
- Real MCP server code (not mocked)

**Data Setup:**
- Test repository with known chunks
- Test queries with predictable results
- Test edge cases (empty repo, large result set)

### 3. Performance Tests

**Scope:** Latency and resource usage

**Coverage Target:** Key performance metrics

#### `performance.test.ts` - Latency Benchmarks

**Test Cases:**

```typescript
describe('Performance', () => {
  describe('latency', () => {
    it('should complete cold start in < 600ms', async () => {
      const start = Date.now()
      await daemon.search({ query: 'test', repo: 'crewchief' })
      const latency = Date.now() - start
      expect(latency).toBeLessThan(600)
    })

    it('should complete warm requests in < 60ms', async () => {
      // Prime the daemon
      await daemon.search({ query: 'test', repo: 'crewchief' })

      // Measure warm request
      const start = Date.now()
      await daemon.search({ query: 'test', repo: 'crewchief' })
      const latency = Date.now() - start
      expect(latency).toBeLessThan(60)
    })

    it('should maintain < 60ms for 100 sequential requests')
    it('should maintain < 100ms for 50 concurrent requests')
  })

  describe('resource usage', () => {
    it('should not leak memory over 1000 requests', async () => {
      // Force GC before baseline measurement
      if (global.gc) {
        global.gc()
      }
      await new Promise(resolve => setTimeout(resolve, 100)) // Allow GC to complete

      const initialMem = process.memoryUsage().heapUsed

      for (let i = 0; i < 1000; i++) {
        await daemon.search({ query: `test ${i}`, repo: 'crewchief' })
      }

      // Force GC before final measurement
      if (global.gc) {
        global.gc()
      }
      await new Promise(resolve => setTimeout(resolve, 100)) // Allow GC to complete

      const finalMem = process.memoryUsage().heapUsed
      const growth = finalMem - initialMem
      expect(growth).toBeLessThan(10 * 1024 * 1024) // < 10MB
    })

    it('should not leak file descriptors over 1000 requests')
    it('should not leak database connections')
  })

  describe('throughput', () => {
    it('should achieve > 50 req/s for concurrent load')
  })
})
```

**Measurement Tools:**
- `Date.now()` for latency
- `process.memoryUsage()` for memory (run tests with `node --expose-gc` to enable forced GC)
- `lsof` for file descriptors (shell command)
- PostgreSQL `pg_stat_activity` for connections

**Memory Leak Detection Methodology:**

To ensure accurate memory leak detection, tests must force garbage collection before measurements:

```bash
# Run tests with GC exposed
node --expose-gc node_modules/.bin/vitest run performance.test.ts
```

```typescript
// In test code
if (global.gc) {
  global.gc()  // Force garbage collection
  await new Promise(resolve => setTimeout(resolve, 100))  // Wait for GC
}
```

**Why force GC?**
- Node.js garbage collection is non-deterministic
- Without forced GC, unreachable objects may still count toward heap usage
- Forcing GC ensures we measure true leaked objects, not GC-eligible garbage
- More accurate leak detection (fewer false positives)

#### `stress.test.ts` - Stress Testing

**Test Cases:**

```typescript
describe('Stress Testing', () => {
  it('should survive 10,000 sequential requests')
  it('should survive 1,000 concurrent requests')
  it('should recover from 10 daemon crashes')
  it('should handle rapid start/stop cycles (100x)')

  describe('connection pool exhaustion', () => {
    it('should queue requests when pool exhausted', async () => {
      // Spawn more concurrent requests than pool size (default: 5)
      const promises = []
      for (let i = 0; i < 10; i++) {
        promises.push(daemon.search({ query: `test ${i}`, repo: 'crewchief' }))
      }

      // All should complete (some queued, none crashed)
      const results = await Promise.all(promises)
      expect(results).toHaveLength(10)
    })

    it('should timeout if pool exhausted beyond timeout period', async () => {
      // Create slow query that holds connection
      // Then spawn more requests than pool size
      // Verify some timeout gracefully (no crash)
    })

    it('should not crash on pool exhaustion', async () => {
      // Verify daemon stays healthy even when pool is fully utilized
    })
  })
})
```

**Failure Modes:**
- Daemon crashes under load
- Request queue overflow
- Database connection pool exhaustion (requests queue or timeout, no crashes)
- Memory pressure (OOM)

### 4. Regression Tests

**Scope:** Ensure existing MCP functionality unchanged

**Coverage Target:** All existing search scenarios

#### `regression.test.ts` - Backward Compatibility

**Test Cases:**

```typescript
describe('Regression (MCP Search)', () => {
  it('should return identical results to spawning approach')
  it('should support all search modes (fts, vector, hybrid)')
  it('should support all filters (repo, worktree, file_type)')
  it('should support debug mode')
  it('should handle large result sets (1000+ hits)')
  it('should handle empty result sets')
  it('should handle malformed queries gracefully')
})
```

**Comparison Strategy:**
- Run same queries via daemon and spawning
- Compare result sets (order-independent)
- Verify chunk IDs match
- Verify metadata matches (scores, metadata)

## Test Infrastructure

### Test Fixtures

**Database:**
```typescript
// tests/fixtures/db-setup.ts
export async function setupTestDatabase() {
  await client.query('DROP SCHEMA IF EXISTS maproom_test CASCADE')
  await client.query('CREATE SCHEMA maproom_test')
  // ... load test schema, seed data
}

export async function teardownTestDatabase() {
  await client.query('DROP SCHEMA maproom_test CASCADE')
}
```

**Test Data:**
- 3 repositories (small, medium, large)
- 100 chunks per repository (varied content)
- Known queries with predictable results

**Binary Discovery:**
```typescript
// tests/fixtures/binary-path.ts
export function getTestBinaryPath(): string {
  const platform = process.platform
  const binDir = path.join(__dirname, '../../packages/cli/bin', platform)
  const binary = path.join(binDir, 'crewchief-maproom')
  if (!fs.existsSync(binary)) {
    throw new Error(`Test binary not found: ${binary}`)
  }
  return binary
}
```

### Mock Strategies

**When to Mock:**
- Unit tests (isolate component under test)
- Fast feedback (avoid slow operations)
- Controlled behavior (simulate failures)

**When NOT to Mock:**
- Integration tests (test real interactions)
- Performance tests (measure real latency)
- Regression tests (verify actual behavior)

**Mocking Tools:**
- `vitest.mock()` for module mocking
- `vitest.spyOn()` for function spying
- `vitest.useFakeTimers()` for timer control

### CI/CD Integration

**GitHub Actions Workflow:**

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_DB: maproom_test
          POSTGRES_USER: maproom
          POSTGRES_PASSWORD: maproom
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v3
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      # Build Rust daemon
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --release --bin crewchief-maproom
      - run: cp target/release/crewchief-maproom packages/cli/bin/linux/

      # Install and test
      - run: pnpm install
      - run: pnpm test
      - run: pnpm test:integration
      - run: pnpm test:performance
```

**Test Stages:**
1. Unit tests (fast, no DB)
2. Integration tests (slower, requires DB and daemon)
3. Performance tests (slowest, measures latency)

**Failure Handling:**
- Fast fail (stop on first failure)
- Retry flaky tests (max 3 attempts)
- Upload test artifacts (logs, core dumps)

## Quality Gates

### Pre-Merge Requirements

**Required:**
- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ No regression in existing tests
- ✅ Code coverage > 80% for new code

**Recommended:**
- ✅ Performance tests within targets
- ✅ No memory leaks detected
- ✅ Linter and formatter pass

**Blocked By:**
- ❌ Any test failure
- ❌ Coverage regression (drop > 5%)
- ❌ Performance regression (latency > 2x)

### Post-Merge Monitoring

**Metrics to Track:**
- Daemon restart rate (target < 1%)
- Average search latency (target < 50ms warm)
- Error rate (target < 0.1%)
- Memory usage (target < 100MB)

**Alerting:**
- Restart rate > 5% (warning)
- Restart rate > 10% (critical)
- Error rate > 1% (warning)
- Memory growth > 20MB/day (warning)

## Risk-Based Testing

### Critical Paths (Must Test)

1. **Happy Path Search**
   - Priority: CRITICAL
   - Risk: High (core functionality)
   - Coverage: Unit + Integration + Performance

2. **Daemon Crash Recovery**
   - Priority: CRITICAL
   - Risk: Medium (auto-restart should handle)
   - Coverage: Unit + Integration + Stress

3. **Concurrent Request Handling**
   - Priority: HIGH
   - Risk: Medium (ID matching, race conditions)
   - Coverage: Integration + Performance

4. **Timeout Handling**
   - Priority: HIGH
   - Risk: Medium (hung daemon detection)
   - Coverage: Unit + Integration

### Medium Priority (Should Test)

1. **Health Checking**
   - Priority: MEDIUM
   - Risk: Low (nice-to-have, not critical)
   - Coverage: Unit

2. **Graceful Shutdown**
   - Priority: MEDIUM
   - Risk: Low (resource cleanup)
   - Coverage: Unit + Integration

3. **Error Message Quality**
   - Priority: MEDIUM
   - Risk: Low (UX, not correctness)
   - Coverage: Integration

### Low Priority (Optional)

1. **Log Output Formatting**
   - Priority: LOW
   - Risk: Very Low (observability, not correctness)
   - Coverage: Manual verification

2. **Configuration Validation**
   - Priority: LOW
   - Risk: Very Low (fail fast on invalid config)
   - Coverage: Unit

## Test Maintenance

### Keeping Tests Valuable

**Principles:**
- Delete tests that don't catch bugs
- Merge duplicate test cases
- Refactor brittle tests (over-specified)
- Update tests when behavior changes

**Red Flags:**
- Flaky tests (intermittent failures)
- Slow tests (> 1s for unit tests)
- Brittle tests (break on refactoring)
- Duplicate tests (same behavior, different names)

**Actions:**
- Fix flaky tests or delete
- Optimize slow tests or move to integration suite
- Simplify brittle tests (test behavior, not implementation)
- Merge duplicate tests

### Test Ownership

**Responsibility:**
- Feature author writes tests for new code
- Reviewer ensures adequate test coverage
- Maintainer refactors tests with code

**Coverage Reviews:**
- Weekly coverage reports (track trends)
- Pre-release coverage verification
- Post-incident test additions (prevent regressions)

## Debugging Failed Tests

### Local Debugging

**Enable Verbose Logging:**
```bash
RUST_LOG=debug pnpm test:integration
```

**Run Single Test:**
```bash
pnpm vitest run --reporter=verbose client.test.ts
```

**Inspect Daemon Logs:**
```bash
tail -f /tmp/maproom-daemon-*.log
```

### CI Debugging

**Download Artifacts:**
- Test logs (stdout/stderr)
- Daemon logs (RUST_LOG output)
- Core dumps (if daemon crashed)

**Reproduce Locally:**
```bash
# Use same environment as CI
docker run -it --rm \
  -v $(pwd):/workspace \
  -e MAPROOM_DATABASE_URL=... \
  node:18 \
  bash -c "cd /workspace && pnpm install && pnpm test"
```

## Acceptance Criteria

**Project is ready to ship when:**

1. **Correctness**
   - ✅ All unit tests pass (> 80% coverage)
   - ✅ All integration tests pass (100% pass rate)
   - ✅ All regression tests pass (no functionality lost)

2. **Performance**
   - ✅ Cold start < 600ms (acceptable)
   - ✅ Warm requests < 60ms (20x improvement)
   - ✅ No memory leaks (1000 requests stable)

3. **Reliability**
   - ✅ Auto-restart works (daemon crashes handled)
   - ✅ Circuit breaker prevents loops (max 5 restarts)
   - ✅ Graceful shutdown works (no zombie processes)

4. **Integration**
   - ✅ MCP search tool uses daemon (spawning replaced)
   - ✅ Chunk ID fetching works (DB queries succeed)
   - ✅ Error messages actionable (users can debug)

5. **Documentation**
   - ✅ API docs complete (daemon-client README)
   - ✅ Integration guide complete (MCP migration)
   - ✅ Troubleshooting guide complete (common issues)

---

**Quality Strategy Defined:** 2025-11-22
**Status:** Ready for implementation
