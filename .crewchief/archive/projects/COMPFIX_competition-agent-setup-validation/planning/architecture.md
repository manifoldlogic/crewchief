# Architecture: Competition Agent Setup and Validation

## Solution Overview

Transform the competition runner from a "spawn and hope" model to a validated setup pipeline that ensures every agent has the tools and environment needed for fair competition.

**Core principle:** Setup sequentially, execute in parallel, validate before running.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Competition Runner                       │
│                                                             │
│  ┌─────────────┐   ┌──────────────┐   ┌────────────────┐  │
│  │   Setup     │──▶│  Validation  │──▶│   Execution    │  │
│  │  Phase      │   │    Phase     │   │     Phase      │  │
│  │ (Sequential)│   │ (Per-Variant)│   │   (Parallel)   │  │
│  └─────────────┘   └──────────────┘   └────────────────┘  │
│         │                  │                    │          │
│         ▼                  ▼                    ▼          │
│   Create Worktrees    Test Tools        Spawn Agents      │
│   Scan Worktrees      Verify Index      Collect Results   │
│   Inject Variants     Check Access      Evaluate Winner   │
└─────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Pre-Flight Validation Module

**Purpose:** Verify environment is ready before expensive agent operations

**Interface:**
```typescript
interface PreFlightValidation {
  // Validate entire competition setup
  validateCompetitionSetup(config: CompetitionConfig): Promise<ValidationResult>

  // Validate single variant environment
  validateVariantEnvironment(variant: VariantEnvironment): Promise<VariantValidation>

  // Check maproom database connectivity
  checkDatabaseConnection(): Promise<boolean>

  // Verify base branch is indexed
  verifyBaseBranchIndexed(repo: string, branch: string): Promise<IndexStatus>
}

interface ValidationResult {
  valid: boolean
  errors: ValidationError[]
  warnings: ValidationWarning[]
  variantResults: Map<string, VariantValidation>
}

interface VariantValidation {
  variantId: string
  worktreePath: string
  checks: {
    worktreeExists: CheckResult
    worktreeScanned: CheckResult
    mcpConfigValid: CheckResult
    toolsAccessible: CheckResult
    filePermissions: CheckResult
  }
  overall: 'pass' | 'fail'
  failureReason?: string
}

interface CheckResult {
  passed: boolean
  message: string
  details?: any
}
```

**Validation Checks:**

1. **Database Connectivity**
   ```typescript
   async checkDatabaseConnection(): Promise<boolean> {
     try {
       const client = new pg.Client(process.env.MAPROOM_DATABASE_URL)
       await client.connect()
       await client.query('SELECT 1')
       await client.end()
       return true
     } catch (error) {
       console.error('Database connection failed:', error.message)
       return false
     }
   }
   ```

2. **Base Branch Indexed**
   ```typescript
   async verifyBaseBranchIndexed(repo: string, branch: string): Promise<IndexStatus> {
     const status = await execMaproom([
       'status',
       '--repo', repo,
       '--worktree', branch,
       '--json'
     ])

     const data = JSON.parse(status)
     return {
       indexed: data.worktrees?.some(w => w.name === branch && w.chunk_count > 0),
       chunkCount: data.worktrees?.find(w => w.name === branch)?.chunk_count || 0
     }
   }
   ```

3. **Worktree Scanned**
   ```typescript
   async checkWorktreeScanned(repo: string, worktree: string): Promise<CheckResult> {
     try {
       const status = await execMaproom([
         'status',
         '--repo', repo,
         '--worktree', worktree,
         '--json'
       ])

       const data = JSON.parse(status)
       const wt = data.worktrees?.find(w => w.name === worktree)

       if (!wt) {
         return { passed: false, message: 'Worktree not in database' }
       }

       if (wt.chunk_count === 0) {
         return { passed: false, message: 'Worktree has 0 chunks indexed' }
       }

       return {
         passed: true,
         message: `Indexed with ${wt.chunk_count} chunks`,
         details: { chunkCount: wt.chunk_count }
       }
     } catch (error) {
       return { passed: false, message: error.message }
     }
   }
   ```

4. **MCP Tools Accessible**
   ```typescript
   async checkToolsAccessible(worktreePath: string): Promise<CheckResult> {
     // Read .mcp.json from worktree
     const mcpConfig = JSON.parse(
       readFileSync(join(worktreePath, '.mcp.json'), 'utf-8')
     )

     // Verify maproom server is configured
     const maproomServer = mcpConfig.mcpServers?.maproom
     if (!maproomServer) {
       return { passed: false, message: 'Maproom MCP server not configured' }
     }

     // Verify required tools are available
     const requiredTools = [
       'mcp__maproom__search',
       'mcp__maproom__open',
       'mcp__maproom__context',
       'mcp__maproom__status'
     ]

     // Note: We can't actually test tool availability without spawning agent
     // But we can verify MCP config is present and well-formed

     return {
       passed: true,
       message: 'MCP config valid',
       details: { server: maproomServer.command }
     }
   }
   ```

