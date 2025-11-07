# Optimization Tracking System

Architecture documentation for the genetic optimization winner tracking and leaderboard system.

## Overview

The optimization tracking system provides comprehensive infrastructure for tracking, comparing, and managing variants across multiple genetic optimization runs. It consists of three main components:

1. **Leaderboard** - Global top 10 variants across all runs
2. **Production Variant System** - Deployment management and rollback
3. **Run Registry** - Historical tracking and learnings extraction

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                  Genetic Iterator                            │
│  (runGeneticIterations)                                      │
└────────────────┬────────────────────────────────────────────┘
                 │
                 │ registers run at start
                 │ updates on completion
                 │ saves winners to leaderboard
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│              Tracking System                                 │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Leaderboard  │  │  Production  │  │ Run Registry │      │
│  │              │  │              │  │              │      │
│  │ • Top 10     │  │ • Current    │  │ • All runs   │      │
│  │ • Ranking    │  │ • History    │  │ • Learnings  │      │
│  │ • Scores     │  │ • Rollback   │  │ • Comparison │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                               │
└─────────────────────────────────────────────────────────────┘
                 │
                 │ writes to
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│              File System                                     │
│  .crewchief/                                                 │
│  ├── leaderboard.json                                        │
│  ├── optimization-runs/                                      │
│  │   └── index.json                                          │
│  └── production/                                             │
│      ├── current.json                                        │
│      ├── deployment-log.md                                   │
│      └── variants/                                           │
│          └── {id}.json                                       │
└─────────────────────────────────────────────────────────────┘
```

## Data Structures

### Leaderboard Schema

```typescript
interface Leaderboard {
  schemaVersion: number
  allTimeTopVariants: LeaderboardEntry[]
  productionVariant: string | null
  productionDeployedAt: Date | null
  lastUpdated: Date
}

interface LeaderboardEntry {
  rank: number
  variantId: string
  name: string
  compositeScore: number
  tierScores: {
    tier1: number
    tier2: number
    tier3: number
  }
  runId: string
  generation: number
  converged: boolean
  timestamp: Date
  taskCoverage: {
    total: number
    passed: number
  }
  toolSelectionAccuracy: number
}
```

### Production Pointer Schema

```typescript
interface ProductionPointer {
  schemaVersion: number
  currentVariantId: string
  deployedAt: Date
  deployedBy?: string
  reason?: string
  previousVariantId?: string // For rollback
}
```

### Run Registry Schema

```typescript
interface RunRegistry {
  schemaVersion: number
  runs: OptimizationRun[]
  lastUpdated: Date
}

interface OptimizationRun {
  runId: string
  startedAt: Date
  completedAt: Date | null
  status: 'running' | 'completed' | 'failed'
  convergenceReached: boolean
  bestVariant: {
    id: string
    name: string
    score: number
    generation: number
  }
  config: IterationConfig
  learnings: RunLearnings | null
  generations: number
  finalAvgScore: number
  multiTierEnabled: boolean
}
```

### Learnings Schema

```typescript
interface RunLearnings {
  bestMutationTypes: Array<{
    type: MutationType
    avgImprovement: number
    successRate: number
  }>
  convergencePattern: {
    generationsToConverge: number | null
    plateauDetected: boolean
    finalImprovement: number
  }
  taskCoverageTrends: {
    startingPassRate: number
    finalPassRate: number
    improvement: number
  }
  scoreVelocity: {
    avgImprovementPerGeneration: number
    bestGenerationImprovement: number
    worstGenerationImprovement: number
  }
  successfulParameters: {
    populationSize: number
    mutationRate: number
    convergenceThreshold: number
  }
  insights: string[]
}
```

## File System Layout

```
.crewchief/
├── leaderboard.json              # Global top 10 variants
│
├── optimization-runs/
│   ├── index.json                # Run registry
│   └── run-{timestamp}/          # Per-run results (existing)
│       ├── generations/
│       ├── final-results.json
│       └── convergence-log.json
│
└── production/
    ├── current.json              # Current production pointer
    ├── deployment-log.md         # Historical deployments
    └── variants/
        ├── {variant-id-1}.json   # Production variant copies
        └── {variant-id-2}.json
