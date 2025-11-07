# Ticket: TESTDES-2901: Tier 1 Suite Validation Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration tests that validate the Tier 1 benchmark suite meets all quality requirements defined in the quality strategy. Tests must verify grep-impossibility, search-suitability, objective success criteria, statistical significance, and result consistency for all 8-10 tasks in the Tier 1 suite.

## Background
The Tier 1 benchmark suite is the foundation of our grep-impossible task framework. It must demonstrate that:
1. Tasks genuinely defeat grep (construct validity)
2. Semantic search provides significant advantage (discriminant validity)
3. Results are reproducible and reliable (test-retest reliability)
4. Success criteria are objective and deterministic

This testing ticket ensures we validate the entire suite holistically, not just individual tasks. It proves the benchmark suite as a whole meets our quality standards before we proceed with Tier 2 and Tier 3 tasks.

**Reference**: This ticket implements the testing strategy defined in `planning/quality-strategy.md` Phase 2: Suite Validation, and validates the architecture specified in `planning/architecture.md` Section "Validation Pipeline".

**Dependencies**:
- TESTDES-2004: Tier 1 Benchmark Suite must be implemented
- TESTDES-3001: Task Validator infrastructure must exist

## Acceptance Criteria
- [ ] Integration test validates all Tier 1 tasks pass grep-impossibility check (<30% grep success rate)
- [ ] Integration test validates all Tier 1 tasks pass search-suitability check (>70% search success rate)
- [ ] Integration test validates all tasks have objective success criteria (deterministic, binary validation)
- [ ] Integration test validates statistical significance of grep vs search difference (p<0.05)
- [ ] Integration test validates result consistency across runs (<10% variance per task)
- [ ] Integration test validates complete validation workflow: Load suite → Validate → Generate report
- [ ] Integration test validates failure detection with intentionally bad tasks
- [ ] Integration test validates suite-level metrics (80%+ tasks defeat grep, category coverage, statistical power)
- [ ] All tests use the TaskValidator from TESTDES-3001
- [ ] Test suite passes with clear, actionable error messages

## Technical Requirements

### Test File Structure
```typescript
// benchmarks/__tests__/tier1-validation.test.ts

import { describe, it, expect, beforeAll } from 'vitest'
import { TIER1_SUITE } from '../tier1-impossible'
import { TaskValidator } from '../../validation/task-validator'
import { runSuite, runTask } from '../../evaluation/baseline-runner'
import { calculateStatistics } from '../../evaluation/statistics'

describe('Tier 1 Suite Validation', () => {
  // Test suite-level quality requirements
})
```

### Required Test Cases

1. **Grep-Impossibility Validation**
   - Run all Tier 1 tasks with grep/glob/read only
   - Assert 80%+ of tasks have <30% success rate
   - Assert no individual task has >50% grep success (too easy)

2. **Search-Suitability Validation**
   - Run all Tier 1 tasks with search available
   - Assert all tasks have >70% success rate with search
   - Assert significant improvement over grep baseline

3. **Objective Criteria Validation**
   - Check all tasks have deterministic success validators
   - Verify no subjective criteria ("good explanation", "clear code")
   - Assert all validators return binary boolean results

4. **Statistical Significance Validation**
   - Run t-test comparing grep vs search scores
   - Assert p-value < 0.05 for suite-level difference
   - Validate statistical power (sufficient iterations)

5. **Consistency Validation**
   - Run each task 5 times with same configuration
   - Calculate variance for each task
   - Assert all tasks have <10% variance across runs

6. **Complete Workflow Integration**
   - Load Tier 1 suite
   - Run TaskValidator on each task
   - Generate validation report
   - Assert all tasks pass validation

7. **Failure Detection**
   - Inject intentionally bad tasks:
     - Too easy (grep succeeds >60%)
     - Too hard (search also fails <50%)
     - Unreliable (high variance >20%)
   - Verify TaskValidator correctly identifies and categorizes failures
   - Assert appropriate error messages

8. **Suite-Level Metrics**
   - Validate category coverage (all 3 Phase 2 categories represented):
     - Relationship Discovery
     - Architectural Understanding
     - Negative Space
   - Validate difficulty distribution (mix of easy/medium/hard within grep-impossible)
   - Validate sufficient sample size for statistical power

### Integration Points
- Use `TaskValidator` from `validation/task-validator.ts` (TESTDES-3001)
- Use `runTask` and `runSuite` from `evaluation/baseline-runner.ts` (TESTDES-1002)
- Use `calculateStatistics` from `evaluation/statistics.ts` (TESTDES-1003)
- Reference `TIER1_SUITE` from `benchmarks/tier1-impossible.ts` (TESTDES-2004)

