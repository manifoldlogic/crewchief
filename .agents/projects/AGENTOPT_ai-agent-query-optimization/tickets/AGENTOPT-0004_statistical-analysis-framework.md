# Ticket: AGENTOPT-0004 - Build Statistical Analysis Framework

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

Implement statistical analysis tools for comparing variant performance, detecting winners with statistical significance, and recommending mutations based on results. This provides rigorous statistical validation to ensure variant improvements are real, not random variation.

## Background

This ticket implements Phase 0, Step 4 from the AGENTOPT project plan (planning/plan.md lines 442-451). The statistical analyzer is a critical component of the testing infrastructure that enables data-driven decisions about which tool description variants are truly better than others.

**Context**: The AGENTOPT project uses a competitive testing approach where multiple variants of the maproom MCP tool description are tested against a standard query set. This analyzer takes the metrics produced by the testing harness (AGENTOPT-0003) and applies rigorous statistical methods to determine winners and recommend next mutations.

**Why this matters**: Without statistical significance testing, we risk deploying variants that appear better due to random chance rather than genuine improvements. This analyzer prevents false positives through proper hypothesis testing (p<0.05 threshold), effect size analysis, and confidence intervals.

## Acceptance Criteria

- [ ] Pairwise t-test implementation for variant comparison (Welch's t-test for unequal variances)
- [ ] 95% confidence interval calculator for all metrics
- [ ] Winner detection logic with statistical significance validation
- [ ] Mutation recommendation engine based on test results
- [ ] Experiment report generator with clear statistical summaries
- [ ] Minimum sample size validation (warn if n<100)
- [ ] Multiple comparison correction for >2 variants

## Technical Requirements

**Statistical Methods**:
- Two-sample Welch's t-test for success rate comparison (assumes unequal variances)
- 95% confidence intervals (bootstrap or analytical methods acceptable)
- Cohen's d effect size calculation (medium effect threshold: >0.3)
- Minimum sample size validation (n≥100 per variant, warn if power <0.8)
- Bonferroni correction for family-wise error rate when testing >2 variants

**Winner Detection Criteria**:
- p-value < 0.05 (statistically significant improvement)
- Success rate improvement >5% (practically significant, in addition to statistical significance)
- No degradation in simple query success (fail-safe criterion)

**Mutation Recommendations**:
- **If winner found**: Suggest mutations to explore similar space (crossover + small mutations)
- **If tie** (no statistical winner): Suggest crossover between top 2 variants
- **If all fail**: Suggest radical mutations (large parameter changes)

**Output Format** (matching architecture.md lines 1005-1026):
```typescript
interface AnalysisResult {
  experiment_id: string
  winner: string | null
  statistical_significance: boolean
  p_value: number
  effect_size: number
  confidence_interval: {
    lower: number
    upper: number
    confidence: number
  }
  recommendation: string
  variants: VariantResult[]
}

interface VariantResult {
  name: string
  success_rate: number
  n_trials: number
  mean_result: number
  std_dev: number
  vs_baseline: {
    delta: number
    p_value: number
    effect_size: number
  }
}
```

## Implementation Notes

Create `packages/maproom-mcp/test/tool-description-optimization/analyzer.ts` with the following structure:

```typescript
interface AnalysisResult {
  experiment_id: string
  winner: string | null
  statistical_significance: boolean
  p_value: number
  effect_size: number
  confidence_interval: {
    lower: number
    upper: number
    confidence: number
  }
  recommendation: string
  variants: VariantResult[]
}

interface VariantResult {
  name: string
  success_rate: number
  n_trials: number
  mean_result: number
  std_dev: number
  vs_baseline: {
    delta: number
    p_value: number
    effect_size: number
  }
}

// Core functions needed:
// - welchTTest(variant1, variant2): { t, df, p_value }
// - cohensD(variant1, variant2): number
// - confidenceInterval(data, confidence): { lower, upper }
// - detectWinner(variants): AnalysisResult
// - recommendMutations(result): string[]
// - generateReport(result): string
```

**Statistical Implementation Options**:
1. Use existing npm package (e.g., `simple-statistics`, `jstat`)
2. Implement Welch's t-test directly (relatively simple math)
3. Bootstrap confidence intervals (Monte Carlo approach)

**Design Considerations**:
- Welch's t-test is preferred over Student's t-test because variants may have different variance
- Bootstrap CIs are more robust than analytical methods but slower; analytical CIs acceptable for this use case
- For >2 variants, apply Bonferroni correction: adjusted α = 0.05 / number_of_comparisons
- Cohen's d interpretation: 0.2 (small), 0.5 (medium), 0.8 (large)
- Power analysis: require n≥100 per variant to achieve 80% power for medium effect size

## Dependencies

- **AGENTOPT-0003** (testing harness) - provides the metrics data that this analyzer consumes
- Node.js built-in math or npm statistical library (to be selected)

## Risk Assessment

- **Risk**: False positives due to multiple comparisons (testing multiple variants simultaneously)
  - **Mitigation**: Apply Bonferroni correction for family-wise error rate control

- **Risk**: Insufficient sample size leading to low statistical power
  - **Mitigation**: Require minimum n≥100 per variant; warn if statistical power <0.8

- **Risk**: Assumptions violated (non-normal distributions, very unequal variances)
  - **Mitigation**: Use Welch's t-test (robust to unequal variances); consider non-parametric alternatives if needed

- **Risk**: Practical vs statistical significance mismatch (p<0.05 but only 1% improvement)
  - **Mitigation**: Require both statistical (p<0.05) AND practical (>5% improvement) significance

## Files/Packages Affected

- `packages/maproom-mcp/test/tool-description-optimization/analyzer.ts` - Main analyzer implementation
- `packages/maproom-mcp/test/tool-description-optimization/statistics.ts` - Statistical utility functions
- `packages/maproom-mcp/test/tool-description-optimization/types.ts` - TypeScript interfaces (if needed for shared types)

## Planning References

- **Phase 0 Overview**: planning/plan.md lines 200-300 (testing infrastructure phase)
- **Step 4 Details**: planning/plan.md lines 442-451 (this step)
- **Architecture Design**: planning/architecture.md lines 1005-1026 (output format specification)
