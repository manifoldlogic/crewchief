# Benchmark Suite Runner

The suite-runner module orchestrates execution of benchmark suites to compare grep-based vs semantic search performance.

## Overview

Located at: `src/search-optimization/benchmarks/suite-runner.ts`

This module provides:

- **Orchestration scaffolding** for sequential/parallel task execution
- **Aggregate metrics** calculation across all tasks in a suite
- **Validation** against acceptance criteria (TESTDES-2004)
- **Mock execution** for testing without expensive LLM API calls

## Key Concepts

### Mock vs Real Execution

**Mock Execution (Default):**

- Fast and free
- Uses `task.expectedGrepSuccess` and `task.expectedSearchSuccess`
- Perfect for testing orchestration logic, CI/CD, development
- Enabled by default in `runBenchmarkSuite()`

**Real Execution (Manual):**

- Slow and expensive (requires LLM API calls)
- Uses `baseline-runner.ts` from TESTDES-1002
- Actual agent evaluation with tools
- For validation, benchmarking, research, publications

### Why Mock Data?

Running 8 tasks with LLM agents costs:

- ~$0.50-2.00 per task (depending on model and duration)
- 5-10 minutes per task
- Total: **$4-16 and 40-80 minutes** for one suite run

Mock execution:

- Free
- ~100ms for entire suite
- Uses validated expected metrics

## API Reference

### Core Functions

#### `runBenchmarkSuite(suite, config?)`

Main entry point for suite execution. Returns comprehensive results including metrics and validation.

```typescript
const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
  parallel: false, // Sequential vs parallel execution
  iterations: 1, // Number of iterations per task
  useMockData: true, // Use mock vs real execution
})

console.log('Grep avg:', result.aggregate.grepAvgSuccess)
console.log('Search avg:', result.aggregate.searchAvgSuccess)
console.log('Improvement:', result.aggregate.avgImprovement)
console.log('Validation:', result.validation.details)
```

#### `calculateAggregateMetrics(taskResults)`

Computes aggregate metrics from individual task results.

```typescript
const aggregate = calculateAggregateMetrics(taskResults)

// Aggregate includes:
// - grepAvgSuccess: Average grep success rate (0-1)
// - searchAvgSuccess: Average search success rate (0-1)
// - avgImprovement: Average improvement (search - grep)
// - tasksDefeatingGrep: Count where improvement >30%
```

#### `validateSuiteResults(taskResults, suite)`

Validates results against acceptance criteria from TESTDES-2004.

```typescript
const validation = validateSuiteResults(taskResults, suite)

// Checks:
// - meetsGrepFailureCriterion: Grep <40% (proves grep-hard)
// - meetsSearchSuccessCriterion: Search >70% (proves semantic advantage)
// - allTasksValidated: Each task within ±10% of expected
// - details: Array of validation messages
```

#### `formatSuiteSummary(result)`

Formats suite results as human-readable text.

```typescript
const summary = formatSuiteSummary(result)
console.log(summary)

// Output:
// Suite: Tier 1: Grep-Impossible Tasks (v1.0.0)
// Tasks: 8
// Execution time: 92ms
//
// Performance:
//   Grep avg:   23.1%
//   Search avg: 78.8%
//   Improvement: +55.6%
//   Tasks defeating grep: 8/8
//
// Validation:
//   ✓ Grep failure criterion met: 23.1% < 40%
//   ✓ Search success criterion met: 78.8% > 70%
//   ✓ All 8 tasks met expected performance ranges (±10%)
```

### Types

#### `SuiteResult`

Complete result from running a benchmark suite.

```typescript
interface SuiteResult {
  suite: BenchmarkSuite // The suite that was executed
  executionTime: number // Total time in milliseconds
  taskResults: TaskResult[] // Individual task results
  aggregate: AggregateMetrics // Aggregate metrics
  validation: ValidationStatus // Validation results
}
```

#### `TaskResult`

Result for a single task execution.

```typescript
interface TaskResult {
  task: SearchTask // The task that was executed
  grepSuccess: number // Success rate with grep (0-1)
  searchSuccess: number // Success rate with search (0-1)
  improvement: number // searchSuccess - grepSuccess
}
```

#### `AggregateMetrics`

Aggregate metrics across all tasks.

```typescript
interface AggregateMetrics {
  grepAvgSuccess: number // Average grep success
  searchAvgSuccess: number // Average search success
  avgImprovement: number // Average improvement
  tasksDefeatingGrep: number // Count with >30% improvement
}
```

#### `ValidationStatus`

Validation results against acceptance criteria.

```typescript
interface ValidationStatus {
  meetsGrepFailureCriterion: boolean // Grep <40%
  meetsSearchSuccessCriterion: boolean // Search >70%
  allTasksValidated: boolean // All tasks in range
  details: string[] // Validation messages
}
```

## Usage Examples

See `/benchmarks/examples/` for complete working examples:

### Example 1: Basic Usage

```typescript
import { TIER1_GREP_IMPOSSIBLE_SUITE, runBenchmarkSuite } from './benchmarks'

const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE)
console.log(formatSuiteSummary(result))
```

### Example 2: Parallel Execution

```typescript
const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
  parallel: true, // Faster execution
})
```

