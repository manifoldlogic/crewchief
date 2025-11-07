# Ticket: TESTDES-5002: Create Full Validation Run Script

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create an executable script that runs the complete validation suite across all 30+ tasks (Tier 1, 2, and 3), executes both grep-only baseline and search-available conditions, performs comprehensive statistical analysis, and generates a detailed validation report showing the proof of semantic search value. This is the "proof moment" - the comprehensive validation that demonstrates measurable search advantages.

## Background
After building the three-tier benchmark framework (Phases 1-4) and integrating with the genetic optimizer (TESTDES-5001), we need a comprehensive validation run that proves semantic search provides measurable value across all task categories. This script executes the complete benchmark suite to validate that:

1. Tier 1 tasks defeat grep (grep-impossible)
2. Tier 2 tasks show significant time/efficiency savings
3. Tier 3 tasks result in voluntary search adoption
4. Statistical significance (p < 0.05) for grep vs search comparison
5. Genetic optimizer improves search usage on appropriate tasks

**Critical Note**: This script is expensive to run ($20-50 in API costs) and should be executed manually with explicit confirmation, NOT in CI/CD pipelines.

**Reference**: See plan.md Section "Phase 5.2 Validation Run" (lines 288-304) and quality-strategy.md "Success Metrics" (lines 428-446).

## Acceptance Criteria
- [ ] Script executes all 30+ tasks across Tier 1 (grep-impossible), Tier 2 (grep-hard), and Tier 3 (real-world)
- [ ] Runs both conditions: grep-only baseline and search-available
- [ ] Performs statistical analysis (t-tests, confidence intervals, effect sizes)
- [ ] Generates comprehensive validation report with:
  - [ ] Per-tier results summary (Tier 1, 2, 3)
  - [ ] Per-category results (6 categories: relationship-discovery, conceptual-similarity, etc.)
  - [ ] Overall grep vs search comparison with statistical significance
  - [ ] Tool selection patterns and correctness metrics
  - [ ] Detailed failure analysis by category
- [ ] Displays cost estimation before execution
- [ ] Requires explicit user confirmation before running (expensive operation)
- [ ] Saves results to timestamped directory for archival (`.crewchief/validation-results/YYYY-MM-DD-HHMMSS/`)
- [ ] Includes summary statistics: mean, median, std dev, confidence intervals
- [ ] Reports p-values for statistical significance tests

## Technical Requirements
- TypeScript implementation in `packages/cli/src/search-optimization/scripts/run-full-validation.ts`
- Uses multi-tier optimizer from TESTDES-5001 for task execution
- Uses comparison framework from TESTDES-1003 for grep vs search evaluation
- Integrates all benchmark suites:
  - Tier 1 from TESTDES-2004 (grep-impossible)
  - Tier 2 from TESTDES-4001 (grep-hard)
  - Tier 3 from TESTDES-4002 (real-world)
- Statistical analysis using t-tests, confidence intervals, effect sizes
- Cost estimation based on task count and expected API calls per task
- CLI interface with confirmation prompt
- Results saved as JSON + Markdown report
- Progress indicators during execution (can take 30-60 minutes)
- Error handling and partial result recovery

## Implementation Notes

### Script Structure
```typescript
// Main script workflow
async function runFullValidation(options: ValidationOptions): Promise<ValidationResults> {
  // 1. Load all benchmark suites
  const tier1 = loadBenchmarkSuite('tier1-impossible')
  const tier2 = loadBenchmarkSuite('tier2-hard')
  const tier3 = loadBenchmarkSuite('tier3-realworld')

  // 2. Estimate cost and get confirmation
  const estimatedCost = estimateCost([tier1, tier2, tier3])
  const confirmed = await promptConfirmation(estimatedCost)
  if (!confirmed) return

  // 3. Run grep baseline (control)
  const grepResults = await runSuiteBaseline([tier1, tier2, tier3], {
    tools: ['grep', 'glob', 'read', 'bash']
  })

  // 4. Run with search available (treatment)
  const searchResults = await runSuiteWithSearch([tier1, tier2, tier3], {
    tools: ['grep', 'glob', 'read', 'bash', 'mcp__maproom__search']
  })

  // 5. Statistical analysis
  const stats = performStatisticalAnalysis(grepResults, searchResults)

  // 6. Generate report
  const report = generateValidationReport({
    grepResults,
    searchResults,
    stats,
    timestamp: new Date()
  })

  // 7. Save results
  await saveResults(report, options.outputDir)

  return report
}
```

### Cost Estimation
- Estimate based on:
  - Total task count (30+)
  - Average API calls per task (~10-15 calls)
  - API cost per call (~$0.01-0.02)
  - Run twice (grep baseline + search available)
- Display: "Estimated cost: $20-50 USD. Continue? (y/N)"

