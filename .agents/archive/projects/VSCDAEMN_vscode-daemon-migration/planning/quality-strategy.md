# VSCDAEMN Quality Strategy

## Testing Philosophy

**MVP Mindset**: Tests prevent rework, not ceremonial checkboxes. Focus on confidence over coverage.

### Critical Paths to Test

1. **Daemon Integration** (Unit Tests)
   - DaemonClient singleton creation
   - Configuration passing (binary path, env vars)
   - Graceful shutdown on extension deactivation

2. **Scan Operation** (Integration Tests)
   - Full scan via daemon (real binary, real database)
   - Progress callback invocation
   - Error handling (daemon crash, timeout, database unavailable)

3. **VSCode UX** (Integration Tests)
   - Progress notification displays correctly
   - Status bar updates on completion
   - Error messages user-friendly

4. **Regression** (Backward Compatibility)
   - Scan produces same results as spawning pattern
   - All existing commands still work (setup, restart watchers, show status)
   - No breaking changes to user workflows

## Test Strategy

### Unit Tests (> 80% coverage)

**Target Files**:
- `src/daemon/index.ts` - Daemon singleton management
- `src/process/scan.ts` - Scan operation with daemon

**Test Cases**:
```typescript
describe('getDaemonClient', () => {
  it('creates singleton daemon with correct config')
  it('reuses existing daemon on subsequent calls')
  it('passes environment variables correctly')
  it('uses correct binary path from extension root')
})

describe('runInitialScan', () => {
  it('calls daemon.scan() with workspace path')
  it('reports progress via VSCode progress API')
  it('updates status bar on completion')
  it('handles daemon start failure gracefully')
  it('handles scan timeout with error message')
  it('handles daemon crash during scan')
})

describe('shutdownDaemon', () => {
  it('stops daemon gracefully')
  it('cleans up resources')
  it('handles shutdown timeout')
})
```

### Integration Tests (E2E)

**Test Cases**:
```typescript
describe('Scan Integration', () => {
  it('scans small workspace successfully', async () => {
    // Real database, real daemon, small test repo
    const result = await runInitialScan(config)
    expect(result.filesProcessed).toBeGreaterThan(0)
  })
  
  it('handles concurrent scans gracefully', async () => {
    // Simulate user triggering scan multiple times
    const scans = [runInitialScan(config), runInitialScan(config)]
    await expect(Promise.all(scans)).resolves.toBeTruthy()
  })
  
  it('recovers from daemon crash mid-scan', async () => {
    // Kill daemon during scan, verify auto-restart
  })
})
```

### Regression Tests

**Test Cases**:
```typescript
describe('Backward Compatibility', () => {
  it('scan results match spawning pattern', async () => {
    const daemonResult = await scanWithDaemon(workspace)
    const spawnResult = await scanWithSpawning(workspace)
    expect(daemonResult.chunks).toEqual(spawnResult.chunks)
  })
  
  it('all existing commands still work', async () => {
    await vscode.commands.executeCommand('maproom.setup')
    await vscode.commands.executeCommand('maproom.restartWatchers')
    await vscode.commands.executeCommand('maproom.showStatus')
  })
})
```

## Risk Mitigation Through Testing

| Risk | Test Type | Coverage |
|------|-----------|----------|
| Daemon fails to start | Unit + Integration | `daemon.start()` throws, fallback tested |
| Progress events lost | Integration | Progress callback invoked correctly |
| Concurrent scans | Integration | Multiple scans handled gracefully |
| Database unavailable | Integration | postgres-checker prevents daemon start |
| Extension deactivation leak | Unit | Daemon shutdown verified |

## Quality Gates

### PR Requirements
- ✅ All unit tests pass (> 80% coverage)
- ✅ Integration tests pass (full scan E2E)
- ✅ Regression tests pass (no breaking changes)
- ✅ TypeScript compilation passes (no errors)
- ✅ ESLint passes (no errors or warnings)

### Release Requirements
- ✅ Manual testing on 3 platforms (macOS, Linux, Windows)
- ✅ Large repository scan completes successfully (>10K files)
- ✅ Daemon auto-restart verified (crash recovery works)
- ✅ User-facing documentation updated

## Test Coverage Targets

**Package-Level Coverage**:
- `packages/vscode-maproom/`: > 80% (increased from current ~70%)
- `packages/daemon-client/`: > 82% (already achieved in DAEMIGR)

**Critical Paths**: 100% coverage
- Daemon singleton management
- Scan operation
- Extension lifecycle (activate, deactivate)

**Nice to Have**: 50-80% coverage
- UI components (status bar, progress)
- Setup wizard (user interaction heavy)

## Performance Benchmarks

**Metrics to Track**:
- Cold scan latency (first scan): < 300ms
- Warm scan latency (re-scan): < 100ms
- Extension activation time: < 500ms (no regression)
- Memory usage: < 150MB baseline (daemon + extension)

**Performance Tests**:
```typescript
describe('Performance', () => {
  it('cold scan completes in < 300ms', async () => {
    const start = performance.now()
    await daemon.scan(params)
    const elapsed = performance.now() - start
    expect(elapsed).toBeLessThan(300)
  })
  
  it('warm scan completes in < 100ms', async () => {
    await daemon.scan(params) // Prime
    const start = performance.now()
    await daemon.scan(params)
    const elapsed = performance.now() - start
    expect(elapsed).toBeLessThan(100)
  })
})
```

## Conclusion

Quality strategy focuses on **confidence over coverage**, with targeted testing of critical paths (daemon integration, scan operation, error handling) and comprehensive E2E tests to prevent regressions.
