# Suite Runner Quick Start

## 30-Second Overview

The suite-runner orchestrates benchmark execution to compare grep vs semantic search. It uses **mock data by default** to avoid expensive LLM API calls.

## Basic Usage

```typescript
import { TIER1_GREP_IMPOSSIBLE_SUITE, runBenchmarkSuite, formatSuiteSummary } from './benchmarks'

// Run suite with mock data (fast, free)
const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE)
console.log(formatSuiteSummary(result))
```

## Output

```
Suite: Tier 1: Grep-Impossible Tasks (v1.0.0)
Tasks: 8
Execution time: 92ms

Performance:
  Grep avg:   23.1%
  Search avg: 78.8%
  Improvement: +55.6%
  Tasks defeating grep: 8/8

Validation:
  ✓ Grep failure criterion met: 23.1% < 40%
  ✓ Search success criterion met: 78.8% > 70%
  ✓ All 8 tasks met expected performance ranges (±10%)
```

## Key Points

### Mock vs Real

| Aspect       | Mock (Default)        | Real (Manual)        |
| ------------ | --------------------- | -------------------- |
| **Speed**    | ~100ms                | 40-80 minutes        |
| **Cost**     | $0                    | $4-16                |
| **Use Case** | Testing, dev, CI      | Validation, research |
| **How**      | `runBenchmarkSuite()` | baseline-runner.ts   |

### When to Use What

**Use Mock For:**

- ✅ Testing orchestration
- ✅ Development
- ✅ CI/CD pipelines
- ✅ Documentation
- ✅ Examples

**Use Real For:**

- ✅ Final validation
- ✅ Benchmark results
- ✅ Research papers
- ✅ Publications

## Common Tasks

### Run Suite

```typescript
const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE)
```

### Get Metrics

```typescript
const { aggregate } = result
console.log('Grep:', aggregate.grepAvgSuccess)
console.log('Search:', aggregate.searchAvgSuccess)
console.log('Improvement:', aggregate.avgImprovement)
```

### Validate Results

```typescript
const { validation } = result
if (validation.meetsGrepFailureCriterion && validation.meetsSearchSuccessCriterion) {
  console.log('✓ Validation passed')
}
```

### Parallel Execution

```typescript
const result = await runBenchmarkSuite(suite, { parallel: true })
```

### Process Real Results

```typescript
const realResults: TaskResult[] = [
  { task, grepSuccess: 0.15, searchSuccess: 0.85, improvement: 0.7 },
  // ... from baseline-runner
]
const aggregate = calculateAggregateMetrics(realResults)
const validation = validateSuiteResults(realResults, suite)
```

## Files

```
benchmarks/
├── suite-runner.ts           # Core module
├── SUITE_RUNNER.md          # Full docs
├── QUICK_START.md           # This file
└── examples/
    ├── run-suite-example.ts # Working examples
    └── README.md            # Examples docs
```

## Learn More

- **Full Docs:** `SUITE_RUNNER.md`
- **Examples:** `examples/run-suite-example.ts`
- **Tests:** `__tests__/suite-runner.test.ts`

## Run Examples

```bash
cd packages/cli
pnpm exec tsx src/search-optimization/benchmarks/examples/run-suite-example.ts
```

## Run Tests

```bash
cd packages/cli
pnpm test src/search-optimization/benchmarks/__tests__/suite-runner.test.ts
```

## API Cheat Sheet

```typescript
// Main function
await runBenchmarkSuite(suite, config?)

// Utility functions
calculateAggregateMetrics(taskResults)
validateSuiteResults(taskResults, suite)
formatSuiteSummary(result)

// Types
SuiteResult     // Complete result
TaskResult      // Single task result
AggregateMetrics // Suite-wide metrics
ValidationStatus // Validation results
SuiteRunConfig   // Execution config
```

## Validation Criteria

- **Grep <40%** - Tasks are grep-hard ✓
- **Search >70%** - Semantic advantage ✓
- **Tasks ±10%** - Within expected range ✓

## Need Help?

- Read `SUITE_RUNNER.md` for complete documentation
- Check `examples/` for working code
- Run tests to see usage patterns
- Review `TESTDES-2004_COMPLETION_SUMMARY.md` for design details
