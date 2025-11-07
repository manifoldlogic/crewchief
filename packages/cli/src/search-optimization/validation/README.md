# Task Validation Infrastructure

Validates search tasks across 5 quality dimensions to ensure benchmark suite reliability and validity.

## Overview

This is **validation infrastructure** for task design quality, not for running expensive LLM benchmarks. By default, it operates in **mock mode**, using expected metrics from task definitions to validate design quality without API costs.

The framework includes three major components:

- **Task Validator** (`task-validator.ts`) - Core validation logic across 5 dimensions
- **Ecological Validator** (`ecological.ts`) - Real-world scenario validation
- **Report Generator** (`reporter.ts`) - Comprehensive validation reports

## The 5 Validation Dimensions

### 1. Construct Validity (Grep Baseline)

**Question**: Is the task appropriately difficult for grep?

- **Tier 1**: Grep should mostly fail (<30% success)
- **Tier 2**: Grep can succeed sometimes (<60% success)
- **Tier 3**: More relaxed (<80% success)

**In mock mode**: Uses `task.expectedGrepSuccess`

### 2. Discriminant Validity (Search Advantage)

**Question**: Does semantic search provide meaningful advantage?

- Search must succeed at high rate (>70%)
- Improvement must be substantial (>30pp for Tier 1, >20pp for Tier 2)
- Difference must be statistically significant

**In mock mode**: Uses `task.expectedSearchSuccess` and calculates advantage

### 3. Ecological Validity (Realism)

**Question**: Does this reflect realistic developer scenarios?

- Checks for `basedOnRealScenario: true` flag
- Checks for concrete, detailed task descriptions
- Requires context explaining real-world usage

**Always manual review**: Cannot be fully automated

### 4. Test-Retest Reliability (Variance)

**Question**: Are results consistent across runs?

- Coefficient of variation should be <10%
- Objective validators (code_change, file_creation) more reliable than subjective (explanation)

**In mock mode**: Estimates variance based on validator type

### 5. Statistical Power (Sample Size)

**Question**: Is the sample size adequate?

- Minimum n >= 5 for basic statistical power
- Simple pass/fail check

## Usage

### Validate a Single Task

```typescript
import { validateTask, formatValidationReport } from './validation'

const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  iterations: 5,
  useMockData: true, // Fast, no API calls
})

console.log(formatValidationReport(result))

// Check specific dimensions
if (!result.dimensions.constructValidity.passed) {
  console.log('Task too easy for grep!')
}

// Get recommendations
console.log(result.recommendations)
```

### Validate an Entire Suite

```typescript
import { validateSuite, formatSuiteValidationReport } from './validation'
import { TIER1_GREP_IMPOSSIBLE_SUITE } from '../benchmarks/tier1-impossible'

const result = await validateSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
  iterations: 5,
  useMockData: true,
})

console.log(formatSuiteValidationReport(result))

// Check aggregate statistics
console.log(`Pass rate: ${(result.passedTasks / result.totalTasks) * 100}%`)

// Find failed tasks
const failedTasks = result.taskResults.filter((r) => !r.passed)
for (const task of failedTasks) {
  console.log(`${task.task.id}: ${task.recommendations.join(', ')}`)
}
```

### Generate Validation Reports

```typescript
import { ReportGenerator } from './validation'
import { validateSuite } from './validation'
import { TIER1_SUITE } from '../benchmarks/tier1-impossible'

// Validate suite
const suiteResult = await validateSuite(TIER1_SUITE)

// Generate markdown report
const generator = new ReportGenerator({
  format: 'markdown',
  includePatterns: true,
  includeRecommendations: true,
})

const report = generator.generate(suiteResult.taskResults, 'tier1-impossible', 'tier1')

// Save to file
await generator.save(report)
// Output: ./reports/validation-report-tier1-impossible-2025-01-01T00-00-00.md

// Or print to console
generator.print(report)

// Analyze patterns programmatically
if (report.patterns) {
  const tooEasy = report.patterns.find((p) => p.pattern === 'too-easy')
  if (tooEasy) {
    console.log(`${tooEasy.count} tasks are too easy for grep`)
    console.log(`Affected tasks: ${tooEasy.taskIds.join(', ')}`)
  }
}

// Get high priority recommendations
const highPriority = report.recommendations?.filter((r) => r.priority === 'high')
console.log(`${highPriority?.length} tasks need immediate attention`)
```

