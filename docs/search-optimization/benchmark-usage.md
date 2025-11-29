# Benchmark Usage Guide: Running and Interpreting Benchmarks

## Overview

This guide explains how to run the grep-impossible benchmark framework, interpret results, and integrate benchmarks with the genetic optimizer. The framework uses a three-tier approach to comprehensively evaluate semantic search performance.

### The Three-Tier Framework

The benchmark suite is organized into three tiers, each measuring different aspects of semantic search value:

**Tier 1: Grep-Impossible Tasks** (Capability)
- Tasks that fundamentally defeat grep
- Grep success: <30%
- Search success: >70%
- Proves semantic search enables capabilities grep cannot provide

**Tier 2: Grep-Hard Tasks** (Efficiency)
- Tasks where grep might succeed but is inefficient
- Grep success: 30-60%
- Search success: >70%, significantly faster
- Proves semantic search provides speed/quality advantages

**Tier 3: Real-World Tasks** (Utility)
- Natural developer scenarios without artificial constraints
- Focus on realistic workflows
- Proves semantic search is adopted voluntarily
- Measures practical utility in authentic contexts

### Purpose

Benchmarks serve three main purposes:

1. **Objective Evaluation**: Measure semantic search performance rigorously
2. **Genetic Optimization**: Guide tool description evolution
3. **External Validation**: Prove value to users and researchers

## Running Individual Tasks

### Quick Task Execution

Run a single task with grep-only tools:

```typescript
import { runBaseline } from '../evaluation/baseline-runner.js'
import { TASK_TRANSITIVE_DEPENDENCIES } from '../tasks/relationship-discovery/transitive-dependencies.js'

const result = await runBaseline({
  task: TASK_TRANSITIVE_DEPENDENCIES,
  timeout: 300,  // 5 minutes
  worktreePath: '/path/to/codebase'
})

console.log('Success:', result.success)
console.log('Duration:', result.metrics.durationSeconds)
console.log('Tool calls:', result.metrics.toolCalls)
console.log('Files examined:', result.metrics.filesExamined)
```

### Running with Search Available

Run the same task with semantic search enabled:

```typescript
import { runWithSearch } from '../evaluation/search-runner.js'

const result = await runWithSearch({
  task: TASK_TRANSITIVE_DEPENDENCIES,
  timeout: 300,
  worktreePath: '/path/to/codebase'
})

console.log('Success:', result.success)
console.log('Search used:', result.metrics.toolCalls['mcp__maproom__search'] || 0)
console.log('Duration:', result.metrics.durationSeconds)
```

### Comparing Grep vs Search

Run side-by-side comparison:

```typescript
import { compareToolConfigurations } from '../evaluation/comparison.js'

const comparison = await compareToolConfigurations({
  task: TASK_TRANSITIVE_DEPENDENCIES,
  configurations: [
    {
      name: 'Grep-only',
      tools: ['Grep', 'Glob', 'Read', 'Bash']
    },
    {
      name: 'Search-available',
      tools: ['Grep', 'Glob', 'Read', 'Bash', 'mcp__maproom__search']
    }
  ],
  worktreePath: '/path/to/codebase'
})

console.log('Grep-only success:', comparison.configurations[0].success)
console.log('Search success:', comparison.configurations[1].success)
console.log('Improvement:', comparison.advantage.successDelta)
console.log('Time saved:', comparison.advantage.timeSaved, 'seconds')
```

### Running Multiple Iterations

For statistical robustness:

