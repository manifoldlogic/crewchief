# TESTDES-3001: Implement Task Validator

**Status**: 📋 Planned
**Priority**: High
**Complexity**: Medium-High (6-8 hours)
**Phase**: 3 - Validation Infrastructure
**Dependencies**: TESTDES-1003, TESTDES-2004

## Summary

Implement the task validator that ensures tasks meet all quality dimensions before entering the benchmark suite. This is the gatekeeper ensuring only high-quality, scientifically valid tasks make it into our benchmark, preventing "garbage in, garbage out" scenarios.

## Background

The task validator validates tasks across 5 quality dimensions defined in our quality strategy:

1. **Construct Validity**: Does task measure what it claims? (grep-impossible tasks actually defeat grep)
2. **Discriminant Validity**: Does search significantly outperform grep? (statistical significance)
3. **Ecological Validity**: Is task based on real-world scenarios? (developers would actually do this)
4. **Test-Retest Reliability**: Are results consistent? (low variance across runs)
5. **Statistical Power**: Sufficient iterations for significance testing?

This validator is critical infrastructure. Without it, we risk including tasks that:
- Are too easy (grep solves them fine)
- Are too hard (even search fails)
- Have subjective criteria (unreliable scoring)
- Don't generalize (only work on one codebase)
- Lack real-world relevance (synthetic, unrealistic)

From quality-strategy.md (lines 341-348): Tier 1 validation must block deployment if tasks fail grep baseline, search advantage, determinism, or objective criteria checks.

## Acceptance Criteria

- [ ] Grep baseline validation: verifies <30% success rate for Tier 1 tasks (construct validity)
- [ ] Search performance validation: verifies >70% success rate with search available
- [ ] Search advantage validation: verifies statistical significance (p<0.05) and >30% improvement
- [ ] Objective criteria validation: confirms success criteria are deterministic, measurable
- [ ] Reliability validation: verifies <10% variance across 5 runs (test-retest)
- [ ] Statistical power validation: confirms sufficient iterations for t-test validity
- [ ] Validation report generated showing pass/fail for each dimension with recommendations
- [ ] Integration with Tier 1 benchmark suite (TESTDES-2004) to validate all tasks

## Technical Requirements

**Architecture**:
- TypeScript implementation in `packages/cli/src/search-optimization/validation/`
- Uses comparison framework (TESTDES-1003) for baseline vs search comparison
- Generates detailed validation reports with failure categorization
- Integrates with benchmark suite runner for batch validation

