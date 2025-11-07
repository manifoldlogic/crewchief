# TESTDES-2004: Implement Tier 1 Benchmark Suite

**Status**: 📋 Planned
**Priority**: High
**Complexity**: Medium (4-6 hours)
**Phase**: 2 - Grep-Impossible Tasks
**Dependencies**: TESTDES-2001, TESTDES-2002, TESTDES-2003

## Summary

Implement the Tier 1 Benchmark Suite that aggregates all grep-impossible tasks (8-10 total) into a unified suite with validation pipeline and comprehensive reporting. This suite orchestrates execution of relationship discovery, architectural understanding, and negative space tasks, validates they meet quality criteria (<30% grep success rate), and generates reports comparing grep vs search performance.

## Background

The Tier 1 suite is the core deliverable of Phase 2. It aggregates the individual grep-impossible tasks into a cohesive benchmark that:
1. Validates semantic search capability on tasks where grep fundamentally fails
2. Provides systematic validation that all tasks meet the <30% grep success criterion
3. Generates unified reports showing grep vs search comparison across all tasks
4. Integrates with the comparison framework (TESTDES-1003) for statistical analysis

This suite provides the scientific foundation for claiming "semantic search solves problems grep cannot solve" with objective, measurable evidence.

Research foundation: Benchmark suite design from ML evaluation (GLUE, SuperGLUE), task aggregation patterns from IR research (TREC), validation pipeline concepts from software testing.

## Acceptance Criteria

- [ ] Suite aggregates 8-10 tasks from TESTDES-2001, 2002, 2003
- [ ] Suite runner executes all tasks sequentially or in parallel
- [ ] Validation pipeline confirms 80%+ tasks defeat grep (<30% success rate)
- [ ] Integration with comparison framework for statistical analysis
- [ ] Comprehensive report generation showing task-by-task and aggregate metrics
- [ ] Suite metadata includes category coverage, difficulty distribution

## Technical Requirements

**Architecture**:
- TypeScript implementation in `packages/cli/src/search-optimization/benchmarks/`
- Aggregates tasks from relationship-discovery, architectural-understanding, negative-space
- Integrates with comparison framework (TESTDES-1003) for grep vs search evaluation
- Parallel execution support (optional optimization)

**Interfaces**:
```typescript
interface BenchmarkSuite {
  name: string
  version: string
  tier: 1 | 2 | 3
  tasks: SearchTask[]

  // Organization
  byCategory: Map<TaskCategory, SearchTask[]>
  byDifficulty: Map<'easy' | 'medium' | 'hard', SearchTask[]>

  // Metadata
  metadata: {
    totalTasks: number
    categories: string[]
    expectedGrepSuccessRate: number  // <0.3 for Tier 1
    expectedSearchSuccessRate: number  // >0.7 for Tier 1
  }
}

interface SuiteResult {
  suite: BenchmarkSuite
  executionTime: number

  // Per-task results
  taskResults: Array<{
    task: SearchTask
    grepBaseline: BaselineResult
    searchCondition: CompetitionResult
    advantage: ComparisonResult['advantage']
  }>

  // Aggregate metrics
  aggregate: {
    grepAvgSuccess: number
    searchAvgSuccess: number
    avgImprovement: number
    tasksDefeatingGrep: number  // Count with grep <30%
    significantDifferences: number  // Count with p<0.05
  }

  // Validation status
  validation: {
    meetsGrepFailureCriterion: boolean  // 80%+ tasks defeat grep
    meetsSearchSuccessCriterion: boolean  // Avg search >70%
    allTasksValidated: boolean
  }
}
```

**Suite Composition**:
- 3 tasks from relationship-discovery (TESTDES-2001)
- 3 tasks from architectural-understanding (TESTDES-2002)
- 2 tasks from negative-space (TESTDES-2003)
- Total: 8 tasks minimum (can include alternates)

