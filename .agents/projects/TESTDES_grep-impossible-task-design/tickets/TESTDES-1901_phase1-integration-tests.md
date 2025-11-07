# Ticket: TESTDES-1901: Phase 1 Integration Tests

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
Create comprehensive end-to-end integration tests for Phase 1 infrastructure components (task taxonomy, baseline runner, comparison framework). Validate complete workflows from task creation through baseline execution to statistical comparison, ensuring all Phase 1 components work together correctly before proceeding to Phase 2.

## Background
Phase 1 established the foundational infrastructure for the grep-impossible task design framework (TESTDES-1001, 1002, 1003). While unit tests validate individual components, integration tests are critical to prove the complete workflow functions correctly: create task → run baseline → run comparison → generate report.

The quality strategy (lines 156-196) emphasizes integration testing for workflow validation. Integration tests verify components work together correctly and catch issues that unit tests miss (e.g., data format mismatches, incorrect API contracts, workflow state issues).

Integration tests must use realistic test data rather than mocks, and validate all critical integration points between taxonomy, baseline runner, and comparison framework.

**Reference**: See quality-strategy.md Section "Phase 1: Unit Validation" (lines 156-196) and architecture.md Section "Evaluation Framework" (lines 200-284).

## Acceptance Criteria
- [ ] End-to-end workflow test: create task → run baseline → compare → generate report
- [ ] Integration test for taxonomy → baseline runner interaction (task definitions feed into execution)
- [ ] Integration test for baseline runner → comparison framework interaction (metrics flow correctly)
- [ ] Test validates complete grep-only baseline execution with real task
- [ ] Test validates search-available execution and comparison with real task
- [ ] Statistical test integration validated (t-test, confidence intervals work on real data)
- [ ] Test covers all 6 task categories from taxonomy
- [ ] Test validates error handling across component boundaries
- [ ] All integration tests pass and use realistic test data (not mocks)

## Technical Requirements
- TypeScript implementation in `packages/cli/src/search-optimization/evaluation/__tests__/integration/`
- Use Vitest for test framework
- Tests must import and use real implementations from:
  - `taxonomy/` (TESTDES-1001)
  - `evaluation/baseline-runner.ts` (TESTDES-1002)
  - `evaluation/comparison.ts`, `metrics.ts`, `statistics.ts` (TESTDES-1003)
- Create realistic test tasks spanning all 6 categories
- Use realistic test data (not comprehensive mocks)
- Test file naming: `*.integration.test.ts` to distinguish from unit tests
- Tests should be independent and can run in any order
- Follow existing test patterns in the codebase

## Implementation Notes

### Test Structure
```typescript
// integration/workflow.integration.test.ts
describe('Phase 1 Integration: Complete Workflow', () => {
  it('executes complete task evaluation workflow', async () => {
    // 1. Create task using taxonomy
    const task = createTaskFromCategory('relationship-discovery')

    // 2. Run grep baseline
    const grepResult = await runBaselineExecution(task, { enableSearch: false })

    // 3. Run search-available execution
    const searchResult = await runBaselineExecution(task, { enableSearch: true })

    // 4. Compare results
    const comparison = await compareResults(grepResult, searchResult)

    // 5. Generate report
    const report = generateComparisonReport(comparison)

    // Validate complete workflow
    expect(grepResult.metrics).toBeDefined()
    expect(searchResult.metrics).toBeDefined()
    expect(comparison.advantage.significantDifference).toBeDefined()
    expect(report).toContain('Statistical Analysis')
  })
})
```

### Integration Points to Test

1. **Taxonomy → Baseline Runner**
   - Task definitions correctly feed into execution engine
   - Success validators work with baseline runner
   - Category metadata is preserved through execution

2. **Baseline Runner → Comparison Framework**
   - Metrics format is compatible
   - Tool usage tracking flows correctly
   - Execution transcripts are captured

3. **Comparison Framework → Statistical Tests**
   - Metrics feed into t-test correctly
   - Confidence intervals calculate properly
   - Significance determination works

### Test Data Strategy

Use realistic test tasks:
```typescript
const testTasks = {
  relationshipDiscovery: {
    description: "Find code that depends on worktree creation",
    category: "relationship-discovery",
    expectedGrepSuccess: 0.2,
    expectedSearchSuccess: 0.8
  },
  conceptualSimilarity: {
    description: "Find all retry implementations",
    category: "conceptual-similarity",
    expectedGrepSuccess: 0.4,
    expectedSearchSuccess: 0.9
  }
  // ... one for each category
}
```

### Edge Cases to Test

1. **Empty results** - Task returns no matches
2. **Error propagation** - Failures in one component affect others correctly
3. **Statistical edge cases** - Same results from both tools (no difference)
4. **Timeout handling** - Long-running tasks handled gracefully
5. **Invalid task definitions** - Malformed tasks rejected early

### Performance Considerations

Integration tests will be slower than unit tests (actual execution). Consider:
- Mark as integration tests for separate CI runs
- Use smaller codebases for test execution
- Set reasonable timeouts (30s per test)
- Don't run on every file save (manual trigger or pre-commit)

## Dependencies
- TESTDES-1001 (Task Taxonomy Infrastructure) - MUST be complete
- TESTDES-1002 (Baseline Runner) - MUST be complete
- TESTDES-1003 (Comparison Framework) - MUST be complete

All Phase 1 implementation tickets must pass their unit tests before integration tests can be implemented.

## Risk Assessment
- **Risk**: Integration tests may be flaky due to external dependencies (e.g., actual code execution)
  - **Mitigation**: Use controlled test environments, mock only external I/O (file system can use temp directories), retry flaky assertions

- **Risk**: Tests may be slow, blocking development workflow
  - **Mitigation**: Mark as integration tests, run separately from unit tests, consider using smaller test codebases

- **Risk**: Component interfaces may change during Phase 1 development
  - **Mitigation**: Update integration tests alongside implementation changes, keep tests close to the code

- **Risk**: Realistic test data may be hard to maintain
  - **Mitigation**: Use test fixtures that are easy to understand and update, document test data expectations clearly

## Files/Packages Affected
**Files to Create**:
- `packages/cli/src/search-optimization/evaluation/__tests__/integration/workflow.integration.test.ts`
- `packages/cli/src/search-optimization/evaluation/__tests__/integration/taxonomy-baseline.integration.test.ts`
- `packages/cli/src/search-optimization/evaluation/__tests__/integration/baseline-comparison.integration.test.ts`
- `packages/cli/src/search-optimization/evaluation/__tests__/integration/statistical-validation.integration.test.ts`
- `packages/cli/src/search-optimization/evaluation/__tests__/integration/test-fixtures.ts` (shared test data)

**Files to Import From**:
- `packages/cli/src/search-optimization/taxonomy/` (all exports)
- `packages/cli/src/search-optimization/evaluation/baseline-runner.ts`
- `packages/cli/src/search-optimization/evaluation/comparison.ts`
- `packages/cli/src/search-optimization/evaluation/metrics.ts`
- `packages/cli/src/search-optimization/evaluation/statistics.ts`
