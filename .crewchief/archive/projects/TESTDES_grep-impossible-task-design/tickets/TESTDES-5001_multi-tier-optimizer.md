# Ticket: TESTDES-5001: Implement Multi-Tier Optimizer

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Extend the existing genetic optimizer to evaluate variants across all 3 benchmark tiers (Tier 1: grep-impossible, Tier 2: grep-hard, Tier 3: real-world) using weighted multi-tier scoring formula (40% Tier 1 + 40% Tier 2 + 20% Tier 3). Track tool selection correctness and optimize for appropriate search usage patterns rather than single-task performance.

## Background
The current genetic optimizer (genetic-iterator.ts) runs competitions on individual tasks and optimizes based on single-task scores. The analysis document identified this as measuring the wrong thing - we optimized for task performance without considering whether agents chose the right tool for the task.

This ticket transforms the optimizer from single-task to multi-tier evaluation, integrating the complete 3-tier framework:
- **Tier 1 (40%)**: Grep-impossible tasks - prove capability advantage
- **Tier 2 (40%)**: Grep-hard tasks - prove efficiency advantage
- **Tier 3 (20%)**: Real-world tasks - prove natural utility

The weighted scoring ensures we optimize for both capability and real-world adoption, not just description quality on synthetic tasks.

**Reference**:
- Architecture.md Section "Integration Points > With Genetic Iterator" (lines 606-624)
- Architecture.md Section "Design Decisions > Three-Tier Framework" (lines 680-683)
- Plan.md Phase 5.1 "Multi-Tier Optimizer" (lines 271-287)
- Quality-strategy.md Tier weightings (40% T1 + 40% T2 + 20% T3)

## Acceptance Criteria
- [x] Genetic optimizer runs all 3 benchmark suites (30+ tasks total) per generation
- [x] Scoring formula implemented: 40% Tier 1 + 40% Tier 2 + 20% Tier 3
- [x] Tool selection tracking: records when search was used appropriately vs inappropriately
- [x] Multi-tier scoring module created with tier-weighted aggregation logic
- [x] Optimization improves all 3 tiers, not just one (validated via generation reports)
- [x] Reports show per-tier breakdown and tool selection patterns
- [x] Integration tests validate multi-tier evaluation with sample tasks

## Technical Requirements
- TypeScript implementation extending existing `genetic-iterator.ts`
- Create new module: `packages/cli/src/search-optimization/multi-tier-scoring.ts`
- Multi-tier scoring must:
  - Accept results from all 3 benchmark suites
  - Apply tier weights (40%, 40%, 20%)
  - Track tool selection correctness per tier
  - Calculate composite score
- Update `genetic-iterator.ts` to:
  - Run all 3 benchmark suites per generation (not just 1 task)
  - Use multi-tier scoring instead of single-task score
  - Report per-tier performance in generation reports
  - Track tool selection patterns across generations
- Follow existing code style (ESM modules, strict typing)
- Use Vitest for unit tests

## Implementation Notes

### Current State (genetic-iterator.ts)
The genetic optimizer currently:
```typescript
// Runs ONE task per generation
for (const task of config.tasks) {
  const result = await runCompetition({ task, variants, ... })
  // Score based on this single task
}
```

### Target State (multi-tier)
The enhanced optimizer should:
```typescript
// Run ALL 3 tiers per generation
const tier1Results = await runSuite(TIER1_SUITE, variants)
const tier2Results = await runSuite(TIER2_SUITE, variants)
const tier3Results = await runSuite(TIER3_SUITE, variants)

// Calculate weighted score
const multiTierScore = calculateMultiTierScore({
  tier1: tier1Results,
  tier2: tier2Results,
  tier3: tier3Results,
  weights: { tier1: 0.4, tier2: 0.4, tier3: 0.2 }
})
```

### Multi-Tier Scoring Logic

```typescript
interface MultiTierScore {
  composite: number  // 0-1, weighted average across tiers

  tier1: {
    avgScore: number  // Average across Tier 1 tasks
    searchUsageRate: number  // % of tasks where search was used
    appropriateUsage: number  // % where search use was appropriate
  }

  tier2: {
    avgScore: number
    searchUsageRate: number
    efficiencyGain: number  // Average time/tool-call reduction vs grep
  }

  tier3: {
    avgScore: number
    voluntaryAdoptionRate: number  // % where search used without coercion
    naturalBehavior: boolean  // Tool selection matches real-world patterns
  }

  toolSelection: {
    correctSearchUse: number  // Used search on grep-impossible/hard tasks
    correctGrepUse: number  // Used grep on grep-possible tasks
    overallAccuracy: number  // % of correct tool choices
  }
}
```

### Tier-Appropriate Metrics

**Tier 1 (Grep-Impossible)**:
- Success rate (did agent complete task?)
- Search usage (search should be used - grep fails)
- Completeness (found all required elements)