### Example 3: Process Real Results

```typescript
import { calculateAggregateMetrics, validateSuiteResults } from './benchmarks'

// After running baseline-runner to get actual results
const realResults: TaskResult[] = [
  {
    task: suite.tasks[0],
    grepSuccess: 0.15, // From actual execution
    searchSuccess: 0.85,
    improvement: 0.7,
  },
  // ... more results
]

const aggregate = calculateAggregateMetrics(realResults)
const validation = validateSuiteResults(realResults, suite)
```

### Example 4: Task Analysis

```typescript
const result = await runBenchmarkSuite(suite)

// Find best performing task
const best = result.taskResults.reduce((best, current) => (current.improvement > best.improvement ? current : best))

console.log('Best task:', best.task.name)
console.log('Improvement:', best.improvement)
```

## Real Execution Workflow

For actual benchmark execution with LLM agents:

### 1. Run Tasks with baseline-runner

```bash
# Run individual task
pnpm exec baseline-runner \
  --suite tier1 \
  --task relationship-transitive-deps \
  --tools grep,glob,read

# This will:
# - Spawn an agent with restricted tools
# - Execute the task
# - Record success metrics
# - Save transcript
```

### 2. Collect Results

From baseline-runner output:

```
Task: relationship-transitive-deps
Tools: grep/glob/read
Result: SUCCESS
Search queries: 3
Files examined: 5
Duration: 120s
Success rate: 0.15 (grep baseline)
```

### 3. Create TaskResult Objects

```typescript
const results: TaskResult[] = [
  {
    task: TIER1_GREP_IMPOSSIBLE_SUITE.tasks[0],
    grepSuccess: 0.15, // From grep baseline run
    searchSuccess: 0.85, // From search baseline run
    improvement: 0.7,
  },
  // ... repeat for all tasks
]
```

### 4. Analyze and Validate

```typescript
const aggregate = calculateAggregateMetrics(results)
const validation = validateSuiteResults(results, suite)

if (validation.meetsGrepFailureCriterion && validation.meetsSearchSuccessCriterion && validation.allTasksValidated) {
  console.log('✓ Suite meets all acceptance criteria')
} else {
  console.log('✗ Suite validation failed')
  validation.details.forEach((d) => console.log(d))
}
```

## Acceptance Criteria (TESTDES-2004)

The suite-runner validates against these criteria:

1. **Grep Failure Criterion:** Average grep success <40%
   - Proves tasks are genuinely grep-hard
   - If >40%, tasks are too easy

2. **Search Success Criterion:** Average search success >70%
   - Proves semantic search provides clear advantage
   - If <70%, search isn't effective enough

3. **Individual Task Validation:** Each task within ±10% of expected
   - Ensures consistency with designed metrics
   - Catches outliers or misconfigured tasks

## Integration

The suite-runner integrates with:

- **tier1-impossible.ts:** Suite definitions
- **validation.ts:** Suite composition validation
- **reporter.ts:** Result formatting and reports
- **baseline-runner.ts (TESTDES-1002):** Actual task execution

## Testing

Comprehensive test coverage (27 tests):

```bash
pnpm test src/search-optimization/benchmarks/__tests__/suite-runner.test.ts
```

Tests cover:

- Aggregate metrics calculation
- Validation logic
- Sequential/parallel execution
- Edge cases (empty, all pass, all fail)
- Integration with real suite structure

## Performance

Mock execution is very fast:

- **Sequential:** ~100ms for 8 tasks
- **Parallel:** ~10-20ms for 8 tasks
- **Memory:** Minimal (no LLM calls)

Real execution is slow:

- **Per task:** 5-10 minutes
- **Full suite:** 40-80 minutes
- **API cost:** $0.50-2.00 per task

## Design Decisions

### Why Separate Mock from Real?

1. **Development velocity:** Test orchestration without API costs
2. **CI/CD:** Fast validation in automated pipelines
3. **Documentation:** Runnable examples that don't cost money
4. **Testing:** Predictable results for unit tests

### Why Not Integrate with baseline-runner?

1. **Separation of concerns:** Runner = orchestration, baseline-runner = execution
2. **Flexibility:** Can process results from any source
3. **Cost control:** Explicit decision to run expensive operations
4. **Testability:** Easy to mock and test orchestration logic

### Why ±10% Tolerance?

1. **LLM variability:** Different runs produce different results
2. **Statistical noise:** Small sample sizes have variance
3. **Practical accuracy:** Close enough for validation
4. **False positive prevention:** Too tight causes flaky tests

## Future Enhancements

Potential improvements (not in scope for TESTDES-2004):

1. **Statistical analysis:** Confidence intervals, significance tests
2. **Iteration support:** Run each task N times, aggregate results
3. **Result persistence:** Save/load results to/from JSON
4. **Comparison reports:** Compare multiple suite runs
5. **Visualization:** Charts and graphs of results
6. **Real execution integration:** Direct baseline-runner calls

## See Also

- **TESTDES-2004:** Ticket for this implementation
- **TESTDES-1002:** baseline-runner for actual execution
- **TESTDES-2001, 2002, 2003:** Task definitions
- **/benchmarks/examples/:** Working examples
- **/benchmarks/tier1-impossible.ts:** Suite definition