### Test Data
- Use actual Tier 1 suite tasks (8-10 tasks from TESTDES-2004)
- Create mock "bad tasks" for failure detection testing
- Use deterministic test data where possible to ensure reproducibility

## Implementation Notes

### Test Organization
```typescript
describe('Tier 1 Suite Validation', () => {
  describe('Quality Dimension: Construct Validity', () => {
    it('all tasks defeat grep baseline (<30% success)', async () => {
      // Test grep-impossibility
    })

    it('all tasks succeed with search (>70% success)', async () => {
      // Test search-suitability
    })
  })

  describe('Quality Dimension: Discriminant Validity', () => {
    it('search significantly outperforms grep (p<0.05)', async () => {
      // Statistical test
    })
  })

  describe('Quality Dimension: Test-Retest Reliability', () => {
    it('all tasks show <10% variance across 5 runs', async () => {
      // Consistency test
    })
  })

  describe('Suite-Level Validation', () => {
    it('covers all Phase 2 task categories', () => {
      // Category coverage
    })

    it('has appropriate difficulty distribution', () => {
      // Difficulty balance
    })

    it('80%+ of suite defeats grep', async () => {
      // Suite-level success rate
    })
  })

  describe('Validation Workflow Integration', () => {
    it('complete validation pipeline works end-to-end', async () => {
      // Load → Validate → Report
    })
  })

  describe('Failure Detection', () => {
    it('identifies tasks that are too easy', async () => {
      // Bad task detection
    })

    it('identifies tasks that are too hard', async () => {
      // Bad task detection
    })

    it('identifies unreliable tasks', async () => {
      // Variance detection
    })
  })
})
```

### Performance Considerations
- These tests will be expensive (running full task suite multiple times)
- Consider using smaller task subset for CI, full suite for manual validation
- Mock LLM responses for unit testing where possible
- Use real API calls for integration testing (mark as slow tests)

### Test Execution Strategy
```typescript
// Option 1: Fast unit tests (mocked)
describe('Tier 1 Suite Validation (Unit)', () => {
  // Use mocked task results
  // Fast, runs in CI
})

// Option 2: Integration tests (real API calls)
describe.skip('Tier 1 Suite Validation (Integration)', () => {
  // Real task execution
  // Expensive, manual execution only
  // Enable with: pnpm test:integration
})
```

### Validation Report Assertions
The validation workflow test should assert the report contains:
- Per-task validation status (pass/fail)
- Grep baseline metrics for each task
- Search performance metrics for each task
- Statistical significance results
- Failure categorization (if any tasks fail)
- Recommendations for improvement

## Dependencies
- **TESTDES-2004**: Tier 1 Benchmark Suite (provides tasks to validate)
- **TESTDES-3001**: Task Validator (provides validation infrastructure)
- **TESTDES-1002**: Baseline Runner (provides task execution)
- **TESTDES-1003**: Comparison Framework (provides statistical tests)

## Risk Assessment
- **Risk**: Integration tests are expensive to run (API costs, time)
  - **Mitigation**: Create two test suites: fast mocked unit tests for CI, slow integration tests for manual validation

- **Risk**: Flaky tests due to LLM variance
  - **Mitigation**: Use generous variance thresholds (<10%), run multiple iterations, focus on statistical trends not perfect consistency

- **Risk**: Tests may fail if Tier 1 suite quality is poor
  - **Mitigation**: This is intentional—tests should catch quality issues. Work with TESTDES-2004 implementer to improve task quality if needed

- **Risk**: Test suite becomes outdated as tasks evolve
  - **Mitigation**: Tests validate against quality dimensions (not specific tasks), so they remain valid as tasks change

## Files/Packages Affected
- **New File**: `packages/cli/src/search-optimization/benchmarks/__tests__/tier1-validation.test.ts`
- **Dependencies**:
  - `packages/cli/src/search-optimization/benchmarks/tier1-impossible.ts` (TESTDES-2004)
  - `packages/cli/src/search-optimization/validation/task-validator.ts` (TESTDES-3001)
  - `packages/cli/src/search-optimization/evaluation/baseline-runner.ts` (TESTDES-1002)
  - `packages/cli/src/search-optimization/evaluation/statistics.ts` (TESTDES-1003)
- **Test Framework**: Vitest
- **Package**: `@crewchief/cli`