**Tier 2 (Grep-Hard)**:
- Success rate
- Efficiency (time saved, tool calls reduced)
- Precision (fewer false positives)
- Search usage (search preferred but grep works)

**Tier 3 (Real-World)**:
- Task completion
- Voluntary search adoption (no hints given)
- Natural behavior (tool choice matches developer expectations)

### Generation Report Enhancement

Current reports show:
```
Generation 1:
  Best: variant-a (85.3%)
  Avg: 78.1%
  Improvement: +5.2%
```

Enhanced reports should show:
```
Generation 1:
  Best: variant-a (Composite: 81.5%)
    Tier 1 (40%): 90.2% (8/10 tasks, search used on all)
    Tier 2 (40%): 78.5% (10/12 tasks, 45% time savings avg)
    Tier 3 (20%): 72.0% (6/8 tasks, 60% voluntary adoption)

  Tool Selection:
    Appropriate search use: 87% (grep-impossible/hard tasks)
    Appropriate grep use: 65% (grep-possible tasks)
    Overall accuracy: 79%

  Improvement: +4.8% (Tier 1: +2.1%, Tier 2: +6.3%, Tier 3: +5.2%)
```

### Convergence Criteria Update

Current convergence: improvement < threshold for 3 consecutive generations

Enhanced convergence should require:
- Composite score improvement < threshold
- All 3 tiers showing stability (no tier degrading)
- Tool selection accuracy improving or stable

### Integration with Existing Suite Runners

```typescript
// Use TESTDES-2004, TESTDES-4001, TESTDES-4002 suite runners
import { TIER1_SUITE } from './benchmarks/tier1-impossible.js'
import { TIER2_SUITE } from './benchmarks/tier2-hard.js'
import { TIER3_SUITE } from './benchmarks/tier3-realworld.js'

// Run each suite and aggregate results
async function runMultiTierEvaluation(variants: Variant[]): Promise<MultiTierResult> {
  const results = await Promise.all([
    runBenchmarkSuite(TIER1_SUITE, variants),
    runBenchmarkSuite(TIER2_SUITE, variants),
    runBenchmarkSuite(TIER3_SUITE, variants),
  ])

  return aggregateMultiTierResults(results)
}
```

### Performance Considerations

Running 30+ tasks per generation is expensive:
- **Parallelization**: Run tasks in parallel where possible
- **Caching**: Cache results for unchanged variants
- **Sampling**: Consider running subset of tasks for intermediate generations
- **Cost estimation**: Log estimated API costs per generation

### Testing Strategy

**Unit Tests**:
- Multi-tier scoring calculation with mock data
- Tier weight application (40%, 40%, 20%)
- Tool selection tracking logic
- Score aggregation across suites

**Integration Tests**:
- Run genetic optimizer with 3 simple tasks (1 per tier)
- Verify all tiers are evaluated
- Verify composite score calculated correctly
- Verify generation reports include all tier metrics

## Dependencies
- **TESTDES-2004**: Tier 1 benchmark suite must be implemented
- **TESTDES-4001**: Tier 2 benchmark suite must be implemented
- **TESTDES-4002**: Tier 3 benchmark suite must be implemented
- **TESTDES-1003**: Comparison framework for metrics calculation
- Existing `genetic-iterator.ts` (extends this)

## Risk Assessment
- **Risk**: Running 30+ tasks per generation is expensive (~$50-100/generation)
  - **Mitigation**: Implement caching, parallel execution, cost estimation logging, consider smaller representative subset for intermediate generations

- **Risk**: Tier weighting might need adjustment based on empirical results
  - **Mitigation**: Make weights configurable via IterationConfig, document rationale for 40/40/20 split, prepare to adjust based on findings

- **Risk**: All 3 tiers might not converge simultaneously
  - **Mitigation**: Track per-tier convergence separately, allow different convergence criteria per tier, accept that some tiers may stabilize before others

- **Risk**: Tool selection tracking might be subjective (what is "appropriate"?)
  - **Mitigation**: Use objective thresholds from taxonomy (Tier 1 tasks should use search, Tier 3 should show voluntary adoption), document decision criteria

- **Risk**: Existing genetic-iterator.ts might need significant refactoring
  - **Mitigation**: Minimize changes to core iteration logic, create separate multi-tier module for scoring, make tier evaluation optional (backward compatible)

## Files/Packages Affected
**Files to Create**:
- `packages/cli/src/search-optimization/multi-tier-scoring.ts`
- `packages/cli/src/search-optimization/__tests__/multi-tier-scoring.test.ts`
- `packages/cli/src/search-optimization/__tests__/genetic-iterator-multi-tier.integration.test.ts`

**Files to Update**:
- `packages/cli/src/search-optimization/genetic-iterator.ts` - Add multi-tier evaluation mode
- `packages/cli/src/search-optimization/types.ts` - Add MultiTierScore and related types
- `packages/cli/src/search-optimization/index.ts` - Export multi-tier scoring module