```typescript
async function runMultipleIterations(task, n = 5) {
  const grepResults = []
  const searchResults = []

  for (let i = 0; i < n; i++) {
    console.log(`\nIteration ${i + 1}/${n}`)

    // Grep-only
    console.log('  Running grep-only...')
    const grepResult = await runBaseline({ task, timeout: 300 })
    grepResults.push(grepResult)

    // Search available
    console.log('  Running with search...')
    const searchResult = await runWithSearch({ task, timeout: 300 })
    searchResults.push(searchResult)
  }

  // Calculate statistics
  const grepSuccessRate = grepResults.filter(r => r.success).length / n
  const searchSuccessRate = searchResults.filter(r => r.success).length / n
  const avgGrepTime = grepResults.reduce((sum, r) => sum + r.metrics.durationSeconds, 0) / n
  const avgSearchTime = searchResults.reduce((sum, r) => sum + r.metrics.durationSeconds, 0) / n

  return {
    grepSuccessRate,
    searchSuccessRate,
    improvement: searchSuccessRate - grepSuccessRate,
    avgGrepTime,
    avgSearchTime,
    timeSaved: avgGrepTime - avgSearchTime
  }
}

const stats = await runMultipleIterations(TASK_TRANSITIVE_DEPENDENCIES, 5)
console.log('Grep success rate:', (stats.grepSuccessRate * 100).toFixed(0) + '%')
console.log('Search success rate:', (stats.searchSuccessRate * 100).toFixed(0) + '%')
console.log('Improvement:', (stats.improvement * 100).toFixed(0) + 'pp')
console.log('Time saved:', stats.timeSaved.toFixed(1) + 's')
```

## Running Full Validation

### Tier 1 Suite Execution

Run all Tier 1 (grep-impossible) tasks:

```typescript
import { runSuite } from '../evaluation/suite-runner.js'
import { TIER1_GREP_IMPOSSIBLE_SUITE } from '../benchmarks/tier1-impossible.js'

console.log('Running Tier 1: Grep-Impossible Suite')
console.log(`Total tasks: ${TIER1_GREP_IMPOSSIBLE_SUITE.tasks.length}`)

const results = await runSuite({
  suite: TIER1_GREP_IMPOSSIBLE_SUITE,
  configurations: [
    { name: 'grep-only', enableSearch: false },
    { name: 'search-available', enableSearch: true }
  ],
  iterations: 5,  // Run each task 5 times for statistical power
  worktreePath: '/path/to/codebase',
  outputDir: '.crewchief/benchmark-results/tier1'
})

console.log('\nSuite Results:')
console.log('Tasks passed (grep-only):', results.configurations[0].passedTasks)
console.log('Tasks passed (search):', results.configurations[1].passedTasks)
console.log('Average improvement:', (results.overallAdvantage.successDelta * 100).toFixed(0) + 'pp')
console.log('Statistical significance: p =', results.statisticalTest.p.toFixed(4))
```

### All Three Tiers

Run complete benchmark suite:

```typescript
import { TIER1_GREP_IMPOSSIBLE_SUITE } from '../benchmarks/tier1-impossible.js'
import { TIER2_GREP_HARD_SUITE } from '../benchmarks/tier2-hard.js'
import { TIER3_REALWORLD_SUITE } from '../benchmarks/tier3-realworld.js'

async function runFullBenchmark() {
  console.log('Running Full Benchmark Suite\n')
  console.log('This will take significant time and API credits...\n')

  const results = []

  // Tier 1: Grep-Impossible
  console.log('=== TIER 1: GREP-IMPOSSIBLE ===')
  const tier1Results = await runSuite({
    suite: TIER1_GREP_IMPOSSIBLE_SUITE,
    configurations: [
      { name: 'grep-only', enableSearch: false },
      { name: 'search-available', enableSearch: true }
    ],
    iterations: 5,
    outputDir: '.crewchief/benchmark-results/tier1'
  })
  results.push({ tier: 1, ...tier1Results })

  // Tier 2: Grep-Hard
  console.log('\n=== TIER 2: GREP-HARD ===')
  const tier2Results = await runSuite({
    suite: TIER2_GREP_HARD_SUITE,
    configurations: [
      { name: 'grep-only', enableSearch: false },
      { name: 'search-available', enableSearch: true }
    ],
    iterations: 5,
    outputDir: '.crewchief/benchmark-results/tier2'
  })
  results.push({ tier: 2, ...tier2Results })

  // Tier 3: Real-World
  console.log('\n=== TIER 3: REAL-WORLD ===')
  const tier3Results = await runSuite({
    suite: TIER3_REALWORLD_SUITE,
    configurations: [
      { name: 'grep-only', enableSearch: false },
      { name: 'search-available', enableSearch: true }
    ],
    iterations: 3,  // Fewer iterations for Tier 3 (more variable)
    outputDir: '.crewchief/benchmark-results/tier3'
  })
  results.push({ tier: 3, ...tier3Results })

  return results
}

const fullResults = await runFullBenchmark()

// Generate summary report
console.log('\n=== FULL BENCHMARK SUMMARY ===')
fullResults.forEach(({ tier, overallAdvantage, statisticalTest }) => {
  console.log(`\nTier ${tier}:`)
  console.log(`  Improvement: ${(overallAdvantage.successDelta * 100).toFixed(0)}pp`)
  console.log(`  Time saved: ${overallAdvantage.timeSaved.toFixed(1)}s`)
  console.log(`  p-value: ${statisticalTest.p.toFixed(4)}`)
})
```

