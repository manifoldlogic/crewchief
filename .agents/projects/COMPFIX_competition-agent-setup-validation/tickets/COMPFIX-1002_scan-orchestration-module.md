# Ticket: COMPFIX-1002: Scan Orchestration Module

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

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

Create a scan orchestration module that ensures all variant worktrees are indexed by maproom before agent execution. This module will manage sequential scanning with progress tracking, parse scan results, and fail fast on any scan errors to prevent agents from running in unindexed environments.

## Background

The competition framework creates variant worktrees but never scans them for indexing. This causes:
- 0% search tool usage (agents can't search unindexed worktrees)
- Agents receive empty search results (no chunks in database)
- Competition results are meaningless (all variants fail equally)

Each variant worktree must be scanned before its agent is spawned. Scanning is fast due to embedding reuse from the base branch (~5-15 seconds per worktree), making sequential scanning acceptable for up to 12 variants (~2-3 minutes total overhead).

This ticket implements the scanning orchestration that prepares all variant environments for agent execution.

**Reference:** Section "Scan Orchestration Module" in `planning/architecture.md` (lines 206-309)

## Acceptance Criteria

- [x] `ScanOrchestrator` module created in `packages/cli/src/search-optimization/scan-orchestrator.ts`
- [x] `scanWorktree(config)` function scans a single worktree and returns detailed results
- [x] `scanAllWorktrees(configs)` function scans multiple worktrees sequentially with progress logging
- [x] `waitForScanCompletion(scanId, timeout)` function polls for scan completion (if async scanning added later)
- [x] All subprocess execution uses `spawn()` with args array (NOT `execSync` with string interpolation - command injection protection)
- [x] Scan output parsed for chunk count and duration
- [x] Clear progress logging shows: current worktree, chunk count, duration
- [x] Errors captured with full context (command, output, exit code)
- [x] Fail-fast behavior: throw error immediately if any scan fails
- [x] Unit tests achieve 95%+ coverage with mocked spawn
- [x] Test cases cover: successful scan, scan failure, invalid output, timeout

## Technical Requirements

### Module Structure

```typescript
// packages/cli/src/search-optimization/scan-orchestrator.ts

export interface ScanConfig {
  worktreePath: string
  repo: string
  worktree: string
  commit: string
  baseDir: string
}

export interface ScanResult {
  success: boolean
  worktree: string
  chunkCount: number
  durationMs: number
  error?: string
}

export async function scanWorktree(config: ScanConfig): Promise<ScanResult>
export async function scanAllWorktrees(configs: ScanConfig[]): Promise<ScanResult[]>
export async function waitForScanCompletion(scanId: string, timeout?: number): Promise<void>
```

### Single Worktree Scan

```typescript
export async function scanWorktree(config: ScanConfig): Promise<ScanResult> {
  const startTime = Date.now()

  console.log(`📊 Scanning worktree: ${config.worktree}`)
  console.log(`   Path: ${config.worktreePath}`)

  try {
    // CRITICAL: Use spawn with args array (command injection protection)
    const proc = spawn('crewchief-maproom', [
      'scan',
      '--repo', config.repo,
      '--worktree', config.worktree,
      '--commit', config.commit,
      '--root', config.worktreePath
    ], {
      stdio: 'pipe',
      shell: false  // NEVER use shell for security
    })

    // Collect output
    let stdout = ''
    let stderr = ''
    proc.stdout.on('data', (data) => { stdout += data.toString() })
    proc.stderr.on('data', (data) => { stderr += data.toString() })

    // Wait for completion
    const exitCode = await new Promise<number>((resolve) => {
      proc.on('close', resolve)
    })

    if (exitCode !== 0) {
      throw new Error(`Scan failed with code ${exitCode}: ${stderr}`)
    }

    // Parse output for chunk count
    const match = stdout.match(/Total chunks: (\d+)/)
    const chunkCount = match ? parseInt(match[1]) : 0

    const durationMs = Date.now() - startTime

    console.log(`   ✅ Scan complete: ${chunkCount} chunks in ${durationMs}ms`)

    return {
      success: true,
      worktree: config.worktree,
      chunkCount,
      durationMs
    }
  } catch (error) {
    const durationMs = Date.now() - startTime
    console.error(`   ❌ Scan failed: ${error.message}`)

    return {
      success: false,
      worktree: config.worktree,
      chunkCount: 0,
      durationMs,
      error: error.message
    }
  }
}
```

### Batch Worktree Scanning

```typescript
export async function scanAllWorktrees(configs: ScanConfig[]): Promise<ScanResult[]> {
  const results: ScanResult[] = []

  console.log(`\n📊 Scanning ${configs.length} worktrees...`)
  console.log('='.repeat(60))

  // Scan sequentially (fast due to embedding reuse)
  for (const config of configs) {
    const result = await scanWorktree(config)
    results.push(result)

    // Fail fast on errors
    if (!result.success) {
      throw new Error(`Scan failed for ${config.worktree}: ${result.error}`)
    }
  }

  const totalDuration = results.reduce((sum, r) => sum + r.durationMs, 0)
  const totalChunks = results.reduce((sum, r) => sum + r.chunkCount, 0)

  console.log('='.repeat(60))
  console.log(`✅ All scans complete in ${(totalDuration / 1000).toFixed(1)}s`)
  console.log(`📊 Total chunks indexed: ${totalChunks}`)
  console.log()

  return results
}
```

### Wait for Scan Completion (Future)

```typescript
export async function waitForScanCompletion(
  scanId: string,
  timeout: number = 60000
): Promise<void> {
  // Placeholder for async scanning support
  // Currently all scans are synchronous
  // This function enables future parallel scanning optimization

  const startTime = Date.now()

  while (Date.now() - startTime < timeout) {
    // Check scan status via database or status file
    const status = await checkScanStatus(scanId)

    if (status === 'complete') return
    if (status === 'failed') throw new Error('Scan failed')

    await new Promise(resolve => setTimeout(resolve, 1000))
  }

  throw new Error(`Scan timeout after ${timeout}ms`)
}

// Helper for future use
async function checkScanStatus(scanId: string): Promise<'pending' | 'complete' | 'failed'> {
  // Query database or check status file
  // Not implemented in MVP (all scans are synchronous)
  throw new Error('Async scanning not implemented')
}
```

## Implementation Notes

### Command Injection Protection

**CRITICAL SECURITY REQUIREMENT:** All subprocess execution MUST use `spawn()` with args array.

**UNSAFE (vulnerable to command injection):**
```typescript
// DON'T DO THIS
execSync(`crewchief-maproom scan --repo ${repo} --worktree ${worktree}`)
```

**SAFE (command injection protected):**
```typescript
// DO THIS
spawn('crewchief-maproom', [
  'scan',
  '--repo', repo,
  '--worktree', worktree
], {
  shell: false  // Critical: no shell interpretation
})
```

This protection is documented in `planning/security-review.md` (lines 146-200) and is a HIGH priority security control.

### Output Parsing

Maproom scan output format (example):
```
Scanning directory: /workspace/.crewchief/worktrees/variant-a
Processing files: 234
Generating embeddings: 156 (78 reused)
Total chunks: 567
```

Parse regex: `/Total chunks: (\d+)/`

### Error Handling

**Capture full error context:**
- Command executed (sanitized, no sensitive data)
- Exit code
- stdout (last 500 chars)
- stderr (last 500 chars)
- Duration before failure

**Example error message:**
```
❌ Scan failed for variant-a-detailed

Command: crewchief-maproom scan --repo crewchief --worktree variant-a-detailed
Exit code: 1
Duration: 2.3s

Error output:
  Permission denied: cannot read directory /workspace/.crewchief/worktrees/variant-a-detailed

Troubleshooting:
- Check directory exists and is readable
- Verify crewchief-maproom binary is in PATH
- Check database write permissions
```

### Testing Strategy

Create `packages/cli/src/search-optimization/scan-orchestrator.test.ts`:

```typescript
describe('ScanOrchestrator', () => {
  describe('scanWorktree', () => {
    it('returns success when scan completes', async () => {
      const mockSpawn = jest.spyOn(child_process, 'spawn')
      mockSpawn.mockImplementation(() => {
        const proc = new EventEmitter()
        proc.stdout = new EventEmitter()
        proc.stderr = new EventEmitter()

        setTimeout(() => {
          proc.stdout.emit('data', 'Total chunks: 123')
          proc.emit('close', 0)
        }, 10)

        return proc
      })

      const result = await scanWorktree({
        worktreePath: '/tmp/test',
        repo: 'crewchief',
        worktree: 'test',
        commit: 'abc123',
        baseDir: '/tmp'
      })

      expect(result.success).toBe(true)
      expect(result.chunkCount).toBe(123)
      mockSpawn.mockRestore()
    })

    it('returns failure when scan command fails', async () => {
      const mockSpawn = jest.spyOn(child_process, 'spawn')
      mockSpawn.mockImplementation(() => {
        const proc = new EventEmitter()
        proc.stdout = new EventEmitter()
        proc.stderr = new EventEmitter()

        setTimeout(() => {
          proc.stderr.emit('data', 'Permission denied')
          proc.emit('close', 1)
        }, 10)

        return proc
      })

      const result = await scanWorktree({
        worktreePath: '/tmp/test',
        repo: 'crewchief',
        worktree: 'test',
        commit: 'abc123',
        baseDir: '/tmp'
      })

      expect(result.success).toBe(false)
      expect(result.error).toContain('Permission denied')
      mockSpawn.mockRestore()
    })

    it('uses spawn with args array (not shell)', async () => {
      const mockSpawn = jest.spyOn(child_process, 'spawn')
      mockSpawn.mockImplementation(() => createMockProcess(0, 'Total chunks: 100'))

      await scanWorktree({
        worktreePath: '/tmp/test',
        repo: 'test',
        worktree: 'variant-a',
        commit: '123',
        baseDir: '/tmp'
      })

      expect(mockSpawn).toHaveBeenCalledWith(
        'crewchief-maproom',
        expect.arrayContaining(['scan', '--repo', 'test']),
        expect.objectContaining({ shell: false })
      )
      mockSpawn.mockRestore()
    })
  })

  describe('scanAllWorktrees', () => {
    it('scans all worktrees sequentially', async () => {
      const mockScan = jest.fn()
        .mockResolvedValueOnce({ success: true, chunkCount: 100, durationMs: 5000 })
        .mockResolvedValueOnce({ success: true, chunkCount: 150, durationMs: 6000 })

      const configs = [
        { worktreePath: '/tmp/a', repo: 'test', worktree: 'a', commit: '123', baseDir: '/tmp' },
        { worktreePath: '/tmp/b', repo: 'test', worktree: 'b', commit: '456', baseDir: '/tmp' }
      ]

      // Mock scanWorktree temporarily
      const originalScan = scanWorktree
      scanWorktree = mockScan

      const results = await scanAllWorktrees(configs)

      expect(mockScan).toHaveBeenCalledTimes(2)
      expect(results).toHaveLength(2)
      expect(results.every(r => r.success)).toBe(true)

      scanWorktree = originalScan
    })

    it('throws error if any scan fails', async () => {
      const mockScan = jest.fn()
        .mockResolvedValueOnce({ success: true, chunkCount: 100 })
        .mockResolvedValueOnce({ success: false, error: 'Scan failed' })

      const configs = [
        { worktreePath: '/tmp/a', repo: 'test', worktree: 'a', commit: '123', baseDir: '/tmp' },
        { worktreePath: '/tmp/b', repo: 'test', worktree: 'b', commit: '456', baseDir: '/tmp' }
      ]

      const originalScan = scanWorktree
      scanWorktree = mockScan

      await expect(scanAllWorktrees(configs))
        .rejects
        .toThrow('Scan failed')

      scanWorktree = originalScan
    })
  })
})
```

**Coverage targets:**
- `scanWorktree`: 95% (main flows + error cases)
- `scanAllWorktrees`: 95% (sequential execution + early failure)
- `waitForScanCompletion`: 50% (future enhancement, minimal testing)

### Performance Expectations

**For 12 variants (ultra configuration):**
- First variant scan: 10-15s (some new embeddings)
- Subsequent scans: 5-10s each (embedding reuse)
- Total sequential time: ~2-3 minutes
- Acceptable overhead for 100% success rate

**Why sequential vs parallel?**
- Simpler implementation (no race conditions)
- Minimal benefit (10-15s saved per variant)
- Database handles sequential writes better
- Setup time isn't the bottleneck (agent execution is)

See `planning/architecture.md` (lines 534-558) for detailed performance analysis.

## Dependencies

- **Prerequisite tickets:** None (independent module)

- **External dependencies:**
  - `crewchief-maproom` binary in PATH
  - PostgreSQL running and accessible
  - Node.js `child_process` module

- **Internal dependencies:**
  - Type definitions for `ScanConfig`, `ScanResult`

- **Blocks:**
  - COMPFIX-1003 (Enhanced Competition Runner) - needs scan orchestration

## Risk Assessment

- **Risk**: Maproom binary not in PATH
  - **Mitigation**: Check binary exists before scan, clear error message

- **Risk**: Scan takes too long (>30s per worktree)
  - **Mitigation**: Log warning if scan exceeds threshold, continue anyway
  - **Note**: Typical scans are 5-15s due to embedding reuse

- **Risk**: Command injection via variant names
  - **Mitigation**: Use spawn with args array (HIGH priority security control)

- **Risk**: Stdout/stderr parsing fails on format change
  - **Mitigation**: Fallback to checking database for chunk count

## Files/Packages Affected

**New files:**
- `packages/cli/src/search-optimization/scan-orchestrator.ts`
- `packages/cli/src/search-optimization/scan-orchestrator.test.ts`

**No modifications to existing files** - this is a new module

**Dependencies:**
- `child_process` (Node.js built-in)
- No new npm packages required