### Custom Thresholds

```typescript
import { validateTask, DEFAULT_THRESHOLDS } from './validation'

const customThresholds = {
  ...DEFAULT_THRESHOLDS,
  tier1: {
    ...DEFAULT_THRESHOLDS.tier1,
    grepMaxSuccess: 0.4, // More lenient
    minAdvantage: 0.25, // Lower threshold
  },
}

const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  thresholds: customThresholds,
})
```

## Mock Mode vs Real Mode

### Mock Mode (Default, Recommended)

```typescript
const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  useMockData: true, // Default
})
```

**Advantages**:

- Fast (milliseconds)
- Deterministic
- No API costs
- Suitable for CI/CD
- Validates task design quality

**Limitations**:

- Uses expected metrics, not actual measurements
- Mocks statistical tests
- Cannot verify actual agent performance

### Real Mode (Manual Only)

```typescript
const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  useMockData: false, // EXPENSIVE!
})
```

**Requires**:

- Running actual baseline-runner benchmarks
- LLM agent execution
- Real API calls ($$$)
- Multiple iterations for variance

**Status**: Not implemented in this phase. Use baseline-runner separately for actual execution.

## Interpreting Results

### All Dimensions Pass

```
Status: ✓ PASSED
```

Task meets all quality criteria. Ready for benchmarking.

### Construct Validity Fails

```
[✗] Construct Validity (Grep Baseline)
    Actual:   0.5
    Expected: ≤ 0.3
```

**Problem**: Task too easy for grep
**Solution**: Add indirection, require transitive relationships, use conceptual queries

### Discriminant Validity Fails

```
[✗] Discriminant Validity (Search Advantage)
    Actual:   60% (Δ +10pp)
    Expected: ≥ 70% (Δ ≥ 30pp)
```

**Problem**: Search advantage too small
**Solution**: Either make task harder for grep or ensure tools support semantic approach

### Ecological Validity Fails

```
[✗] Ecological Validity (Realism)
    Actual:   Synthetic
    Expected: Based on real developer scenarios
```

**Problem**: Task doesn't reflect real usage
**Solution**: Base on actual scenarios, add context, mark with `basedOnRealScenario: true`

### Reliability Fails

```
[✗] Test-Retest Reliability
    Actual:   CV = 12.0%
    Expected: CV ≤ 10%
```

**Problem**: Results too variable
**Solution**: Use objective validators (code_change over explanation), tighten criteria

### Statistical Power Fails

```
[✗] Statistical Power
    Actual:   n = 3
    Expected: n ≥ 5
```

**Problem**: Sample size too small
**Solution**: Increase iterations to at least 5

## Examples

Run the example file to see validation in action:

```bash
npx tsx src/search-optimization/validation/example.ts
```

## Integration with Workflow

1. **Task Design**: Create task in `tasks/` directory
2. **Validate Design**: Run validator in mock mode
   ```typescript
   const result = await validateTask({ task, tier: 'tier1-impossible' })
   ```
3. **Fix Issues**: Address failed dimensions based on recommendations
4. **Add to Suite**: Once validated, add to benchmark suite
5. **Real Validation** (manual): Eventually run baseline-runner for actual metrics

## Report Formats

The Report Generator supports three output formats:

### Markdown (Primary)

Human-readable reports with:

- Summary statistics and pass/fail counts
- Dimension breakdown table
- Per-task results with status indicators (✅/❌)
- Failure pattern analysis
- Prioritized recommendations

Example output structure:

```markdown
# Validation Report

**Generated:** 2025-01-01T00:00:00.000Z
**Total Tasks:** 10

## Summary

✅ **Overall:** 8/10 tasks passed (80.0%)

## Per-Task Results

### ✅ TASK-001: Find transitive dependencies

| Dimension             | Status | Actual        | Expected |
| --------------------- | ------ | ------------- | -------- |
| Construct Validity    | ✅     | 25%           | ≤ 30%    |
| Discriminant Validity | ✅     | 75% (Δ +50pp) | ≥ 70%    |

...

## Failure Patterns

### Tasks that are too easy for grep

**Count:** 2 tasks
**Affected Tasks:** TASK-002, TASK-005
```

### JSON (Structured Data)

Machine-readable format for:

- Programmatic analysis
- CI/CD integration
- External reporting tools
- Data pipelines

```json
{
  "metadata": {
    "timestamp": "2025-01-01T00:00:00.000Z",
    "version": "1.0.0",
    "totalTasks": 10
  },
  "summary": {
    "total": 10,
    "passed": 8,
    "failed": 2,
    "passRate": 80,
    "dimensionBreakdown": { ... }
  },
  "patterns": [ ... ],
  "recommendations": [ ... ]
}
```

### Console (Quick View)

Terminal-friendly output for:

- Quick validation checks
- Development workflow
- CI/CD status summaries

```
════════════════════════════════════════════════════════════════════════════════
VALIDATION REPORT: tier1-impossible
════════════════════════════════════════════════════════════════════════════════

✅ Overall: 8/10 passed (80.0%)

Dimension Status:
  ✅ Construct Validity: 9/10 (90%)
  ⚠️  Discriminant Validity: 8/10 (80%)
  ✅ Ecological Validity: 10/10 (100%)

Failed Tasks (2):
  ❌ TASK-002: Find code patterns
     Failed: Construct Validity, Discriminant Validity
```

## Report Files

Generated reports are saved to:

```
src/search-optimization/validation/reports/
├── validation-report-tier1-impossible-2025-01-01T00-00-00.md
├── validation-report-tier2-hard-2025-01-01T12-30-00.md
└── ...
```

File naming: `validation-report-{suite-name}-{timestamp}.{md|json}`

## API Reference

### Types

```typescript
interface ValidationConfig {
  task: SearchTask
  tier: 'tier1-impossible' | 'tier2-hard' | 'tier3-realworld'
  iterations?: number // Default: 5
  thresholds?: ValidationThresholds
  useMockData?: boolean // Default: true
}

interface ValidationResult {
  task: SearchTask
  passed: boolean
  tier: string
  dimensions: {
    constructValidity: DimensionResult
    discriminantValidity: DimensionResult
    ecologicalValidity: DimensionResult
    reliability: DimensionResult
    statisticalPower: DimensionResult
  }
  recommendations: string[]
  timestamp: Date
}

interface DimensionResult {
  dimension: string
  passed: boolean
  actual: number | string
  expected: number | string
  details: string
}
```

### Functions

#### `validateTask(config: ValidationConfig): Promise<ValidationResult>`

Validate a single task across all 5 dimensions.

#### `validateSuite(suite: BenchmarkSuite, config?): Promise<SuiteValidationResult>`

Validate all tasks in a benchmark suite.

#### `formatValidationReport(result: ValidationResult): string`

Format individual task validation as human-readable report.

#### `formatSuiteValidationReport(result: SuiteValidationResult): string`

Format suite validation as human-readable report.

## Default Thresholds

```typescript
const DEFAULT_THRESHOLDS = {
  tier1: {
    grepMaxSuccess: 0.3, // 30%
    searchMinSuccess: 0.7, // 70%
    minAdvantage: 0.3, // 30pp
    maxVariance: 0.1, // 10%
    minPValue: 0.05, // p < 0.05
  },
  tier2: {
    grepMaxSuccess: 0.6, // 60%
    searchMinSuccess: 0.7, // 70%
    minAdvantage: 0.2, // 20pp
    maxVariance: 0.1, // 10%
    minPValue: 0.05, // p < 0.05
  },
  tier3: {
    grepMaxSuccess: 0.8, // 80%
    searchMinSuccess: 0.6, // 60%
    minAdvantage: 0.1, // 10pp
    maxVariance: 0.15, // 15%
    minPValue: 0.05, // p < 0.05
  },
}
```

## Testing

Comprehensive test suite with 59 tests:

```bash
pnpm test src/search-optimization/validation
```

Tests cover:

- Each validation dimension (pass/fail cases)
- Tier threshold differences
- Batch suite validation
- Report generation
- Recommendation generation
- Edge cases (missing fields, custom thresholds)

## Philosophy

This validator embodies the principle:

> **Validate design quality cheaply before expensive execution**

By using mock mode with expected metrics, we can:

1. Catch design flaws early (task too easy, advantage too small)
2. Iterate quickly on task design
3. Ensure consistency across benchmark suite
4. Save API costs by validating before running
5. Build confidence in benchmark quality

Real execution validation comes later, after design is validated.