### Validation-Only Mode

Validate task quality without expensive execution:

```typescript
import { validateSuite } from '../validation/task-validator.js'

// Fast validation using mock data (no LLM execution)
const validation = await validateSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
  iterations: 5,
  useMockData: true  // Uses task.expectedGrepSuccess/expectedSearchSuccess
})

console.log('Suite validation:', validation.passed ? 'PASSED' : 'FAILED')
console.log('Tasks passed:', validation.passedTasks, '/', validation.totalTasks)

if (!validation.passed) {
  console.log('\nFailed tasks:')
  validation.taskResults
    .filter(r => !r.passed)
    .forEach(r => {
      console.log(`  - ${r.task.name}`)
      console.log(`    Issues:`, r.recommendations.join('; '))
    })
}
```

## Reading and Interpreting Reports

### Task-by-Task Results

Understanding individual task results:

```typescript
// Task result structure
interface TaskResult {
  task: SearchTask
  success: boolean
  metrics: {
    durationSeconds: number
    toolCalls: Record<string, number>
    searchQueries: string[]
    filesExamined: number
  }
  runDir: string
  transcriptPath?: string
}
```

**Interpreting task results**:

```
Task: Find Transitive Dependencies
Status: ✓ SUCCESS
Duration: 45.2s
Tool Calls:
  Grep: 8
  Read: 12
  mcp__maproom__search: 3
Search Queries: ["dependency graph", "import chain", "transitive deps"]
Files Examined: 12
Transcript: .crewchief/baselines/baseline-12345/transcript.md
```

**What to look for**:
- ✓ Success: Task completed, criteria met
- Duration: Time to completion (shorter = more efficient)
- Search usage: Did agent choose semantic search appropriately?
- Files examined: Efficiency indicator (fewer = more targeted)
- Transcript: Full conversation for debugging

### Suite-Level Statistics

Understanding aggregate results:

```typescript
// Suite result structure
interface SuiteResult {
  suite: BenchmarkSuite
  configurations: ConfigurationResult[]
  overallAdvantage: {
    successDelta: number    // Improvement in success rate
    timeSaved: number       // Average time saved per task
    qualityImprovement: number
  }
  statisticalTest: {
    p: number              // Statistical significance
    effectSize: number     // Cohen's d
  }
}
```

**Interpreting suite results**:

```
Tier 1: Grep-Impossible Suite
Total Tasks: 8
Iterations: 5

Grep-only Configuration:
  Success Rate: 22% (9/40 tasks)
  Avg Duration: 125.3s
  Avg Tool Calls: 23.4

Search-available Configuration:
  Success Rate: 78% (31/40 tasks)
  Avg Duration: 67.8s
  Avg Tool Calls: 15.2
  Search Usage: 95% (used in 38/40 tasks)

Overall Advantage:
  Success Improvement: +56pp
  Time Saved: 57.5s per task
  Statistical Significance: p = 0.0003
  Effect Size: d = 1.82 (large)

Conclusion: ✓✓ Search provides critical advantage on grep-impossible tasks
```