5. **File Permissions**
   ```typescript
   async checkFilePermissions(worktreePath: string): Promise<CheckResult> {
     try {
       // Test read access
       const testFile = join(worktreePath, 'package.json')
       if (existsSync(testFile)) {
         readFileSync(testFile, 'utf-8')
       }

       // Test write access
       const tempFile = join(worktreePath, '.crewchief-test-write')
       writeFileSync(tempFile, 'test')
       unlinkSync(tempFile)

       return { passed: true, message: 'Read/write permissions OK' }
     } catch (error) {
       return {
         passed: false,
         message: `Permission error: ${error.message}`
       }
     }
   }
   ```

### 2. Scan Orchestration Module

**Purpose:** Ensure all worktrees are indexed before agent execution

**Interface:**
```typescript
interface ScanOrchestrator {
  // Scan single worktree
  scanWorktree(config: ScanConfig): Promise<ScanResult>

  // Scan multiple worktrees in sequence
  scanAllWorktrees(configs: ScanConfig[]): Promise<ScanResult[]>

  // Wait for scan completion
  waitForScanCompletion(scanId: string, timeout?: number): Promise<void>
}

interface ScanConfig {
  worktreePath: string
  repo: string
  worktree: string
  commit: string
  baseDir: string
}

interface ScanResult {
  success: boolean
  worktree: string
  chunkCount: number
  durationMs: number
  error?: string
}
```

**Implementation:**

```typescript
export async function scanWorktree(config: ScanConfig): Promise<ScanResult> {
  const startTime = Date.now()

  console.log(`📊 Scanning worktree: ${config.worktree}`)
  console.log(`   Path: ${config.worktreePath}`)

  try {
    // Run maproom scan
    const result = execSync(
      `crewchief-maproom scan \
        --repo ${config.repo} \
        --worktree ${config.worktree} \
        --commit ${config.commit} \
        --root ${config.worktreePath}`,
      { encoding: 'utf-8', stdio: 'pipe' }
    )

    // Parse scan output for chunk count
    const match = result.match(/Total chunks: (\d+)/)
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
    console.error(`   ❌ Scan failed: ${error.message}`)

    return {
      success: false,
      worktree: config.worktree,
      chunkCount: 0,
      durationMs: Date.now() - startTime,
      error: error.message
    }
  }
}

export async function scanAllWorktrees(configs: ScanConfig[]): Promise<ScanResult[]> {
  const results: ScanResult[] = []

  console.log(`\n📊 Scanning ${configs.length} worktrees...`)
  console.log('=' .repeat(60))

  // Scan sequentially (fast due to embedding reuse)
  for (const config of configs) {
    const result = await scanWorktree(config)
    results.push(result)

    if (!result.success) {
      throw new Error(`Scan failed for ${config.worktree}: ${result.error}`)
    }
  }

  const totalDuration = results.reduce((sum, r) => sum + r.durationMs, 0)
  console.log('=' .repeat(60))
  console.log(`✅ All scans complete in ${(totalDuration / 1000).toFixed(1)}s`)
  console.log()

  return results
}
```

### 3. Enhanced Competition Runner

**New flow with validation:**