```

## Core Operations

### Leaderboard Management

#### Update Leaderboard
```typescript
const leaderboard = updateLeaderboard(
  variant,
  multiTierScore,
  runId,
  converged,
  baseDir
)
```

**Algorithm:**
1. Create new LeaderboardEntry from variant and score
2. Add to allTimeTopVariants array
3. Sort by compositeScore (descending)
4. Keep only top 10
5. Reassign ranks (1-10)
6. Save atomically using write-then-rename

**Atomic Write Pattern:**
```typescript
// Write to temporary file
writeFileSync(`${path}.tmp`, JSON.stringify(data))

// Atomic rename
renameSync(`${path}.tmp`, path)
```

This prevents corruption if process crashes during write.

#### Save to Leaderboard
```typescript
const result = saveToLeaderboard(
  variant,
  multiTierScore,
  runId,
  converged,
  baseDir
)
```

**Qualification Check:**
- If leaderboard has < 10 entries: Always qualifies
- If leaderboard has 10 entries: Must exceed 10th place score
- Returns null if doesn't qualify

### Production Management

#### Promote to Production
```typescript
const pointer = promoteToProduction(
  variant,
  reason,
  deployedBy,
  baseDir
)
```

**Steps:**
1. Load current production (for previousVariantId)
2. Copy variant JSON to `production/variants/{id}.json`
3. Create ProductionPointer with metadata
4. Save pointer to `production/current.json`
5. Update leaderboard.productionVariant
6. Append deployment entry to `deployment-log.md`

**Deployment Log Format:**
```markdown
### Deployment: 2025-11-07T12:34:56.789Z

- **Action**: promote
- **Variant**: variant-xyz
- **Previous**: variant-abc
- **Reason**: Improved accuracy by 5%
- **Deployed By**: alice@example.com
```

#### Rollback Production
```typescript
const pointer = rollbackProduction(
  reason,
  deployedBy,
  baseDir
)
```

**Steps:**
1. Load current production pointer
2. Verify previousVariantId exists
3. Load previous variant from `production/variants/`
4. Create new pointer with swapped IDs
5. Update leaderboard.productionVariant
6. Append rollback entry to deployment log

### Run Registry

#### Register Run
```typescript
const run = registerRun(runId, config, baseDir)
```

Called at start of `runGeneticIterations()`:
- Creates OptimizationRun with status='running'
- Adds to registry
- Saves immediately for crash recovery

#### Update Run Status
```typescript
const run = updateRunStatus(
  runId,
  status,
  history,
  baseDir
)
```

Called at end of `runGeneticIterations()`:
- Sets status='completed' or 'failed'
- Extracts best variant info
- Calls `extractLearnings(history)`
- Updates registry

#### Extract Learnings
```typescript
const learnings = extractLearnings(history)
```

**Analyzes history to extract:**

1. **Mutation Performance**
   - Track which mutation types led to improvements
   - Calculate avg improvement and success rate per type
   - Rank by effectiveness

2. **Convergence Pattern**
   - Detect plateau (3+ generations with <1% improvement)
   - Track generations to convergence
   - Measure final improvement rate

3. **Task Coverage Trends**
   - Compare starting vs final pass rates
   - Calculate improvement percentage
   - Identify coverage gaps

4. **Optimization Velocity**
   - Average improvement per generation
   - Best/worst generation improvements
   - Detect acceleration or deceleration

5. **Insights Generation**
   - Human-readable observations
   - Recommendations for future runs
   - Parameter effectiveness notes

#### Compare Runs
```typescript
const comparison = compareRunResults(
  runId1,
  runId2,
  baseDir
)
```

Generates side-by-side comparison report:
- Performance metrics (scores, convergence)
- Learnings comparison
- Configuration differences
- Recommendations

## Integration with Genetic Iterator

### Registration Point

```typescript
export async function runGeneticIterations(config: IterationConfig) {
  const baseDir = config.baseDir || join('.crewchief', 'genetic-iterations', `run-${Date.now()}`)
  const runId = baseDir.split('/').pop() || `run-${Date.now()}`

  // Register run at start
  registerRun(runId, config, '.crewchief')

  // ... iterations ...
}
```

### Convergence Point

```typescript
if (hasConverged) {
  const finalHistory: IterationHistory = { /* ... */ }

  // Update run status
  updateRunStatus(runId, 'completed', finalHistory, '.crewchief')

  // Save to leaderboard if multi-tier
  if (config.multiTier?.enabled && multiTierScores) {
    const bestMultiTierScore = multiTierScores.get(bestVariant.id)
    if (bestMultiTierScore) {
      saveToLeaderboard(bestVariant, bestMultiTierScore, runId, true, '.crewchief')
    }
  }

  return finalHistory
}
```

### Max Iterations Point

```typescript
// Max iterations reached
const finalHistory: IterationHistory = { /* ... */ }