**Validation Pipeline**:
```typescript
async function validateSuite(suite: BenchmarkSuite): Promise<ValidationResult> {
  // Step 1: Verify task quality
  const taskValidations = await validateAllTasks(suite.tasks)

  // Step 2: Run grep baseline on all tasks
  const grepBaseline = await runGrepBaseline(suite)

  // Step 3: Validate grep failure criterion
  const tasksDefeatingGrep = grepBaseline.filter(r => r.success < 0.3).length
  const meetsGrepCriterion = tasksDefeatingGrep / suite.tasks.length >= 0.8

  // Step 4: Check category coverage
  const categories = new Set(suite.tasks.map(t => t.category))
  const meetsCategories = categories.size >= 3

  return {
    passed: meetsGrepCriterion && meetsCategories,
    grepFailureRate: tasksDefeatingGrep / suite.tasks.length,
    categoryCoverage: categories.size
  }
}
```

**Report Generation**:
- Markdown report with:
  - Executive summary (overall grep vs search performance)
  - Task-by-task comparison table
  - Category breakdown
  - Statistical significance analysis
  - Validation status
- Export CSV for further analysis (optional)

## Implementation Notes

### Suite Runner Pattern
```typescript
async function runBenchmarkSuite(
  suite: BenchmarkSuite,
  config: { parallel?: boolean; iterations?: number }
): Promise<SuiteResult> {
  const results = config.parallel
    ? await Promise.all(suite.tasks.map(task => runTaskComparison(task)))
    : await runSequentially(suite.tasks)

  return {
    taskResults: results,
    aggregate: calculateAggregateMetrics(results),
    validation: validateSuiteResults(results, suite)
  }
}
```

### Task Aggregation Strategy
- Import tasks from each category module
- Validate each task has required metadata
- Organize by category and difficulty
- Ensure balanced representation across categories

### Integration with Comparison Framework
- Use comparison framework (TESTDES-1003) for each task
- Aggregate statistical tests across all tasks
- Meta-analysis: overall effect size, combined p-values
- Report both individual and aggregate significance

### Validation Criteria
1. **Grep Failure Criterion**: 80%+ tasks must have grep success <30%
2. **Search Success Criterion**: Average search success >70%
3. **Category Coverage**: All 3 categories represented
4. **Statistical Significance**: 60%+ tasks show p<0.05 difference

### Report Format Example
```markdown
# Tier 1 Benchmark Suite Results

## Executive Summary
- **Tasks**: 8 grep-impossible tasks
- **Grep Success**: 18% average
- **Search Success**: 82% average
- **Improvement**: 64 percentage points
- **Statistical Significance**: 7/8 tasks (p<0.05)

## Task-by-Task Results

| Task | Category | Grep | Search | Improvement | p-value |
|------|----------|------|--------|-------------|---------|
| Transitive Deps | Relationship | 15% | 85% | +70pp | <0.001 |
| Call Chain | Relationship | 20% | 80% | +60pp | 0.002 |
| ... | ... | ... | ... | ... | ... |

## Category Breakdown
- **Relationship Discovery**: 3 tasks, avg improvement +65pp
- **Architectural Understanding**: 3 tasks, avg improvement +63pp
- **Negative Space**: 2 tasks, avg improvement +68pp

## Validation Status
✓ Grep failure criterion met (87.5% tasks defeat grep)
✓ Search success criterion met (82% avg success)
✓ Category coverage complete (3/3 categories)
✓ Statistical significance strong (87.5% significant)
```

## Files to Create/Modify

**New Files**:
- `packages/cli/src/search-optimization/benchmarks/tier1-impossible.ts` - Suite definition and task aggregation
- `packages/cli/src/search-optimization/benchmarks/suite-runner.ts` - Execution orchestration
- `packages/cli/src/search-optimization/benchmarks/validation.ts` - Suite validation logic
- `packages/cli/src/search-optimization/benchmarks/reporter.ts` - Report generation
- `packages/cli/src/search-optimization/benchmarks/__tests__/tier1-suite.test.ts` - Integration tests
- `packages/cli/src/search-optimization/benchmarks/__tests__/suite-validation.test.ts` - Validation tests

**Updated Files**:
- `packages/cli/src/search-optimization/benchmarks/index.ts` - Export suite and runner
- `packages/cli/src/search-optimization/index.ts` - Export benchmark suite publicly

