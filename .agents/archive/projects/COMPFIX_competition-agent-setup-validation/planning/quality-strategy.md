# Quality Strategy: Competition Agent Setup and Validation

## Testing Philosophy

**Confidence, not coverage.** We need to ensure the competition framework correctly validates environments and fails fast on setup errors. The goal is zero wasted API credits from invalid setups.

**Critical paths:**
1. Pre-flight validation catches all setup failures
2. Worktree scanning completes successfully
3. Agents have access to maproom tools
4. Tests only run when environment is valid

## Test Strategy

### Unit Tests (60% effort, 80% confidence)

**Target:** Individual validation functions and helpers

#### 1. Pre-Flight Validation Module

**File:** `packages/cli/src/search-optimization/validation/pre-flight-validator.test.ts`

```typescript
describe('PreFlightValidator', () => {
  describe('checkDatabaseConnection', () => {
    it('returns true when database is reachable', async () => {
      const validator = new PreFlightValidator()
      const result = await validator.checkDatabaseConnection()
      expect(result).toBe(true)
    })

    it('returns false when MAPROOM_DATABASE_URL is invalid', async () => {
      const originalUrl = process.env.MAPROOM_DATABASE_URL
      process.env.MAPROOM_DATABASE_URL = 'postgresql://invalid:invalid@localhost:9999/fake'

      const validator = new PreFlightValidator()
      const result = await validator.checkDatabaseConnection()

      expect(result).toBe(false)

      process.env.MAPROOM_DATABASE_URL = originalUrl
    })

    it('returns false when database is not running', async () => {
      // Mock pg.Client to throw connection error
      jest.mock('pg', () => ({
        Client: jest.fn().mockImplementation(() => ({
          connect: jest.fn().mockRejectedValue(new Error('ECONNREFUSED')),
        }))
      }))

      const validator = new PreFlightValidator()
      const result = await validator.checkDatabaseConnection()

      expect(result).toBe(false)
    })
  })

  describe('verifyBaseBranchIndexed', () => {
    it('returns true when base branch has chunks', async () => {
      // Mock maproom status command
      const mockExec = jest.spyOn(child_process, 'execSync')
      mockExec.mockReturnValue(JSON.stringify({
        worktrees: [
          { name: 'main', chunk_count: 1234 }
        ]
      }))

      const validator = new PreFlightValidator()
      const result = await validator.verifyBaseBranchIndexed('crewchief', 'main')

      expect(result.indexed).toBe(true)
      expect(result.chunkCount).toBe(1234)
      mockExec.mockRestore()
    })

    it('returns false when base branch not in database', async () => {
      const mockExec = jest.spyOn(child_process, 'execSync')
      mockExec.mockReturnValue(JSON.stringify({
        worktrees: []
      }))

      const validator = new PreFlightValidator()
      const result = await validator.verifyBaseBranchIndexed('crewchief', 'main')

      expect(result.indexed).toBe(false)
      expect(result.chunkCount).toBe(0)
      mockExec.mockRestore()
    })
  })

  describe('checkWorktreeScanned', () => {
    it('passes when worktree has chunks indexed', async () => {
      const mockExec = jest.spyOn(child_process, 'execSync')
      mockExec.mockReturnValue(JSON.stringify({
        worktrees: [
          { name: 'variant-a', chunk_count: 567 }
        ]
      }))

      const validator = new PreFlightValidator()
      const result = await validator.checkWorktreeScanned('crewchief', 'variant-a')

      expect(result.passed).toBe(true)
      expect(result.details.chunkCount).toBe(567)
      mockExec.mockRestore()
    })

    it('fails when worktree has 0 chunks', async () => {
      const mockExec = jest.spyOn(child_process, 'execSync')
      mockExec.mockReturnValue(JSON.stringify({
        worktrees: [
          { name: 'variant-a', chunk_count: 0 }
        ]
      }))

      const validator = new PreFlightValidator()
      const result = await validator.checkWorktreeScanned('crewchief', 'variant-a')

      expect(result.passed).toBe(false)
      expect(result.message).toContain('0 chunks')
      mockExec.mockRestore()
    })

    it('fails when worktree not in database', async () => {
      const mockExec = jest.spyOn(child_process, 'execSync')
      mockExec.mockReturnValue(JSON.stringify({
        worktrees: []
      }))

      const validator = new PreFlightValidator()
      const result = await validator.checkWorktreeScanned('crewchief', 'variant-a')

      expect(result.passed).toBe(false)
      expect(result.message).toContain('not in database')
      mockExec.mockRestore()
    })
  })

  describe('checkMcpConfigValid', () => {
    it('passes when .mcp.json has maproom server', () => {
      const tempDir = mkdtempSync(join(tmpdir(), 'test-'))
      writeFileSync(join(tempDir, '.mcp.json'), JSON.stringify({
        mcpServers: {
          maproom: {
            command: 'node',
            args: ['/path/to/maproom/index.js']
          }
        }
      }))

      const validator = new PreFlightValidator()
      const result = validator.checkMcpConfigValid(tempDir)

      expect(result.passed).toBe(true)
      expect(result.message).toContain('MCP config valid')

      rmSync(tempDir, { recursive: true })
    })

    it('fails when .mcp.json is missing', () => {
      const tempDir = mkdtempSync(join(tmpdir(), 'test-'))

      const validator = new PreFlightValidator()
      const result = validator.checkMcpConfigValid(tempDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('not found')

      rmSync(tempDir, { recursive: true })
    })

    it('fails when maproom server not configured', () => {
      const tempDir = mkdtempSync(join(tmpdir(), 'test-'))
      writeFileSync(join(tempDir, '.mcp.json'), JSON.stringify({
        mcpServers: {}
      }))

      const validator = new PreFlightValidator()
      const result = validator.checkMcpConfigValid(tempDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('not configured')

      rmSync(tempDir, { recursive: true })
    })
  })

  describe('checkFilePermissions', () => {
    it('passes when worktree is readable/writable', () => {
      const tempDir = mkdtempSync(join(tmpdir(), 'test-'))
      writeFileSync(join(tempDir, 'package.json'), '{}')

      const validator = new PreFlightValidator()
      const result = validator.checkFilePermissions(tempDir)

      expect(result.passed).toBe(true)

      rmSync(tempDir, { recursive: true })
    })

    it('fails when worktree is not writable', () => {
      const tempDir = mkdtempSync(join(tmpdir(), 'test-'))
      chmodSync(tempDir, 0o444) // read-only

      const validator = new PreFlightValidator()
      const result = validator.checkFilePermissions(tempDir)

      expect(result.passed).toBe(false)
      expect(result.message).toContain('Permission')

      chmodSync(tempDir, 0o755) // restore permissions
      rmSync(tempDir, { recursive: true })
    })
  })
})
```

