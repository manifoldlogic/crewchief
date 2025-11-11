# Ticket: COMPFIX-1003: Enhanced Competition Runner

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

Transform the competition runner from a "spawn and hope" model to a validated setup pipeline with three distinct phases: Setup (sequential) → Validation (per-variant) → Execution (parallel). This ensures every agent has tools and environment ready before execution, eliminating the current 0% success rate.

## Background

The current competition runner (`packages/cli/src/search-optimization/competition-runner.ts`) immediately spawns agents after worktree creation, with no verification that:
- Database is accessible
- Base branch is indexed
- Worktrees are scanned
- MCP tools are available
- File permissions are correct

This leads to:
- 100% agent failure rate (no tools available)
- Wasted API credits (~$15-20 per failed run)
- No actionable feedback when setup fails
- Meaningless competition results (all variants fail equally)

This ticket integrates the validation module (COMPFIX-1001) and scan orchestrator (COMPFIX-1002) into the competition runner, implementing the three-phase flow that ensures 100% valid agent environments.

**Reference:** Section "Enhanced Competition Runner" in `planning/architecture.md` (lines 312-456)

## Acceptance Criteria

- [ ] Phase 1 (Setup) runs sequentially and includes ALL new validation steps
- [ ] Database connection validated before any worktree creation
- [ ] Base branch verified as indexed before proceeding
- [ ] All variant worktrees scanned after creation
- [ ] Phase 2 (Validation) runs per-variant validation checks
- [ ] Pre-flight validation results logged clearly for each variant
- [ ] Competition STOPS immediately if any validation fails (fail-fast)
- [ ] Phase 3 (Execution) only runs if ALL validations pass
- [ ] Parallel execution preserved for agents (existing behavior)
- [ ] Clear console output shows progress through all phases
- [ ] Competition report includes setup metrics (scan times, validation results)
- [ ] Integration tests verify: happy path, database failure, scan failure, validation failure
- [ ] All agents have validation ordering: validate BEFORE spawn (verified via test spies)

## Technical Requirements

### Three-Phase Flow

```typescript
// packages/cli/src/search-optimization/competition-runner.ts

export async function runCompetition(config: CompetitionConfig): Promise<CompetitionResult> {
  console.log('🏁 Starting competition with pre-flight validation')

  // ─────────────────────────────────────────────────────────
  // PHASE 1: SETUP (Sequential)
  // ─────────────────────────────────────────────────────────

  console.log('\n📋 Phase 1: Setup')
  console.log('='.repeat(60))

  // 1.1: Validate database connection
  const validator = new PreFlightValidator()
  const dbValid = await validator.checkDatabaseConnection()
  if (!dbValid) {
    throw new Error('Database connection failed - check MAPROOM_DATABASE_URL')
  }
  console.log('✅ Database connection verified')

  // 1.2: Verify base branch is indexed
  const baseIndexed = await validator.verifyBaseBranchIndexed('crewchief', 'main')
  if (!baseIndexed.indexed) {
    throw new Error(
      'Base branch not indexed - run: crewchief-maproom scan --repo crewchief --worktree main'
    )
  }
  console.log(`✅ Base branch indexed (${baseIndexed.chunkCount} chunks)`)

  // 1.3: Create competition directory
  const compId = `comp-${Date.now()}`
  const compDir = join(config.baseDir, compId)
  mkdirSync(compDir, { recursive: true })
  console.log(`✅ Competition directory: ${compDir}`)

  // 1.4: Load variants
  const variants = await loadVariants(config.variants)
  console.log(`✅ Loaded ${variants.length} variants`)

  // 1.5: Create variant worktrees (via SDK)
  const variantEnvs: VariantEnvironment[] = []
  for (const variant of variants) {
    const env = await createVariantWorktree({
      variant,
      taskName: config.task.id,
      baseDir: compDir
    })
    variantEnvs.push(env)
    console.log(`✅ Created worktree for ${variant.name}`)
  }

  // 1.6: Inject variant tool descriptions into MCP configs
  for (const env of variantEnvs) {
    await injectVariantDescription(env.worktreePath, env.variant)
    console.log(`✅ Injected variant: ${env.variant.name}`)
  }

  // 1.7: Scan all worktrees (NEW)
  console.log('\n📊 Scanning worktrees...')
  const scanConfigs = variantEnvs.map(env => ({
    worktreePath: env.worktreePath,
    repo: 'crewchief',
    worktree: env.worktreeName,
    commit: 'HEAD',
    baseDir: compDir
  }))

  const scanResults = await scanAllWorktrees(scanConfigs)
  console.log(`✅ All worktrees scanned (${scanResults.length} total)`)

  // ─────────────────────────────────────────────────────────
  // PHASE 2: VALIDATION (Per-Variant) (NEW)
  // ─────────────────────────────────────────────────────────

  console.log('\n🔍 Phase 2: Pre-Flight Validation')
  console.log('='.repeat(60))

  const validationResults: VariantValidation[] = []
  for (const env of variantEnvs) {
    const validation = await validator.validateVariantEnvironment(env)
    validationResults.push(validation)

    if (validation.overall === 'fail') {
      console.error(`❌ Validation failed for ${env.variant.name}: ${validation.failureReason}`)

      // Log all failed checks
      Object.entries(validation.checks).forEach(([check, result]) => {
        if (!result.passed) {
          console.error(`   - ${check}: ${result.message}`)
        }
      })

      throw new Error(`Pre-flight validation failed: ${validation.failureReason}`)
    }

    console.log(`✅ ${env.variant.name}: All checks passed`)
  }

  console.log('\n✅ All variants validated - ready for execution')

  // ─────────────────────────────────────────────────────────
  // PHASE 3: EXECUTION (Parallel) (Existing + Enhancements)
  // ─────────────────────────────────────────────────────────

  console.log('\n🚀 Phase 3: Agent Execution')
  console.log('='.repeat(60))

  const participants: ParticipantResult[] = []

  if (config.parallelExecution) {
    // Spawn all agents in parallel
    const promises = variantEnvs.map(env =>
      runVariantAgent(env, config.task, compDir)
    )
    const results = await Promise.all(promises)
    participants.push(...results)
  } else {
    // Run agents sequentially
    for (const env of variantEnvs) {
      const result = await runVariantAgent(env, config.task, compDir)
      participants.push(result)
    }
  }

  // ─────────────────────────────────────────────────────────
  // PHASE 4: EVALUATION (Existing)
  // ─────────────────────────────────────────────────────────

  console.log('\n📊 Phase 4: Evaluation')
  console.log('='.repeat(60))

  const winner = determineWinner(participants)
  const metrics = calculateCompetitionMetrics(participants)

  // Enhanced report with setup metrics
  const report = generateCompetitionReport({
    competitionId: compId,
    task: config.task,
    participants,
    winner,
    metrics,
    setupMetrics: {
      scanResults,
      validationResults,
      totalSetupTime: scanResults.reduce((sum, r) => sum + r.durationMs, 0)
    }
  })

  // Save report
  writeFileSync(join(compDir, 'report.txt'), report)
  console.log(`\n📄 Report saved: ${join(compDir, 'report.txt')}`)

  return {
    competitionId: compId,
    task: config.task,
    participants,
    winner,
    metrics,
    report
  }
}
```

