# TESTDES-1003: Implement Comparison Framework with Statistical Analysis

**Status**: ✅ Complete
**Priority**: High
**Complexity**: Medium-High (6-8 hours)
**Phase**: 1 - Foundation
**Dependencies**: TESTDES-1001, TESTDES-1002

## Workflow Status
- [x] Task completed - acceptance criteria met
- [x] Tests pass - all 55 tests passing (37 statistics + 18 comparison)
- [x] Verified - by verify-ticket agent

## Summary

Implement the comparison framework that runs side-by-side evaluations (grep-only vs search-available) and calculates advantage metrics with statistical significance testing. This is the scientific core enabling objective claims about semantic search value.

## Background

The comparison framework validates our hypothesis scientifically. It orchestrates:
1. Grep-only baseline (TESTDES-1002)
2. Search-available condition (existing competition-runner)
3. Advantage metric calculation (time saved, quality improvement)
4. Statistical significance testing (t-tests, confidence intervals)

This enables objective, reproducible claims like "semantic search provides 40% improvement with p<0.05 confidence" instead of subjective "seems better."

Research foundation: A/B testing methodology from HCI research, t-tests from statistics, effect size calculation from meta-analysis.

## Acceptance Criteria

- [x] Side-by-side execution works: runs same task under grep-only and search-available conditions
- [x] Advantage metrics calculated: timeSaved (seconds), qualityImprovement (0-1 score delta), toolSelectionCorrect (boolean)
- [x] Statistical tests implemented: t-test for score difference, confidence intervals, effect size
- [x] Comparison report generated as markdown showing all metrics, tables, significance
- [x] Integration test validates full comparison flow with mock tasks

## Technical Requirements

**Architecture**:
- TypeScript implementation in `packages/cli/src/search-optimization/evaluation/`
- Orchestrates baseline-runner (TESTDES-1002) and competition-runner (existing)
- Implements basic statistical tests (t-test, confidence intervals)

**Interfaces**:
```typescript
interface ComparisonConfig {
  task: SearchTask
  iterations: number  // For statistical power (n≥5 recommended)
  parallelExecution?: boolean
  timeout?: number
}

interface ComparisonResult {
  task: SearchTask
  grepBaseline: {
    results: BaselineResult[]
    avgScore: number
    avgTime: number
  }
  searchCondition: {
    results: CompetitionResult[]
    avgScore: number
    avgTime: number
    searchUsageRate: number
  }
  advantage: {
    timeSaved: number  // seconds
    qualityImprovement: number  // score delta
    toolSelectionCorrect: boolean
  }
  significance: {
    pValue: number
    confidenceInterval: [number, number]
    effectSize: number
    significant: boolean  // p < 0.05
  }
}
```

**Statistical Tests**:
- Independent samples t-test for score comparison
- Cohen's d for effect size
- 95% confidence intervals
- Document assumptions (normality, equal variance)

**Report Generation**:
- Markdown tables comparing conditions
- Visualization-ready data (CSV export optional)
- Interpretation guidance (what p-value means)

## Implementation Notes

### Orchestration Pattern
```typescript
async function runComparison(config: ComparisonConfig): Promise<ComparisonResult> {
  // Run grep-only baseline multiple times
  const grepResults = await runMultiple(config.iterations, () =>
    runBaseline(config.task, { availableTools: ['grep', 'glob', 'read'] })
  )

  // Run search-available condition
  const searchResults = await runMultiple(config.iterations, () =>
    runCompetition({ task: config.task, variants: [currentVariant] })
  )

  return {
    advantage: calculateAdvantage(grepResults, searchResults),
    significance: performTTest(
      grepResults.map(r => r.score),
      searchResults.map(r => r.score)
    )
  }
}
```

### Statistical Validity
- Minimum n=5 iterations for t-test validity
- Report confidence intervals alongside p-values
- Effect size (Cohen's d) shows practical significance
- Document limitations (LLM variance, small samples)

### Integration Points
- Uses baseline-runner from TESTDES-1002
- Uses competition-runner from existing code (packages/cli/src/search-optimization/competition-runner.ts)
- Generates reports compatible with validation framework (Phase 3)

## Files to Create/Modify

**New Files**:
- `packages/cli/src/search-optimization/evaluation/comparison.ts` - Main orchestration
- `packages/cli/src/search-optimization/evaluation/metrics.ts` - Advantage calculation helpers
- `packages/cli/src/search-optimization/evaluation/statistics.ts` - T-test, confidence intervals
- `packages/cli/src/search-optimization/evaluation/__tests__/comparison.test.ts` - Integration tests
- `packages/cli/src/search-optimization/evaluation/__tests__/statistics.test.ts` - Statistical test validation

**Updated Files**:
- `packages/cli/src/search-optimization/evaluation/index.ts` - Export comparison functions

## Dependencies

**Required Tickets**:
- TESTDES-1001: Task taxonomy (provides SearchTask type)
- TESTDES-1002: Baseline runner (provides grep-only execution)

**Existing Code**:
- `packages/cli/src/search-optimization/competition-runner.ts` - Search-available condition
- `packages/cli/src/search-optimization/validators.ts` - Metric calculation patterns (lines 222-235)

## Agent Assignments

**Primary Agent**: general-purpose
**Responsibilities**: TypeScript implementation, statistical tests, report generation

**Supporting Agents**:
- unit-test-runner: Execute and validate tests
- verify-ticket: Check acceptance criteria
- commit-ticket: Create conventional commit

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| High variance with LLM agents | Statistical tests inconclusive | Document confidence intervals, increase iterations if needed |
| Small sample sizes (n<10) | Weak statistical power | Recommend n≥5, document limitations in reports |
| Statistical assumptions violated | Invalid test results | Document assumptions, consider non-parametric alternatives |
| Complex statistics intimidate users | Low adoption | Generate plain-English interpretations |

## Testing Strategy

**Unit Tests**:
- Statistical functions (t-test, confidence intervals) with known datasets
- Metric calculation with mock results
- Report generation with sample data

**Integration Tests**:
- Full comparison flow with real task (simple grep-solvable task)
- Validate all metrics captured correctly
- Verify report generation

**Validation**:
- Compare manual t-test calculation vs implementation
- Verify confidence intervals contain true mean in simulations

## Success Metrics

- [ ] Can run comparison with n=5 iterations in reasonable time (<30 min)
- [ ] Statistical tests match reference implementations (R, Python scipy)
- [ ] Reports are interpretable by non-statisticians
- [ ] Integration test demonstrates significant difference on known grep-impossible task

## References

**Code References**:
- `/workspace/packages/cli/src/search-optimization/competition-runner.ts:113-178` - Agent spawning pattern
- `/workspace/packages/cli/src/search-optimization/validators.ts:222-235` - Metric calculation
- `/workspace/packages/cli/src/search-optimization/genetic-iterator.ts:243-265` - Multi-run orchestration

**Planning References**:
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/plan.md` - Phase 1.3 requirements
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/analysis.md` - Statistical methodology research
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:280-310` - Comparison framework architecture

## Notes

The comparison framework is critical for scientific validity. Without it, we can't make objective claims about semantic search value. The framework must:
1. Be reproducible (same task → same conclusion)
2. Be statistically sound (valid tests, documented assumptions)
3. Be interpretable (clear reports, plain-English explanations)

This enables the transition from "search seems better" (subjective) to "search provides 40% improvement, p<0.05" (objective, falsifiable).