**What to look for**:
- Success improvement >30pp for Tier 1
- Statistical significance p < 0.05
- Large effect size (d > 0.8)
- High search usage (>80%) on appropriate tasks

### Failure Pattern Analysis

Understanding why tasks fail:

```typescript
// Failure categories
enum FailureReason {
  TIMEOUT = 'timeout',              // Hit time limit
  WRONG_FILES = 'wrong_files',      // Found wrong files
  INCOMPLETE = 'incomplete',        // Partial success
  VALIDATOR_FAILED = 'validator_failed',  // Criteria not met
  ERROR = 'error'                   // Exception occurred
}
```

**Common failure patterns**:

```
Failure Analysis Report

Failed Tasks: 3/8

1. Task: Find Missing Error Handling
   Reason: incomplete
   Details: Found 2/5 expected files, missed async handlers
   Recommendation: Task may be too broad, consider splitting

2. Task: API Impact Analysis
   Reason: timeout
   Details: Exceeded 300s limit, agent made 45 tool calls
   Recommendation: Increase timeout or simplify task scope

3. Task: Initialization Sequence
   Reason: validator_failed
   Details: Explanation didn't mention "bootstrap" or "startup"
   Recommendation: Validator may be too strict, review criteria
```

**How to use failure analysis**:
- Timeout → Increase time or simplify task
- Wrong files → Improve task description clarity
- Incomplete → Task may be too broad
- Validator failed → Review success criteria
- Multiple failures → Task needs redesign

### Cost Tracking

Understanding API usage:

```typescript
// Cost estimation
interface CostEstimate {
  totalTasks: number
  iterations: number
  avgTokensPerTask: number
  estimatedCost: number
}

function estimateBenchmarkCost(suite: BenchmarkSuite, iterations: number = 5) {
  const avgTokensPerTask = 50000  // Estimate based on task complexity
  const costPer1MTokens = 3.00    // Claude Sonnet pricing

  const totalTasks = suite.tasks.length * iterations * 2  // 2 configs (grep vs search)
  const totalTokens = totalTasks * avgTokensPerTask
  const estimatedCost = (totalTokens / 1_000_000) * costPer1MTokens

  return {
    totalTasks,
    iterations,
    avgTokensPerTask,
    estimatedCost
  }
}

const cost = estimateBenchmarkCost(TIER1_GREP_IMPOSSIBLE_SUITE, 5)
console.log('Estimated cost:', `$${cost.estimatedCost.toFixed(2)}`)
console.log('Total task runs:', cost.totalTasks)
```

**Typical costs**:
- Single task (5 iterations): ~$0.30-0.75
- Tier 1 suite (8 tasks, 5 iterations): ~$12-20
- Full benchmark (30 tasks, 5 iterations): ~$45-75

## Integration with Genetic Optimizer

The benchmark framework integrates with the genetic optimizer to evolve tool descriptions that maximize semantic search adoption and effectiveness.

### Multi-Tier Scoring Approach

The optimizer uses weighted scoring across all three tiers:

```typescript
interface MultiTierScore {
  tier1Score: number    // Capability (weight: 40%)
  tier2Score: number    // Efficiency (weight: 40%)
  tier3Score: number    // Utility (weight: 20%)
  overallScore: number  // Weighted combination
}

function calculateMultiTierScore(results: BenchmarkResults): MultiTierScore {
  const tier1Score = results.tier1.searchSuccessRate - results.tier1.grepSuccessRate
  const tier2Score = results.tier2.searchSuccessRate - results.tier2.grepSuccessRate
  const tier3Score = results.tier3.searchUsageRate  // Voluntary adoption

  const overallScore = (
    tier1Score * 0.40 +  // 40% weight on capability
    tier2Score * 0.40 +  // 40% weight on efficiency
    tier3Score * 0.20    // 20% weight on utility
  )

  return {
    tier1Score,
    tier2Score,
    tier3Score,
    overallScore
  }
}
```

