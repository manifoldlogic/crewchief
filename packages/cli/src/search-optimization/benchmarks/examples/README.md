# Benchmark Suite Runner Examples

This directory contains examples demonstrating how to use the benchmark suite runner.

## Running the Examples

```bash
# From packages/cli directory
pnpm exec tsx src/search-optimization/benchmarks/examples/run-suite-example.ts
```

## What the Examples Show

### Example 1: Mock Execution

- Demonstrates running a suite with mock data
- Uses expected metrics from task definitions
- Good for testing orchestration logic without API costs

### Example 2: Real Results Processing

- Shows how to process actual execution results
- Calculates aggregate metrics
- Validates against acceptance criteria

### Example 3: Execution Modes

- Compares sequential vs parallel execution
- Shows performance differences
- Helps choose the right mode for your use case

### Example 4: Task Analysis

- Analyzes individual task performance
- Identifies best/worst performing tasks
- Groups results by category

## Important Notes

**Mock Data vs Real Execution:**

- The examples use mock data by default (fast, free)
- Real execution requires running baseline-runner.ts (slow, expensive)
- Mock data uses `task.expectedGrepSuccess` and `task.expectedSearchSuccess`

**When to Use Mock vs Real:**

- **Mock:** Testing, development, CI/CD, documentation
- **Real:** Validation, benchmarking, research, publications

## Real Execution Workflow

For actual benchmark execution:

1. **Run Tasks Manually:**

   ```bash
   # Use baseline-runner from TESTDES-1002
   # This is expensive (LLM API calls)
   pnpm exec baseline-runner --suite tier1 --task relationship-transitive-deps
   ```

2. **Collect Results:**
   - Record grep success rate (e.g., 0.15)
   - Record search success rate (e.g., 0.85)
   - Calculate improvement (0.85 - 0.15 = 0.70)

3. **Create TaskResult[]:**

   ```typescript
   const results: TaskResult[] = [
     {
       task: TIER1_GREP_IMPOSSIBLE_SUITE.tasks[0],
       grepSuccess: 0.15,
       searchSuccess: 0.85,
       improvement: 0.7,
     },
     // ... more results
   ]
   ```

4. **Analyze Results:**
   ```typescript
   const aggregate = calculateAggregateMetrics(results)
   const validation = validateSuiteResults(results, suite)
   ```

## Output Interpretation

### Aggregate Metrics

- **Grep avg:** Average success rate with grep (should be <40%)
- **Search avg:** Average success rate with semantic search (should be >70%)
- **Improvement:** Percentage point improvement (search - grep)
- **Tasks defeating grep:** Count where improvement >30%

### Validation Criteria

- **Grep failure criterion:** Grep <40% (proves tasks are grep-hard)
- **Search success criterion:** Search >70% (proves semantic advantage)
- **Task validation:** Each task within ±10% of expected metrics