**Interfaces**:
```typescript
interface ValidationConfig {
  task: SearchTask
  tier: 'tier1-impossible' | 'tier2-hard' | 'tier3-realworld'
  iterations?: number  // Default 5 for reliability testing
  thresholds?: ValidationThresholds
}

interface ValidationThresholds {
  // Tier 1: Grep-Impossible
  tier1: {
    grepMaxSuccess: 0.3      // <30% grep success
    searchMinSuccess: 0.7    // >70% search success
    minAdvantage: 0.3        // >30% improvement
    maxVariance: 0.1         // <10% variance
    minPValue: 0.05          // p<0.05 significance
  }
  // Tier 2: Grep-Hard
  tier2: {
    grepMaxSuccess: 0.6      // <60% grep success
    searchMinSuccess: 0.7    // >70% search success
    minAdvantage: 0.2        // >20% improvement
    maxVariance: 0.1
    minPValue: 0.05
  }
}

interface ValidationResult {
  task: SearchTask
  passed: boolean
  tier: string

  dimensions: {
    constructValidity: DimensionResult      // Grep baseline check
    discriminantValidity: DimensionResult   // Search advantage check
    ecologicalValidity: DimensionResult     // Realism check (manual/future)
    reliability: DimensionResult            // Variance check
    statisticalPower: DimensionResult       // Sample size check
  }

  recommendations: string[]  // What to fix if failed
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

**Quality Dimensions Implementation**:

1. **Construct Validity (Grep Baseline)**:
   - Run task with grep/glob/read only
   - Measure success rate across iterations
   - Pass if: Tier 1 <30%, Tier 2 <60%
   - From quality-strategy.md lines 20-36

2. **Discriminant Validity (Search Advantage)**:
   - Use comparison framework (TESTDES-1003)
   - Calculate advantage metrics
   - Require p<0.05 statistical significance
   - Require >30% improvement for Tier 1
   - From quality-strategy.md lines 38-60

3. **Objective Criteria Validation**:
   - Check task has defined successValidator
   - Verify criteria are binary/numeric (not subjective)
   - Confirm deterministic scoring
   - From quality-strategy.md lines 163-168, 345-346

4. **Test-Retest Reliability**:
   - Run same task 5 times
   - Calculate variance
   - Pass if variance <10%
   - From quality-strategy.md lines 91-113

5. **Statistical Power**:
   - Check minimum iterations (n≥5)
   - Validate sample size for t-test
   - From quality-strategy.md lines 56-58

**Report Generation**:
- Markdown format showing all dimensions
- Pass/fail for each dimension with actual vs expected
- Failure categorization (quality-strategy.md lines 369-401):
  - Type 1: Task too easy (grep succeeds)
  - Type 2: Task too hard (both fail)
  - Type 3: Insufficient advantage (<20% improvement)
  - Type 4: Unreliable results (high variance)
  - Type 5: Ecological invalid (not realistic)
- Actionable recommendations for fixing failures

## Implementation Notes

### Orchestration Pattern
```typescript
async function validateTask(config: ValidationConfig): Promise<ValidationResult> {
  const thresholds = config.thresholds || DEFAULT_THRESHOLDS[config.tier]

  // 1. Construct Validity: Grep baseline check
  const grepBaseline = await runGrepBaseline(config.task, config.iterations)
  const constructValid = grepBaseline.avgSuccess < thresholds.grepMaxSuccess

  // 2. Discriminant Validity: Search advantage check
  const comparison = await runComparison({
    task: config.task,
    iterations: config.iterations
  })
  const searchValid = comparison.searchCondition.avgScore > thresholds.searchMinSuccess
  const advantageValid = comparison.advantage.qualityImprovement > thresholds.minAdvantage
  const significanceValid = comparison.significance.pValue < thresholds.minPValue

  // 3. Objective Criteria: Check task definition
  const objectiveValid = validateObjectiveCriteria(config.task)

  // 4. Test-Retest Reliability: Variance check
  const variance = calculateVariance(grepBaseline.results.map(r => r.score))
  const reliabilityValid = variance < thresholds.maxVariance

  // 5. Statistical Power: Sample size check
  const powerValid = config.iterations >= 5

  return {
    passed: constructValid && searchValid && advantageValid &&
            significanceValid && objectiveValid && reliabilityValid && powerValid,
    dimensions: {
      constructValidity: {
        dimension: 'Construct Validity (Grep Baseline)',
        passed: constructValid,
        actual: grepBaseline.avgSuccess,
        expected: `<${thresholds.grepMaxSuccess}`,
        details: `Grep success rate: ${(grepBaseline.avgSuccess * 100).toFixed(1)}%`
      },
      // ... other dimensions
    },
    recommendations: generateRecommendations(/* results */)
  }
}
```

### Failure Categorization
```typescript
function categorizeFailure(result: ValidationResult): FailureCategory {
  if (!result.dimensions.constructValidity.passed) {
    // Grep succeeds too often
    return {
      type: 'task-too-easy',
      recommendation: 'Add anti-keyword constraints, increase conceptual complexity'
    }
  }

  if (!result.dimensions.discriminantValidity.passed) {
    if (result.comparison.searchCondition.avgScore < 0.5) {
      // Both grep and search fail
      return {
        type: 'task-too-hard',
        recommendation: 'Simplify task, add context, clarify success criteria'
      }
    } else {
      // Search only marginally better
      return {
        type: 'insufficient-advantage',
        recommendation: 'Redesign to emphasize relationships, concepts, complexity'
      }
    }
  }

  if (!result.dimensions.reliability.passed) {
    return {
      type: 'unreliable-results',
      recommendation: 'Make criteria objective, use binary checks'
    }
  }
}
```

### Integration Points
- Uses comparison framework (TESTDES-1003) for statistical analysis
- Uses grep baseline runner (TESTDES-1002) for construct validity
- Integrates with Tier 1 suite (TESTDES-2004) for batch validation
- Generates reports compatible with validation report generator (TESTDES-3003)

### Thresholds by Tier
Based on quality-strategy.md lines 341-366:

**Tier 1 (Grep-Impossible)**:
- Grep success: <30% (blocks deployment)
- Search success: >70%
- Advantage: >30% improvement
- Significance: p<0.05

**Tier 2 (Grep-Hard)**:
- Grep success: 30-60% (warning)
- Search success: >70%
- Advantage: >20% improvement
- Time saved: >50% faster

**Tier 3 (Real-World)**:
- Natural tool selection (no coercion)
- Voluntary search adoption
- Ecological validity positive

## Files to Create/Modify

**New Files**:
- `packages/cli/src/search-optimization/validation/task-validator.ts` - Main validation orchestration
- `packages/cli/src/search-optimization/validation/grep-baseline.ts` - Construct validity check
- `packages/cli/src/search-optimization/validation/search-performance.ts` - Discriminant validity check
- `packages/cli/src/search-optimization/validation/thresholds.ts` - Tier-specific thresholds
- `packages/cli/src/search-optimization/validation/types.ts` - Validation interfaces
- `packages/cli/src/search-optimization/validation/__tests__/task-validator.test.ts` - Integration tests
- `packages/cli/src/search-optimization/validation/__tests__/grep-baseline.test.ts` - Unit tests
- `packages/cli/src/search-optimization/validation/__tests__/search-performance.test.ts` - Unit tests

**Updated Files**:
- `packages/cli/src/search-optimization/validation/index.ts` - Export validation functions
- `packages/cli/src/search-optimization/benchmarks/tier1-impossible.ts` - Integrate validator

## Dependencies

**Required Tickets**:
- TESTDES-1003: Comparison framework (provides statistical analysis)
- TESTDES-2004: Tier 1 benchmark suite (provides tasks to validate)

**Existing Code**:
- `packages/cli/src/search-optimization/evaluation/comparison.ts` - Statistical comparison
- `packages/cli/src/search-optimization/evaluation/baseline-runner.ts` - Grep-only execution
- `packages/cli/src/search-optimization/taxonomy/categories.ts` - Task categories

## Agent Assignments

**Primary Agent**: general-purpose
**Responsibilities**: TypeScript implementation, validation logic, report generation

**Supporting Agents**:
- unit-test-runner: Execute and validate tests
- verify-ticket: Check acceptance criteria
- commit-ticket: Create conventional commit

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Validation too strict | All tasks fail | Start with lenient thresholds, tighten based on data |
| Validation too lenient | Poor tasks enter suite | Base thresholds on quality-strategy.md research |
| Long validation time | Slow task creation | Cache results, parallelize where possible |
| LLM variance | Inconsistent validation | Require multiple iterations (n≥5) |
| Subjective criteria detection | Can't validate objectively | Define clear patterns (binary checks, numeric thresholds) |

## Testing Strategy

**Unit Tests**:
- Threshold validation with known task results
- Failure categorization logic
- Recommendation generation
- Each quality dimension in isolation

**Integration Tests**:
- Full validation flow with real task
- Known grep-easy task should fail validation
- Known grep-impossible task should pass validation
- Batch validation of multiple tasks

**Validation**:
- Run on existing tasks from TESTDES-2004
- Verify thresholds align with quality-strategy.md
- Compare manual analysis vs automated validation

## Success Metrics

- [ ] Can validate a task in <5 minutes (with n=5 iterations)
- [ ] Correctly identifies grep-easy tasks as failing construct validity
- [ ] Correctly identifies unreliable tasks (high variance)
- [ ] Generates actionable recommendations for failures
- [ ] 100% of Tier 1 tasks pass validation (after fixes based on recommendations)
- [ ] Validation reports are clear and interpretable

## References

**Code References**:
- `/workspace/packages/cli/src/search-optimization/evaluation/comparison.ts` - Comparison framework (TESTDES-1003)
- `/workspace/packages/cli/src/search-optimization/evaluation/baseline-runner.ts` - Grep baseline (TESTDES-1002)
- `/workspace/packages/cli/src/search-optimization/validators.ts:222-235` - Success validation patterns

**Planning References**:
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md:10-154` - All 5 quality dimensions
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md:156-196` - Phase 1 unit validation
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md:341-366` - Tier 1 must-have tests
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md:369-401` - Failure analysis framework
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:286-364` - Validation pipeline architecture
- `.agents/projects/TESTDES_grep-impossible-task-design/tickets/TESTDES_TICKET_INDEX.md:82-87` - Ticket context

## Notes

The task validator is the quality gatekeeper for the entire benchmark suite. It implements the rigorous validation methodology from our quality strategy, ensuring every task:
1. Actually defeats grep (construct validity)
2. Benefits significantly from search (discriminant validity)
3. Produces consistent results (reliability)
4. Has objective success criteria (no subjective judgment)
5. Has statistical power (sufficient iterations)

This is Tier 1 validation - tasks must pass ALL checks to enter the benchmark. Without this rigor, our claims about semantic search value lack scientific validity.

Key insight from quality-strategy.md: "Pragmatic rigor over ceremonial coverage. Every validation step must provide actionable insight that improves task quality or proves real-world utility."