**Why multi-tier scoring?**
- Tier 1: Proves semantic search can do things grep cannot
- Tier 2: Proves semantic search is faster/better than grep
- Tier 3: Proves agents voluntarily adopt semantic search
- Combined: Comprehensive evaluation of real-world value

### Running Genetic Optimization with Benchmarks

```typescript
import { runGeneticOptimization } from '../genetic-iterator.js'
import { TIER1_GREP_IMPOSSIBLE_SUITE } from '../benchmarks/tier1-impossible.js'
import { TIER2_GREP_HARD_SUITE } from '../benchmarks/tier2-hard.js'
import { TIER3_REALWORLD_SUITE } from '../benchmarks/tier3-realworld.js'

const optimization = await runGeneticOptimization({
  // Benchmark suites to evaluate on
  benchmarks: {
    tier1: TIER1_GREP_IMPOSSIBLE_SUITE,
    tier2: TIER2_GREP_HARD_SUITE,
    tier3: TIER3_REALWORLD_SUITE
  },

  // Genetic algorithm parameters
  populationSize: 10,
  maxGenerations: 5,
  mutationRate: 0.3,
  crossoverRate: 0.7,

  // Evaluation settings
  iterationsPerTask: 3,  // Balance cost vs accuracy

  // Scoring weights
  scoring: {
    tier1Weight: 0.40,
    tier2Weight: 0.40,
    tier3Weight: 0.20
  },

  // Output
  outputDir: '.crewchief/optimization-results'
})

console.log('Optimization complete!')
console.log('Best variant:', optimization.bestVariant.description)
console.log('Overall score:', optimization.bestScore.toFixed(3))
console.log('Tier 1 score:', optimization.bestVariant.tier1Score.toFixed(3))
console.log('Tier 2 score:', optimization.bestVariant.tier2Score.toFixed(3))
console.log('Tier 3 score:', optimization.bestVariant.tier3Score.toFixed(3))
```

### Tracking Tool Selection

Monitor when agents choose semantic search vs grep:

```typescript
interface ToolSelectionMetrics {
  totalTasks: number
  searchUsed: number
  searchUsageRate: number
  searchAppropriate: number    // Used on tasks where it helps
  searchInappropriate: number  // Used on tasks where grep is fine
  correctSelectionRate: number
}

function analyzeToolSelection(results: SuiteResult[]): ToolSelectionMetrics {
  let totalTasks = 0
  let searchUsed = 0
  let searchAppropriate = 0
  let searchInappropriate = 0

  for (const suite of results) {
    for (const taskResult of suite.taskResults) {
      totalTasks++

      const usedSearch = taskResult.metrics.toolCalls['mcp__maproom__search'] > 0

      if (usedSearch) {
        searchUsed++

        // Was search appropriate for this task?
        if (suite.tier === 1 || suite.tier === 2) {
          searchAppropriate++
        } else {
          // For Tier 3, check if it actually helped
          if (taskResult.success) {
            searchAppropriate++
          } else {
            searchInappropriate++
          }
        }
      }
    }
  }

  return {
    totalTasks,
    searchUsed,
    searchUsageRate: searchUsed / totalTasks,
    searchAppropriate,
    searchInappropriate,
    correctSelectionRate: searchAppropriate / (searchAppropriate + searchInappropriate)
  }
}
```

**What good tool selection looks like**:
- Tier 1: 90%+ search usage (grep-impossible tasks)
- Tier 2: 70%+ search usage (grep-hard tasks)
- Tier 3: 40%+ search usage (real-world voluntary adoption)
- Correct selection: >80% (used when helpful, avoided when not)