// Update run status
updateRunStatus(runId, 'completed', finalHistory, '.crewchief')

// Save to leaderboard if multi-tier
const lastGen = history[history.length - 1]
if (config.multiTier?.enabled && lastGen?.multiTierScores) {
  const bestMultiTierScore = lastGen.multiTierScores.get(bestOverall.id)
  if (bestMultiTierScore) {
    saveToLeaderboard(bestOverall, bestMultiTierScore, runId, false, '.crewchief')
  }
}
```

## Data Safety

### Atomic Writes

All file operations use write-then-rename pattern:

```typescript
// WRONG - Can corrupt file if crash during write
writeFileSync(path, data)

// RIGHT - Atomic operation, no corruption
writeFileSync(`${path}.tmp`, data)
renameSync(`${path}.tmp`, path)
```

### Concurrent Write Handling

- Atomic renames prevent partial writes
- File system guarantees rename atomicity
- Last-write-wins for concurrent updates
- No explicit locking needed

### Schema Versioning

All schemas include `schemaVersion` field:

```typescript
interface Leaderboard {
  schemaVersion: number  // Current: 1
  // ...
}
```

Enables future migrations:

```typescript
if (data.schemaVersion < CURRENT_VERSION) {
  data = migrateSchema(data, CURRENT_VERSION)
}
```

## Usage Examples

### View Leaderboard

```typescript
import { generateLeaderboardReport } from './tracking'

const report = generateLeaderboardReport('.crewchief')
console.log(report)
```

Output:
```
GENETIC OPTIMIZATION LEADERBOARD
================================================================================

Last Updated: 11/7/2025, 12:34:56 PM
Total Entries: 10
Production Variant: variant-alpha-v2
  Deployed: 11/6/2025, 10:15:23 AM

TOP VARIANTS
--------------------------------------------------------------------------------
1. Variant Alpha v2 [PRODUCTION]
   Composite Score: 87.5%
   Tier Scores: T1=85% T2=88% T3=89%
   Tool Selection: 91% accurate
   Task Coverage: 26/30 passed
   Generation: 5 | Converged: Yes
   Run ID: run-1730987654321
   Timestamp: 11/6/2025, 10:15:23 AM
```

### Promote to Production

```typescript
import { promoteToProduction, loadVariant } from './tracking'

const variant = await loadVariant('variant-alpha-v2')
const pointer = promoteToProduction(
  variant,
  'Improved accuracy by 5%',
  'alice@example.com',
  '.crewchief'
)
```

### Compare Runs

```typescript
import { compareRunResults } from './tracking'