### Error Handling & Fail-Fast

**ALL validation failures MUST stop execution immediately:**

```typescript
// Database failure
if (!dbValid) {
  throw new Error(`
❌ Pre-flight validation failed: Database connection failed

Troubleshooting:
- Verify PostgreSQL is running: docker ps | grep postgres
- Check MAPROOM_DATABASE_URL environment variable
- Test connection: psql $MAPROOM_DATABASE_URL -c "SELECT 1"

Current value: ${sanitizeDbUrl(process.env.MAPROOM_DATABASE_URL)}
  `.trim())
}

// Base branch failure
if (!baseIndexed.indexed) {
  throw new Error(`
❌ Pre-flight validation failed: Base branch 'main' not indexed

Fix: Run scan on base branch first
$ crewchief-maproom scan --repo crewchief --worktree main --root /workspace

This is a one-time setup step. Subsequent scans will be fast.
  `.trim())
}

// Scan failure (handled by scan orchestrator)
// Validation failure (handled in validation loop)
```

### Integration Points

**Import new modules:**
```typescript
import { PreFlightValidator } from './validation/pre-flight-validator'
import { scanAllWorktrees } from './scan-orchestrator'
import type { VariantValidation, ScanResult } from './types'
```

**Enhance report generation:**
```typescript
function generateCompetitionReport(data: {
  competitionId: string
  task: Task
  participants: ParticipantResult[]
  winner: ParticipantResult
  metrics: CompetitionMetrics
  setupMetrics?: {
    scanResults: ScanResult[]
    validationResults: VariantValidation[]
    totalSetupTime: number
  }
}): string {
  let report = existingReportGeneration()

  // Add setup metrics section
  if (data.setupMetrics) {
    report += '\n\n## Setup Metrics\n\n'
    report += `Total setup time: ${(data.setupMetrics.totalSetupTime / 1000).toFixed(1)}s\n\n`

    report += '### Scan Results\n'
    data.setupMetrics.scanResults.forEach(scan => {
      report += `- ${scan.worktree}: ${scan.chunkCount} chunks in ${(scan.durationMs / 1000).toFixed(1)}s\n`
    })

    report += '\n### Validation Results\n'
    data.setupMetrics.validationResults.forEach(val => {
      report += `- ${val.variantId}: ${val.overall}\n`
    })
  }

  return report
}
```

## Implementation Notes

### Console Output Format

**Expected console output for successful run:**