**Coverage targets:**
- checkDatabaseConnection: 100% (critical path)
- verifyBaseBranchIndexed: 100% (critical path)
- checkWorktreeScanned: 100% (critical path)
- checkMcpConfigValid: 100% (critical path)
- checkFilePermissions: 90% (edge cases acceptable)

#### 2. Scan Orchestrator Module

**File:** `packages/cli/src/search-optimization/scan-orchestrator.test.ts`

```typescript
describe('ScanOrchestrator', () => {
  describe('scanWorktree', () => {
    it('returns success when scan completes', async () => {
      const mockExec = jest.spyOn(child_process, 'execSync')
      mockExec.mockReturnValue('Total chunks: 123')

      const result = await scanWorktree({
        worktreePath: '/tmp/test',
        repo: 'crewchief',
        worktree: 'test',
        commit: 'abc123',
        baseDir: '/tmp'
      })

      expect(result.success).toBe(true)
      expect(result.chunkCount).toBe(123)
      mockExec.mockRestore()
    })

    it('returns failure when scan command fails', async () => {
      const mockExec = jest.spyOn(child_process, 'execSync')
      mockExec.mockImplementation(() => {
        throw new Error('Scan failed: permission denied')
      })

      const result = await scanWorktree({
        worktreePath: '/tmp/test',
        repo: 'crewchief',
        worktree: 'test',
        commit: 'abc123',
        baseDir: '/tmp'
      })

      expect(result.success).toBe(false)
      expect(result.error).toContain('permission denied')
      mockExec.mockRestore()
    })
  })

  describe('scanAllWorktrees', () => {
    it('scans all worktrees sequentially', async () => {
      const mockScan = jest.fn().mockResolvedValue({
        success: true,
        chunkCount: 100,
        durationMs: 5000
      })

      const configs = [
        { worktreePath: '/tmp/a', repo: 'test', worktree: 'a', commit: '123', baseDir: '/tmp' },
        { worktreePath: '/tmp/b', repo: 'test', worktree: 'b', commit: '456', baseDir: '/tmp' }
      ]

      const results = await scanAllWorktrees(configs, mockScan)

      expect(mockScan).toHaveBeenCalledTimes(2)
      expect(results).toHaveLength(2)
      expect(results.every(r => r.success)).toBe(true)
    })

    it('throws error if any scan fails', async () => {
      const mockScan = jest.fn()
        .mockResolvedValueOnce({ success: true })
        .mockResolvedValueOnce({ success: false, error: 'Scan failed' })

      const configs = [
        { worktreePath: '/tmp/a', repo: 'test', worktree: 'a', commit: '123', baseDir: '/tmp' },
        { worktreePath: '/tmp/b', repo: 'test', worktree: 'b', commit: '456', baseDir: '/tmp' }
      ]

      await expect(scanAllWorktrees(configs, mockScan))
        .rejects
        .toThrow('Scan failed')
    })
  })
})
```