## Dependencies

**Required Tickets**:
- TESTDES-2001: Relationship discovery tasks (provides 3 tasks)
- TESTDES-2002: Architectural understanding tasks (provides 3 tasks)
- TESTDES-2003: Negative space tasks (provides 2 tasks)

**Foundation Infrastructure**:
- TESTDES-1001: Task taxonomy (provides types, categories)
- TESTDES-1002: Baseline runner (provides grep-only execution)
- TESTDES-1003: Comparison framework (provides statistical analysis)

**Existing Code**:
- `packages/cli/src/search-optimization/evaluation/comparison.ts` - For task comparison
- `packages/cli/src/search-optimization/taxonomy/categories.ts` - For category organization

## Agent Assignments

**Primary Agent**: general-purpose
**Responsibilities**: TypeScript implementation, suite orchestration, validation pipeline, report generation

**Supporting Agents**:
- unit-test-runner: Execute and validate tests
- verify-ticket: Check acceptance criteria
- commit-ticket: Create conventional commit

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Not all tasks meet <30% grep criterion | Suite validation fails | Adjust or replace tasks that don't meet threshold |
| Parallel execution causes resource issues | Timeouts, failures | Make parallel execution optional, default to sequential |
| Reports too verbose or unclear | Low utility | Include both detailed and summary views |
| Statistical aggregation incorrect | Invalid conclusions | Cross-validate with manual calculations, document methodology |
| API costs high with many tasks | Budget overrun | Provide cost estimation, allow subset execution |

## Testing Strategy

**Unit Tests**:
- Suite composition (correct tasks, categories, metadata)
- Validation logic (threshold checks, category coverage)
- Report generation (format, completeness)
- Metric aggregation (averages, percentages)

**Integration Tests**:
- Full suite execution with mock tasks
- Validation pipeline with known pass/fail cases
- Report generation with sample results
- Category breakdown correctness

**Validation**:
- Run full suite with real tasks (expensive, manual)
- Verify 80%+ tasks defeat grep
- Confirm statistical significance
- Review generated reports for clarity

## Success Metrics

- [ ] Suite runs successfully with 8-10 tasks
- [ ] Validation confirms 80%+ tasks defeat grep (<30% success)
- [ ] Average search success >70%
- [ ] Reports are clear and actionable
- [ ] Integration tests pass
- [ ] Can execute in reasonable time (<30 min for sequential, <10 min for parallel)

## References

**Code References**:
- `/workspace/packages/cli/src/search-optimization/evaluation/comparison.ts` - Comparison framework pattern
- `/workspace/packages/cli/src/search-optimization/genetic-iterator.ts:243-265` - Multi-run orchestration example
- `/workspace/packages/cli/src/search-optimization/taxonomy/categories.ts` - Task category structure

**Planning References**:
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/plan.md:127-144` - Phase 2.4 requirements
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:561-583` - Benchmark suite architecture
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md:197-239` - Suite validation strategy

**Related Tickets**:
- TESTDES-2001: Relationship discovery tasks
- TESTDES-2002: Architectural understanding tasks
- TESTDES-2003: Negative space tasks
- TESTDES-1003: Comparison framework

## Notes

The Tier 1 Benchmark Suite is the culmination of Phase 2. It aggregates all grep-impossible tasks into a single, validated benchmark that provides scientific evidence for semantic search value.

Key design principles:
1. **Objectivity**: All metrics are measurable, no subjective judgment
2. **Reproducibility**: Same suite → same results (within LLM variance)
3. **Comprehensiveness**: Covers multiple task categories, not cherry-picked
4. **Statistical Rigor**: Uses comparison framework for significance testing
5. **Transparency**: Reports show individual task results, not just aggregates

This suite enables claims like:
- "Semantic search solves 8 grep-impossible tasks with 82% success vs 18% for grep"
- "7 out of 8 tasks show statistically significant improvement (p<0.05)"
- "Tasks span relationship discovery, architectural understanding, and negative space categories"

These are falsifiable, objective statements backed by data—a significant improvement over subjective "search seems better" claims.