```
🏁 Starting competition with pre-flight validation

📋 Phase 1: Setup
============================================================
✅ Database connection verified
✅ Base branch indexed (1234 chunks)
✅ Competition directory: /tmp/comp-1234567890
✅ Loaded 2 variants

✅ Created worktree for variant-control
✅ Created worktree for variant-a-detailed
✅ Injected variant: variant-control
✅ Injected variant: variant-a-detailed

📊 Scanning worktrees...
============================================================
📊 Scanning worktree: variant-control
   Path: /tmp/comp-1234567890/worktrees/variant-control
   ✅ Scan complete: 567 chunks in 8234ms

📊 Scanning worktree: variant-a-detailed
   Path: /tmp/comp-1234567890/worktrees/variant-a-detailed
   ✅ Scan complete: 571 chunks in 7891ms
============================================================
✅ All scans complete in 16.1s
📊 Total chunks indexed: 1138

🔍 Phase 2: Pre-Flight Validation
============================================================
✅ variant-control: All checks passed
✅ variant-a-detailed: All checks passed

✅ All variants validated - ready for execution

🚀 Phase 3: Agent Execution
============================================================
[Agent execution logs...]

📊 Phase 4: Evaluation
============================================================
[Evaluation logs...]

📄 Report saved: /tmp/comp-1234567890/report.txt
```

### Testing Strategy

Create `packages/cli/src/search-optimization/competition-runner.integration.test.ts`:

```typescript
describe('Competition Runner Integration', () => {
  let testDir: string

  beforeAll(async () => {
    // Verify PostgreSQL is running
    const validator = new PreFlightValidator()
    const dbAvailable = await validator.checkDatabaseConnection()
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
    const result = await runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: ['variant-control', 'variant-a-detailed'],
      parallelExecution: false,
      baseDir: testDir,
      timeout: 180000
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
    // This test requires a separate test database or cleanup
    // Mock the validation to return false
    const mockValidator = jest.spyOn(PreFlightValidator.prototype, 'verifyBaseBranchIndexed')
    mockValidator.mockResolvedValue({ indexed: false, chunkCount: 0 })

    await expect(runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: ['variant-control'],
      baseDir: testDir
    })).rejects.toThrow('Base branch not indexed')

    mockValidator.mockRestore()
  })

  it('fails when worktree scan fails', async () => {
    const mockScan = jest.spyOn(scanOrchestrator, 'scanAllWorktrees')
    mockScan.mockRejectedValue(new Error('Scan failed for variant-a: Permission denied'))

    await expect(runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: ['variant-control'],
      baseDir: testDir
    })).rejects.toThrow('Scan failed')

    mockScan.mockRestore()
  })

  it('validates all worktrees before agent spawn', async () => {
    const spyValidate = jest.spyOn(PreFlightValidator.prototype, 'validateVariantEnvironment')
    const spySpawn = jest.spyOn(agentSdk, 'runAgent')

    await runCompetition({
      task: TASK_FIND_WORKTREE_CREATION,
      variants: ['variant-control', 'variant-a-detailed'],
      parallelExecution: false,
      baseDir: testDir
    })

    // Validation should be called for each variant
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

**Test coverage:**
- Full competition flow (happy path): 1 test (600s timeout - actual agent execution)
- Database failure: 1 test
- Base branch validation: 1 test
- Scan failure: 1 test
- Validation ordering: 1 test (critical - ensures no agents run before validation)

### Performance Impact

**Expected timing (12 variants):**
- Phase 1 setup: ~2-3 minutes (worktree creation + scanning)
- Phase 2 validation: ~10-20 seconds (all variants)
- Phase 3 execution: ~2-5 minutes (parallel agents)
- **Total: ~4-8 minutes** (vs ~2-3 minutes without validation)

**Tradeoff:** +2-3 minutes for 100% success rate (vs 0% currently)

This overhead is acceptable and documented in `planning/architecture.md` (lines 534-558).

## Dependencies

- **Prerequisite tickets:**
  - COMPFIX-1001 (Pre-Flight Validation Module) - REQUIRED
  - COMPFIX-1002 (Scan Orchestration Module) - REQUIRED

- **External dependencies:**
  - Existing competition runner infrastructure
  - Claude Code Agents SDK for agent spawning
  - PostgreSQL running and accessible

- **Blocks:**
  - COMPFIX-2002 (End-to-End Validation) - needs this runner
  - COMPFIX-2003 (Error Scenario Testing) - needs this runner

## Risk Assessment

- **Risk**: Integration breaks existing competition logic
  - **Mitigation**: Preserve all existing Phase 3/4 logic, only add Phases 1-2
  - **Testing**: Integration tests verify existing behavior preserved

- **Risk**: Setup time too long (>5 minutes)
  - **Mitigation**: Measure actual times in tests, optimize if needed
  - **Acceptable**: +2-3 minutes is acceptable for 100% success rate

- **Risk**: Validation false positives/negatives
  - **Mitigation**: Comprehensive validation module tests (COMPFIX-1001)
  - **Fallback**: Manual verification in COMPFIX-2002

- **Risk**: Console output too verbose
  - **Mitigation**: Use progress indicators, collapsible sections
  - **Future**: Add `--quiet` flag for minimal output

## Files/Packages Affected

**Modified files:**
- `packages/cli/src/search-optimization/competition-runner.ts` (major changes)
- `packages/cli/src/search-optimization/types.ts` (add setupMetrics types)

**New files:**
- `packages/cli/src/search-optimization/competition-runner.integration.test.ts`

**No breaking changes** - existing competition configs still work