**Coverage targets:**
- scanWorktree: 95% (main flows + error cases)
- scanAllWorktrees: 95% (sequential execution + early failure)

### Integration Tests (30% effort, 15% confidence)

**Target:** End-to-end competition flow with real worktrees

**File:** `packages/cli/src/search-optimization/competition-runner.integration.test.ts`

```typescript
describe('Competition Runner Integration', () => {
  let testDir: string

  beforeAll(async () => {
    // Verify PostgreSQL is running
    const dbAvailable = await checkDatabaseConnection()
    if (!dbAvailable) {
      throw new Error('PostgreSQL must be running for integration tests')
    }

    // Scan base branch once
    execSync('crewchief-maproom scan --repo crewchief-test --worktree main --root /workspace')
  })

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'comp-test-'))
  })

  afterEach(() => {
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true })
    }
  })

  it('completes full competition with valid setup', async () => {
    const variants = [
      await loadVariant('variant-control'),
      await loadVariant('variant-a-detailed')
    ]

    const result = await runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: variants.map(v => v.id),
      parallelExecution: false,
      baseDir: testDir,
      timeout: 180
    })

    // Verify all phases completed
    expect(result.competitionId).toBeDefined()
    expect(result.participants).toHaveLength(2)
    expect(result.winner).toBeDefined()

    // Verify at least one agent used search
    const searchUsed = result.participants.some(p =>
      p.toolsUsed?.includes('mcp__maproom__search')
    )
    expect(searchUsed).toBe(true)
  }, 600000) // 10min timeout

  it('fails fast when database is unavailable', async () => {
    // Stop database or set invalid URL
    const originalUrl = process.env.MAPROOM_DATABASE_URL
    process.env.MAPROOM_DATABASE_URL = 'postgresql://invalid:invalid@localhost:9999/fake'

    await expect(runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: ['variant-control'],
      baseDir: testDir
    })).rejects.toThrow('Database connection failed')

    process.env.MAPROOM_DATABASE_URL = originalUrl
  })

  it('fails fast when base branch not indexed', async () => {
    // Use non-existent repo
    const result = runCompetition({
      task: {
        ...TASK_FIND_WORKTREE_CREATION,
        id: 'test-unindexed'
      },
      variants: ['variant-control'],
      baseDir: testDir
    })

    await expect(result).rejects.toThrow('Base branch not indexed')
  })

  it('fails when worktree scan fails', async () => {
    // Mock scan failure
    const mockExec = jest.spyOn(child_process, 'execSync')
    mockExec.mockImplementation((cmd) => {
      if (cmd.includes('maproom scan')) {
        throw new Error('Permission denied')
      }
      return ''
    })

    await expect(runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: ['variant-control'],
      baseDir: testDir
    })).rejects.toThrow('Scan failed')

    mockExec.mockRestore()
  })

  it('validates all worktrees before agent spawn', async () => {
    const spyValidate = jest.spyOn(preFlight, 'validateVariantEnvironment')
    const spySpawn = jest.spyOn(sdk, 'runAgent')

    await runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: ['variant-control', 'variant-a-detailed'],
      parallelExecution: false,
      baseDir: testDir
    })

    // Validation should be called before spawn
    expect(spyValidate).toHaveBeenCalledTimes(2)
    expect(spySpawn).toHaveBeenCalledTimes(2)

    // Validate calls should precede spawn calls
    const validateOrder = spyValidate.mock.invocationCallOrder
    const spawnOrder = spySpawn.mock.invocationCallOrder
    expect(Math.max(...validateOrder)).toBeLessThan(Math.min(...spawnOrder))

    spyValidate.mockRestore()
    spySpawn.mockRestore()
  })
})
```