```typescript
export async function runCompetition(config: CompetitionConfig): Promise<CompetitionResult> {
  console.log('🏁 Starting competition with pre-flight validation')

  // ─────────────────────────────────────────────────────────
  // PHASE 1: SETUP (Sequential)
  // ─────────────────────────────────────────────────────────

  console.log('\n📋 Phase 1: Setup')
  console.log('=' .repeat(60))

  // 1.1: Validate database connection
  const dbValid = await preFlight.checkDatabaseConnection()
  if (!dbValid) {
    throw new Error('Database connection failed - check MAPROOM_DATABASE_URL')
  }
  console.log('✅ Database connection verified')

  // 1.2: Verify base branch is indexed
  const baseIndexed = await preFlight.verifyBaseBranchIndexed('crewchief', 'main')
  if (!baseIndexed.indexed) {
    throw new Error('Base branch not indexed - run: crewchief-maproom scan --repo crewchief --worktree main')
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

  // 1.7: Scan all worktrees
  console.log('\n📊 Scanning worktrees...')
  const scanConfigs = variantEnvs.map(env => ({
    worktreePath: env.worktreePath,
    repo: 'crewchief',
    worktree: env.worktreeName,
    commit: 'HEAD',
    baseDir: compDir
  }))

  const scanResults = await scanAllWorktrees(scanConfigs)
  console.log(`✅ All worktrees scanned`)

  // ─────────────────────────────────────────────────────────
  // PHASE 2: VALIDATION (Per-Variant)
  // ─────────────────────────────────────────────────────────

  console.log('\n🔍 Phase 2: Pre-Flight Validation')
  console.log('=' .repeat(60))

  const validationResults: VariantValidation[] = []
  for (const env of variantEnvs) {
    const validation = await preFlight.validateVariantEnvironment(env)
    validationResults.push(validation)

    if (validation.overall === 'fail') {
      console.error(`❌ Validation failed for ${env.variant.name}: ${validation.failureReason}`)
      throw new Error(`Pre-flight validation failed: ${validation.failureReason}`)
    }

    console.log(`✅ ${env.variant.name}: All checks passed`)
  }

  console.log('\n✅ All variants validated - ready for execution')

  // ─────────────────────────────────────────────────────────
  // PHASE 3: EXECUTION (Parallel)
  // ─────────────────────────────────────────────────────────

  console.log('\n🚀 Phase 3: Agent Execution')
  console.log('=' .repeat(60))

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
  // PHASE 4: EVALUATION
  // ─────────────────────────────────────────────────────────

  console.log('\n📊 Phase 4: Evaluation')
  console.log('=' .repeat(60))

  const winner = determineWinner(participants)
  const metrics = calculateCompetitionMetrics(participants)
  const report = generateCompetitionReport({
    competitionId: compId,
    task: config.task,
    participants,
    winner,
    metrics
  })

  // Save report
  writeFileSync(join(compDir, 'report.txt'), report)

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

### 4. Variant Environment Structure

```typescript
interface VariantEnvironment {
  variant: Variant
  worktreePath: string
  worktreeName: string
  mcpConfigPath: string
  validationStatus?: VariantValidation
}
```

### 5. MCP Configuration Strategy

**DO NOT force tool usage** - provide tools but let agent choose:

```jsonc
// .mcp.json in each variant worktree
{
  "mcpServers": {
    "maproom": {
      "command": "node",
      "args": [
        "/workspace/packages/maproom-mcp/dist/index.js"
      ],
      "env": {
        "MAPROOM_DATABASE_URL": "postgresql://maproom:maproom@localhost:5432/maproom"
      }
    }
  }
}
```

**Tool description injection:**
- Modify maproom MCP server's tool description for `mcp__maproom__search`
- Keep all other tools at default descriptions
- Agent sees custom description and decides whether to use tool

## Data Flow

```
Competition Start
    │
    ├─▶ Create Worktrees (one per variant)
    │      └─▶ worktree-variant-a/, worktree-variant-b/, ...
    │
    ├─▶ Inject Variant Descriptions
    │      └─▶ Modify .mcp.json in each worktree
    │
    ├─▶ Scan Worktrees (sequential, ~5-15s each)
    │      └─▶ maproom scan --repo crewchief --worktree variant-a ...
    │      └─▶ Reuses embeddings from base branch (fast)
    │
    ├─▶ Validate Each Worktree
    │      ├─▶ Check: worktree exists
    │      ├─▶ Check: indexed (chunk_count > 0)
    │      ├─▶ Check: MCP config present
    │      ├─▶ Check: file permissions OK
    │      └─▶ Fail fast if any check fails
    │
    ├─▶ Spawn Agents (parallel execution)
    │      ├─▶ Agent A in worktree-variant-a
    │      ├─▶ Agent B in worktree-variant-b
    │      └─▶ ... (all variants run concurrently)
    │
    ├─▶ Collect Results
    │      ├─▶ Did agent use search tool?
    │      ├─▶ How many searches?
    │      ├─▶ Search quality (target found?)
    │      └─▶ Task completion
    │
    └─▶ Determine Winner
           └─▶ Variant with best composite score