### Evolution Across Generations

Track improvement over genetic iterations:

```typescript
interface GenerationMetrics {
  generation: number
  bestScore: number
  avgScore: number
  bestVariant: ToolDescription
  toolSelection: ToolSelectionMetrics
}

// After optimization completes
const evolution = optimization.generations.map((gen, i) => ({
  generation: i,
  bestScore: gen.bestScore,
  avgScore: gen.avgScore,
  searchUsage: gen.toolSelection.searchUsageRate
}))

console.log('\nEvolution across generations:')
console.log('Gen | Best Score | Avg Score | Search Usage')
console.log('----|------------|-----------|-------------')
evolution.forEach(g => {
  console.log(
    `${g.generation}   | ${g.bestScore.toFixed(3)}      | ${g.avgScore.toFixed(3)}     | ${(g.searchUsage * 100).toFixed(0)}%`
  )
})

// Expected pattern:
// Gen 0: Low score, random search usage
// Gen 1-3: Improving score, increasing appropriate search usage
// Gen 4-5: Plateau at optimal score, high correct search usage
```

## Cost Considerations

### Estimating Costs

Before running expensive benchmarks:

```typescript
function estimateFullBenchmarkCost() {
  const tier1Cost = estimateSuiteCost(TIER1_GREP_IMPOSSIBLE_SUITE, 5)
  const tier2Cost = estimateSuiteCost(TIER2_GREP_HARD_SUITE, 5)
  const tier3Cost = estimateSuiteCost(TIER3_REALWORLD_SUITE, 3)

  const totalCost = tier1Cost + tier2Cost + tier3Cost

  console.log('Cost Estimate:')
  console.log(`  Tier 1: $${tier1Cost.toFixed(2)}`)
  console.log(`  Tier 2: $${tier2Cost.toFixed(2)}`)
  console.log(`  Tier 3: $${tier3Cost.toFixed(2)}`)
  console.log(`  Total: $${totalCost.toFixed(2)}`)

  return totalCost
}

function estimateSuiteCost(suite: BenchmarkSuite, iterations: number) {
  const tasksCount = suite.tasks.length
  const totalRuns = tasksCount * iterations * 2  // grep + search
  const avgCostPerRun = 0.375  // Based on ~50k tokens per task

  return totalRuns * avgCostPerRun
}
```

### When to Run Full Validation

**Always run full validation** (expensive):
- Before major releases
- For publication/external reporting
- After significant framework changes
- When adding new task categories

**Run subset validation** (cheaper):
- During task development
- For quick iterations
- Testing single tasks
- Calibrating parameters

**Use mock validation** (free):
- CI/CD pipeline
- Quick quality checks
- Structural validation
- Before real runs

### Cost-Saving Strategies

**1. Start with mock validation**
```typescript
// Free, instant feedback
const mockResult = await validateTask({
  task: newTask,
  tier: 'tier1-impossible',
  useMockData: true
})

// Only proceed if mock passes
if (mockResult.passed) {
  // Now run expensive real validation
  const realResult = await validateTask({
    task: newTask,
    tier: 'tier1-impossible',
    useMockData: false
  })
}
```

**2. Reduce iterations for development**
```typescript
// Development: 3 iterations (~60% cost savings)
const devResults = await runSuite({
  suite: TIER1_GREP_IMPOSSIBLE_SUITE,
  iterations: 3
})

// Production: 5-10 iterations (full statistical power)
const prodResults = await runSuite({
  suite: TIER1_GREP_IMPOSSIBLE_SUITE,
  iterations: 10
})
```

**3. Sample tasks for quick checks**
```typescript
// Test on representative sample (3 tasks) instead of full suite (8 tasks)
const sampleSuite = {
  ...TIER1_GREP_IMPOSSIBLE_SUITE,
  tasks: [
    TASK_TRANSITIVE_DEPENDENCIES,
    TASK_DATA_FLOW_WORKTREE_CREATION,
    TASK_MISSING_ERROR_HANDLING
  ]
}

const quickResults = await runSuite({
  suite: sampleSuite,
  iterations: 3
})
// ~$3-5 instead of $12-20
```