**Coverage targets:**
- Full competition flow: 1 test (happy path)
- Database failure: 1 test
- Base branch validation: 1 test
- Scan failure: 1 test
- Validation ordering: 1 test

**Why so few?** Integration tests are expensive (time + API costs). We get more confidence from unit tests on individual validators.

### Manual Testing (10% effort, 5% confidence)

**Scenarios to test manually:**

1. **Fresh setup (no base branch indexed)**
   ```bash
   # Clear database
   psql $MAPROOM_DATABASE_URL -c "DELETE FROM chunks WHERE repo_id = (SELECT id FROM repos WHERE name = 'crewchief-test')"

   # Run competition - should fail with clear message
   pnpm tsx scripts/run-genetic-optimizer.ts

   # Expected: "Base branch not indexed - run: crewchief-maproom scan..."
   ```

2. **Database disconnected during setup**
   ```bash
   # Stop PostgreSQL mid-competition
   docker stop maproom-postgres

   # Expected: "Database connection failed - check MAPROOM_DATABASE_URL"
   ```

3. **Parallel execution with 12 variants**
   ```bash
   # Run ultra optimizer
   pnpm tsx scripts/run-genetic-optimizer-ultra.ts

   # Verify:
   # - All 12 worktrees scanned
   # - All 12 validations pass
   # - Agents run in parallel
   # - At least some agents use search tool
   ```

4. **Permission issues**
   ```bash
   # Make worktree read-only
   chmod 444 .crewchief/worktrees/test-variant

   # Run competition
   # Expected: "Permission error: EACCES"
   ```

## Risk Mitigation

### High-Risk Areas

1. **Database connection failures**
   - **Risk:** Tests fail silently, waste API credits
   - **Mitigation:** Eager validation at competition start
   - **Test:** Unit tests + integration test with invalid URL

2. **Race conditions in parallel scanning**
   - **Risk:** Concurrent scans corrupt database
   - **Mitigation:** Sequential scanning (simple > fast)
   - **Test:** Not applicable (sequential only)

3. **Worktree not indexed**
   - **Risk:** Agents can't use search tool, 0% usage
   - **Mitigation:** Validation checks chunk_count > 0
   - **Test:** Unit test + integration test