const comparison = compareRunResults(
  'run-1730987654321',
  'run-1730991234567',
  '.crewchief'
)

console.log(comparison)
```

### Export Learnings

```typescript
import { exportLearnings } from './tracking'

const learnings = exportLearnings('run-1730987654321', '.crewchief')
console.log(learnings)
```

Output:
```
LEARNINGS FROM RUN: run-1730987654321
================================================================================

Run Date: 11/6/2025, 10:00:00 AM
Status: completed
Converged: Yes
Best Variant: Variant Alpha v2 (87.5%)

KEY INSIGHTS
--------------------------------------------------------------------------------
• Convergence reached in 5 generations
• Task completion rate improved by 15.0%
• Best mutation type: amplification (5.00% avg improvement)
• Strong optimization velocity - good parameter choices

MUTATION TYPE PERFORMANCE
--------------------------------------------------------------------------------
1. amplification: 5.00% avg improvement, 80% success rate
2. reframing: 3.50% avg improvement, 75% success rate
3. reduction: 2.00% avg improvement, 60% success rate
```

## Performance Considerations

### File I/O

- All tracking operations are synchronous
- File sizes are small (<1MB typical)
- Atomic writes complete in <10ms
- No performance impact on genetic iterations

### Memory Usage

- Leaderboard: ~10 entries × 500 bytes = 5KB
- Run registry: ~100 runs × 2KB = 200KB
- Production: ~10 variants × 5KB = 50KB
- Total: <1MB in typical usage

### Scaling

- Leaderboard: Hard limit of 10 entries
- Run registry: Grows linearly with runs
  - Recommended: Archive old runs after 6 months
- Production variants: Grows with deployments
  - Recommended: Keep last 20 deployments

## Testing

### Test Coverage

- **Leaderboard**: 15 tests covering ranking, limits, persistence
- **Production**: 17 tests covering promotion, rollback, history
- **Run Registry**: 20 tests covering registration, learnings, comparison

### Test Strategy

All tests use temporary directories:
```typescript
const TEST_BASE_DIR = join('/tmp', 'tracking-test-leaderboard')

beforeEach(() => {
  if (existsSync(TEST_BASE_DIR)) {
    rmSync(TEST_BASE_DIR, { recursive: true, force: true })
  }
  mkdirSync(TEST_BASE_DIR, { recursive: true })
})
```

### Running Tests

```bash
# Run all tracking tests
pnpm test src/search-optimization/tracking

# Run specific test file
pnpm test leaderboard.test.ts

# Watch mode
pnpm test:watch tracking
```

## Future Enhancements

### Planned Features

1. **Web Dashboard**
   - Real-time leaderboard visualization
   - Interactive run comparison
   - Production deployment timeline

2. **Advanced Analytics**
   - Mutation effectiveness heatmaps
   - Convergence prediction models
   - Parameter optimization suggestions

3. **Export Formats**
   - CSV export for analysis
   - JSON API endpoints
   - Markdown reports

4. **Notifications**
   - Slack/Discord webhooks on new leader
   - Email alerts for production deployments
   - Convergence notifications

### Extensibility

The tracking system is designed for extension:

```typescript
// Add custom metrics to leaderboard entries
interface CustomLeaderboardEntry extends LeaderboardEntry {
  customMetric: number
}

// Add custom learnings
interface CustomLearnings extends RunLearnings {
  customInsights: string[]
}
```

## References

- [Genetic Iterator](../packages/cli/src/search-optimization/genetic-iterator.ts)
- [Multi-Tier Scoring](../packages/cli/src/search-optimization/multi-tier-scoring.ts)
- [Variant Types](../packages/maproom-mcp/test/tool-description-optimization/types.ts)

## Change Log

### Version 1.0.0 (2025-11-07)
- Initial implementation
- Leaderboard tracking
- Production variant system
- Run registry with learnings
- Full test coverage
- Integration with genetic iterator