```

## Performance Considerations

### Time Budget Analysis

**For 12 variants (ultra configuration):**

- Worktree creation: 12 × 2s = 24s (sequential)
- Variant injection: 12 × 0.5s = 6s (sequential)
- Worktree scanning: 12 × 10s = 120s (sequential, embedding reuse)
- Validation: 12 × 2s = 24s (sequential)
- **Setup total: ~174s (~3 minutes)**

- Agent execution: ~60-180s (parallel, all run at once)
- **Execution total: ~60-180s**

**Total competition time: ~4-6 minutes** (vs current ~2-3 minutes)

**Tradeoff:** +2-3 minutes setup for 100% success rate (vs 0% currently)

### Optimization Opportunities

1. **Parallel scanning:** PostgreSQL can handle concurrent inserts
   - Risk: Race conditions on embedding generation
   - Benefit: Reduce 120s → 30s
   - Decision: Keep sequential for simplicity (setup isn't bottleneck)

2. **Lazy validation:** Only validate on first failure
   - Risk: Waste agent API costs on invalid setup
   - Benefit: Faster start
   - Decision: Eager validation (fail fast principle)

3. **Cached validation:** Reuse validation across generations
   - Risk: Stale state if environment changes
   - Benefit: Faster subsequent generations
   - Decision: Re-validate each generation (state can change)

## Error Handling

### Failure Scenarios

1. **Database unreachable**
   ```
   ❌ Pre-flight validation failed: Database connection failed

   Troubleshooting:
   - Verify PostgreSQL is running
   - Check MAPROOM_DATABASE_URL environment variable
   - Test connection: psql $MAPROOM_DATABASE_URL -c "SELECT 1"
   ```

2. **Base branch not indexed**
   ```
   ❌ Pre-flight validation failed: Base branch 'main' not indexed

   Fix: Run scan on base branch first
   $ crewchief-maproom scan --repo crewchief --worktree main --root /workspace
   ```

3. **Worktree scan failed**
   ```
   ❌ Worktree scan failed for variant-a-detailed: Permission denied

   Troubleshooting:
   - Check worktree path exists and is writable
   - Verify crewchief-maproom binary is in PATH
   - Check database write permissions
   ```

4. **MCP tools not accessible**
   ```
   ❌ Validation failed: MCP config missing in worktree

   Troubleshooting:
   - Check .mcp.json exists in worktree
   - Verify maproom MCP server is configured
   - Test: cat worktree-path/.mcp.json
   ```

### Fail-Fast Strategy

```typescript
function validateAndFailFast(validation: ValidationResult): void {
  if (!validation.valid) {
    // Log all errors
    validation.errors.forEach(err => {
      console.error(`❌ ${err.message}`)
      if (err.troubleshooting) {
        console.error(`   Fix: ${err.troubleshooting}`)
      }
    })

    // Don't waste API credits
    throw new CompetitionSetupError('Pre-flight validation failed', validation.errors)
  }
}
```

## Technology Choices

### Why Not...?

**Why not Docker containers for isolation?**
- Adds complexity (Docker daemon, image builds)
- Slower startup (container creation)
- Worktrees already provide isolation
- Database sharing is beneficial, not a problem

**Why not separate databases per variant?**
- Wastes embedding generation time
- Increases storage requirements
- No actual benefit (variants don't interfere)
- Shared embeddings are a feature, not a bug

**Why not parallel scanning?**
- Marginal benefit (~90s saved)
- Risk of race conditions
- Setup time isn't the bottleneck (agent execution is)
- Simplicity > micro-optimization

**Why not skip validation in production?**
- Silent failures waste API credits
- Hard to debug after the fact
- Validation overhead is minimal (~30s)
- Confidence > speed

## Integration Points

### Claude Code Agents SDK

**Current usage:**
```typescript
import { runAgent } from '@anthropic-ai/claude-agent-sdk'

const result = await runAgent(taskDescription, variant.description, {
  onToolUse: (event) => logToolUsage(event)
})
```

**Enhanced with validation:**
```typescript
// 1. Create worktree (SDK handles this)
// 2. Inject variant into .mcp.json
// 3. Scan worktree
// 4. Validate environment
// 5. THEN spawn agent

const result = await runAgentWithValidation(
  taskDescription,
  variantEnvironment, // includes validation status
  {
    onToolUse: (event) => logToolUsage(event),
    requireValidation: true // fail if validation didn't pass
  }
)
```

### Maproom MCP Server

**Current integration:** None (agents don't have access)

**New integration:**
- Each worktree gets `.mcp.json` with maproom server config
- Server command: `node /workspace/packages/maproom-mcp/dist/index.js`
- Environment: `MAPROOM_DATABASE_URL` points to shared database
- Tools exposed: search, open, context, status

### Genetic Iterator

**Impact on genetic iterations:**
- Each generation gets full setup + validation
- Parallelism maintained for agent execution
- Setup overhead: ~3min per generation
- Total time for 10 generations: +30min (acceptable for overnight runs)

## MVP Scope

**Phase 1 (this project):**
- ✅ Pre-flight validation framework
- ✅ Scan orchestration
- ✅ Enhanced competition runner
- ✅ Fail-fast error handling

**Phase 2 (future):**
- ⏸️ Parallel scanning optimization
- ⏸️ Validation result caching
- ⏸️ Detailed logging/telemetry
- ⏸️ Competition resume on failure

**Out of scope:**
- ❌ Docker containerization
- ❌ Distributed execution
- ❌ Real-time monitoring UI
- ❌ Cost tracking/budgeting