**4. Use existing results when possible**
```typescript
// Cache results for reuse
const cacheDir = '.crewchief/benchmark-cache'

// Check cache before running
const cachedResult = loadCachedResult(task.id, cacheDir)
if (cachedResult && cachedResult.timestamp > lastCodeChange) {
  console.log('Using cached result')
  return cachedResult
}

// Run and cache
const result = await runTask(task)
saveCachedResult(result, cacheDir)
```

## Cross-Project Validation

Testing task generalization across different codebases:

### Running on Multiple Codebases

```typescript
async function validateCrossProject(task: SearchTask) {
  const codebases = [
    { name: 'crewchief', path: '/workspace/crewchief' },
    { name: 'vscode', path: '/workspace/vscode' },
    { name: 'typescript', path: '/workspace/typescript' }
  ]

  const results = []

  for (const codebase of codebases) {
    console.log(`\nTesting on ${codebase.name}...`)

    const result = await runBaseline({
      task: adaptTaskForCodebase(task, codebase.name),
      worktreePath: codebase.path,
      timeout: 300
    })

    results.push({
      codebase: codebase.name,
      success: result.success,
      metrics: result.metrics
    })
  }

  // Calculate generalization rate
  const successCount = results.filter(r => r.success).length
  const generalizationRate = successCount / results.length

  return {
    results,
    generalizationRate,
    passed: generalizationRate >= 0.6  // 60% threshold
  }
}

const crossProject = await validateCrossProject(TASK_FIND_RETRY_LOGIC)
console.log('Generalization rate:', (crossProject.generalizationRate * 100).toFixed(0) + '%')
console.log('Passed:', crossProject.passed ? 'YES' : 'NO')
```

### Adapting Tasks for Different Codebases

```typescript
function adaptTaskForCodebase(task: SearchTask, codebase: string): SearchTask {
  // Clone task with codebase-specific adaptations
  const adapted = { ...task }

  // Adjust expected files based on codebase structure
  if (codebase === 'vscode') {
    // VSCode uses src/vs/ structure
    adapted.followUpTask.validator.mentionsFiles = [
      'src/vs/base/common/retry.ts'
    ]
  } else if (codebase === 'typescript') {
    // TypeScript uses src/ directly
    adapted.followUpTask.validator.mentionsFiles = [
      'src/services/retry.ts'
    ]
  }

  return adapted
}
```

## Best Practices

### Development Workflow

1. **Start with validation**
   ```typescript
   // Always validate before running expensive benchmarks
   const validation = await validateTask({
     task: newTask,
     tier: 'tier1-impossible',
     useMockData: true
   })

   if (!validation.passed) {
     console.log('Fix issues:', validation.recommendations)
     return
   }
   ```

2. **Iterate cheaply**
   ```typescript
   // Use small iterations during development
   const quickTest = await runBaseline({
     task: newTask,
     timeout: 120,  // 2 minutes for quick feedback
     worktreePath: process.cwd()
   })
   ```

3. **Batch expensive operations**
   ```typescript
   // Run full validation overnight or in CI
   async function nightlyBenchmark() {
     await runFullBenchmark()
     await generateReports()
     await notifyTeam()
   }
   ```

### Failure Analysis Workflow

When tasks fail:

1. **Check transcript**
   ```typescript
   // Read agent's reasoning
   const transcript = readFileSync(result.transcriptPath, 'utf-8')
   console.log('Agent transcript:', transcript)
   // Look for: What did agent try? Why did it fail?
   ```

2. **Analyze tool usage**
   ```typescript
   // Which tools were used?
   console.log('Tool calls:', result.metrics.toolCalls)
   // Grep used 20 times but search 0? Description may not guide search usage
   // Search used but failed? Task may be too hard or validator too strict
   ```