4. **MCP config malformed**
   - **Risk:** Tools unavailable but validation passes
   - **Mitigation:** Parse and validate .mcp.json structure
   - **Test:** Unit tests for various malformed configs

5. **File permission issues**
   - **Risk:** Agent can't read/write worktree files
   - **Mitigation:** Test read/write before spawn
   - **Test:** Unit test with read-only directory

### Low-Risk Areas

1. **Scan performance** - Fast enough (5-15s per worktree)
2. **Database capacity** - PostgreSQL handles 12 concurrent connections easily
3. **API rate limits** - Validation doesn't call Anthropic API
4. **Disk space** - Worktrees are small (~100MB each)

## Test Execution

### Local Development

```bash
# Run all tests
pnpm test

# Run only validation tests
pnpm test validation

# Run only competition runner tests
pnpm test competition-runner

# Run integration tests (requires PostgreSQL)
pnpm test:integration

# Watch mode
pnpm test:watch
```

### CI Pipeline

```yaml
# .github/workflows/test.yml
- name: Start PostgreSQL
  run: |
    docker compose -f packages/maproom-mcp/config/docker-compose.yml up -d
    sleep 5

- name: Run unit tests
  run: pnpm test --coverage

- name: Run integration tests
  run: pnpm test:integration
  env:
    MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5432/maproom
```

## Success Metrics

### Definition of Done

**Unit tests:**
- ✅ All validation functions have 95%+ coverage
- ✅ All error paths tested
- ✅ All edge cases covered

**Integration tests:**
- ✅ Full competition completes successfully
- ✅ Database failures are caught
- ✅ Scan failures are caught
- ✅ Validation runs before agent spawn

**Manual verification:**
- ✅ Ultra optimizer completes 12-variant competition
- ✅ At least 50% of agents use search tool
- ✅ Setup failures provide clear error messages
- ✅ No API credits wasted on invalid setups

### Acceptance Criteria

**Before deployment:**
1. All unit tests pass
2. All integration tests pass
3. Manual ultra run completes successfully
4. Zero agents have 0 searches (all have tool access)
5. Documentation updated with new validation steps

**Continuous monitoring:**
- Track: % of competitions that fail validation
- Track: % of agents that use search tool
- Track: Average setup time per variant
- Alert: If validation failure rate > 5%

## Testing Anti-Patterns to Avoid

❌ **Don't test implementation details**
```typescript
// BAD: Testing internal state
expect(validator.privateField).toBe('value')

// GOOD: Testing behavior
expect(validator.checkDatabase()).resolves.toBe(true)
```

❌ **Don't mock everything**
```typescript
// BAD: Over-mocking loses confidence
jest.mock('fs')
jest.mock('child_process')
jest.mock('pg')

// GOOD: Mock only external dependencies
// Use real fs for temp directories
// Mock only network calls
```

❌ **Don't write flaky tests**
```typescript
// BAD: Time-dependent
setTimeout(() => expect(result).toBe(true), 100)

// GOOD: Wait for conditions
await waitFor(() => expect(result).toBe(true))
```

❌ **Don't skip cleanup**
```typescript
// BAD: Leaves temp files
writeFileSync('/tmp/test', 'data')

// GOOD: Always cleanup
const tempDir = mkdtempSync(join(tmpdir(), 'test-'))
try {
  // test code
} finally {
  rmSync(tempDir, { recursive: true })
}
```

## MVP Testing Scope

**In scope:**
- ✅ Validation function unit tests
- ✅ Scan orchestrator unit tests
- ✅ Competition runner integration test (happy path)
- ✅ Database failure test
- ✅ Scan failure test

**Out of scope (Phase 2):**
- ⏸️ Performance benchmarks
- ⏸️ Chaos testing (random failures)
- ⏸️ Load testing (100+ variants)
- ⏸️ UI/UX testing (CLI output)
- ⏸️ Mutation testing

**Why?** MVP scope gets us to 90% confidence with 40% effort. Diminishing returns beyond that.