### Statistical Analysis
Required tests:
1. **Paired t-test**: Compare grep vs search scores per task
2. **Confidence intervals**: 95% CI for mean difference
3. **Effect size**: Cohen's d for practical significance
4. **Success rate**: % tasks where search > grep
5. **Tool selection accuracy**: % tasks where agent chose appropriate tool

### Report Format
```markdown
# Full Validation Report - {timestamp}

## Executive Summary
- Total Tasks: 35
- Grep Baseline: 42% success rate
- Search Available: 78% success rate
- Improvement: +36% (p < 0.001)
- Statistical Significance: ✅ Strong evidence

## Tier 1: Grep-Impossible Tasks (n=10)
- Grep Success: 18%
- Search Success: 85%
- Improvement: +67% (p < 0.001)
- Tasks Defeating Grep: 90% (9/10)

## Tier 2: Grep-Hard Tasks (n=12)
- Grep Success: 45%
- Search Success: 78%
- Time Savings: 42% faster
- Efficiency Improvement: Significant

## Tier 3: Real-World Tasks (n=10)
- Voluntary Search Adoption: 65%
- Natural Tool Selection: 82% correct
- User Satisfaction Proxy: High

## Per-Category Analysis
### Relationship Discovery
- Tasks: 5
- Grep: 12% | Search: 88%
- Advantage: Critical ✅

### Conceptual Similarity
- Tasks: 6
- Grep: 38% | Search: 71%
- Advantage: Significant ✅

[... continue for all 6 categories]

## Statistical Tests
- t-test: t(34) = 8.42, p < 0.001
- Cohen's d: 1.45 (large effect)
- 95% CI: [28%, 44%] improvement
- Power: 0.98

## Tool Selection Analysis
- Correct Search Usage: 78%
- Correct Grep Usage: 85%
- Inappropriate Tool Choice: 12%

## Failure Analysis
[Detailed breakdown of failures by category]

## Conclusion
Strong evidence that semantic search provides measurable value:
✅ Defeats grep on impossible tasks (Tier 1)
✅ Shows efficiency gains on hard tasks (Tier 2)
✅ Natural adoption in real-world scenarios (Tier 3)
✅ Statistically significant across all tiers
```

### Output Structure
```
.crewchief/validation-results/2025-01-15-143022/
├── results.json           # Full data
├── report.md              # Human-readable report
├── grep-baseline/         # Individual task results
│   ├── tier1/
│   ├── tier2/
│   └── tier3/
├── search-available/      # Individual task results
│   ├── tier1/
│   ├── tier2/
│   └── tier3/
└── stats/
    ├── summary.json
    ├── per-tier.json
    └── per-category.json
```

### CLI Usage
```bash
# Run full validation
pnpm search-optimization:validate-full

# With options
pnpm search-optimization:validate-full \
  --output-dir=.crewchief/validation-results \
  --skip-confirmation  # For CI (use with caution)
```

### Integration Points
- **Multi-tier optimizer** (TESTDES-5001): Use for task execution and scoring
- **Comparison framework** (TESTDES-1003): Side-by-side evaluation logic
- **Benchmark suites**: Import from TESTDES-2004, 4001, 4002
- **Validation infrastructure** (TESTDES-3001): Task quality checks

### Error Handling
- Partial failure recovery: Save intermediate results
- Timeout handling: Skip stuck tasks after 5 minutes
- API errors: Retry with exponential backoff
- Results preservation: Save even if script crashes

## Dependencies
- TESTDES-5001 (Multi-Tier Optimizer) - must be complete
- TESTDES-1003 (Comparison Framework) - needed for evaluation
- TESTDES-2004 (Tier 1 Suite) - task source
- TESTDES-4001 (Tier 2 Suite) - task source
- TESTDES-4002 (Tier 3 Suite) - task source

## Risk Assessment
- **Risk**: Expensive API costs ($20-50 per run)
  - **Mitigation**: Require explicit confirmation, show cost estimate, manual execution only

- **Risk**: Long execution time (30-60 minutes)
  - **Mitigation**: Progress indicators, intermediate result saving, ability to resume

- **Risk**: API rate limits or failures
  - **Mitigation**: Retry logic, exponential backoff, save partial results

- **Risk**: Results may not show statistical significance
  - **Mitigation**: This is a valid outcome - documents that current benchmark needs refinement

- **Risk**: Script crashes mid-run, losing expensive results
  - **Mitigation**: Save intermediate results after each tier, resume capability

## Files/Packages Affected
**Files to Create**:
- `packages/cli/src/search-optimization/scripts/run-full-validation.ts`
- `packages/cli/src/search-optimization/reporting/validation-report.ts`
- `packages/cli/src/search-optimization/reporting/statistics.ts`
- `packages/cli/src/search-optimization/scripts/__tests__/run-full-validation.test.ts` (dry-run tests)

**Directories to Create**:
- `.crewchief/validation-results/` (for output storage)

**Files to Update**:
- `packages/cli/package.json` (add script: `search-optimization:validate-full`)
- `packages/cli/src/search-optimization/index.ts` (export validation runner)