3. **Review success criteria**
   ```typescript
   // Were criteria too strict?
   const validator = task.followUpTask.validator
   console.log('Required files:', validator.mentionsFiles)
   console.log('Required pattern:', validator.mentionsPattern)
   // Relax if needed, but maintain objectivity
   ```

4. **Iterate and retest**
   ```typescript
   // Fix and validate again
   const fixedTask = { ...task, /* changes */ }
   const retest = await runBaseline({ task: fixedTask })
   console.log('Retry success:', retest.success)
   ```

### Contributing Tasks to Suite

1. **Design task** (see [Task Design Guide](./task-design-guide.md))
2. **Validate task** (see [Validation Guide](./validation-guide.md))
3. **Add to suite**
   ```typescript
   // Add to appropriate suite
   const updatedSuite = {
     ...TIER1_GREP_IMPOSSIBLE_SUITE,
     tasks: [
       ...TIER1_GREP_IMPOSSIBLE_SUITE.tasks,
       MY_NEW_TASK
     ]
   }
   ```
4. **Run suite validation**
   ```typescript
   const validation = await validateSuite(updatedSuite)
   if (validation.passed) {
     console.log('Ready to commit!')
   }
   ```

## Command-Line Interface

### Quick Commands

```bash
# Validate a single task (mock mode, fast)
npm run benchmark:validate -- --task tier1-transitive-deps

# Run single task (real mode, expensive)
npm run benchmark:run -- --task tier1-transitive-deps --iterations 5

# Run full Tier 1 suite
npm run benchmark:run -- --suite tier1 --iterations 5

# Run all three tiers
npm run benchmark:run -- --full --iterations 5

# Generate report from existing results
npm run benchmark:report -- --results .crewchief/benchmark-results/tier1
```

### Configuration File

Create `.crewchief/benchmark-config.json`:

```json
{
  "suites": {
    "tier1": {
      "iterations": 5,
      "timeout": 300
    },
    "tier2": {
      "iterations": 5,
      "timeout": 300
    },
    "tier3": {
      "iterations": 3,
      "timeout": 300
    }
  },
  "optimization": {
    "populationSize": 10,
    "maxGenerations": 5,
    "scoring": {
      "tier1Weight": 0.40,
      "tier2Weight": 0.40,
      "tier3Weight": 0.20
    }
  },
  "output": {
    "baseDir": ".crewchief/benchmark-results",
    "cacheResults": true,
    "generateReports": true
  }
}
```

## Troubleshooting

### Common Issues

**Issue: Tasks timing out**
- Increase timeout: `timeout: 600` (10 minutes)
- Simplify task scope
- Check if agent is stuck in loop

**Issue: Low search usage on Tier 1 tasks**
- Tool description may not encourage search
- Task may have obvious keywords
- Check tool selection metrics

**Issue: High variance in results**
- Increase iterations (5 → 10)
- Make success criteria more objective
- Check task description clarity

**Issue: Expensive API costs**
- Use mock validation during development
- Reduce iterations (5 → 3)
- Test on task subset
- Cache results when possible

## Next Steps

- **To design tasks**: See [Task Design Guide](./task-design-guide.md)
- **To validate tasks**: See [Validation Guide](./validation-guide.md)
- **For architecture details**: See [.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md](../../.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md)
- **For genetic optimization**: See `/packages/cli/src/search-optimization/genetic-iterator.ts`

## Summary

The benchmark framework provides:
- **Three-tier evaluation**: Capability, efficiency, utility
- **Statistical rigor**: Multiple iterations, significance testing
- **Cost management**: Mock validation, caching, sampling
- **Genetic optimization**: Multi-tier scoring, tool selection tracking
- **Cross-project validation**: Generalization testing

Use this framework to prove semantic search provides measurable value in real-world developer workflows.
