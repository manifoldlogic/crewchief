# Ticket: AGENTOPT-0006: Phase 0 Integration and First Experiment

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
Integrate all Phase 0 components (test query set, variants, testing harness, statistical analyzer, and results aggregator) and execute the first competitive experiment with 5 initial variants to establish baseline data-driven optimization process.

## Background
This ticket completes Phase 0 by integrating components from AGENTOPT-0001 through AGENTOPT-0005 and executing the first empirical experiment. This work validates the entire data-driven testing framework, establishes the experimental workflow, and produces the first empirically-validated variant for Phase 1 deployment. The experiment runs 5 variants × 100 queries = 500 total searches using rule-based simulation for speed, collects full metrics per variant, and performs statistical comparison to identify the clear winner or confirm no significant improvement. This integration is critical because it transforms isolated components into a functional optimization pipeline and generates real empirical data to inform Phase 1 deployment decisions.

## Acceptance Criteria
- [ ] All Phase 0 components integrated and working together
- [ ] First experiment executed with 5 variants (control + 4 competitors)
- [ ] Statistical analysis identifies clear winner or no significant difference
- [ ] Experiment report generated with full methodology documentation
- [ ] Winner variant (or control if no improvement) prepared for Phase 1 deployment
- [ ] Process documentation created for running future experiments

## Technical Requirements
- **Integration Architecture**:
  - Test query set (AGENTOPT-0001) feeds testing harness
  - Variants (AGENTOPT-0002) loaded by tester
  - Testing harness (AGENTOPT-0003) executes experiments
  - Statistical analyzer (AGENTOPT-0004) processes results
  - Results inform next variant generation
  - Results aggregator (AGENTOPT-0005) synthesizes findings

- **Experiment Execution**:
  - Run 5 variants × 100 queries = 500 total searches
  - Use rule-based simulation for speed (upgrade to API if needed)
  - Collect full metrics per variant (success rate, latency, token usage)
  - Statistical comparison with p<0.05 threshold for significance

- **Deliverables**:
  - Experiment report JSON matching architecture.md specification (lines 1005-1026)
  - Human-readable summary with recommendations
  - Winner variant file ready for deployment
  - Complete experiment methodology documentation

## Implementation Notes
Create experiment runner orchestrating all Phase 0 components:

```typescript
// packages/maproom-mcp/test/tool-description-optimization/experiment.ts
async function runExperiment(variants: Variant[], queries: Query[]): Promise<ExperimentReport> {
  // 1. Load variants and test queries from Phase 0 components
  // 2. Run testing harness for each variant
  // 3. Collect metrics (success rate, latency, tokens)
  // 4. Statistical analysis using analyzer component
  // 5. Generate experiment report with methodology
  // 6. Recommend next steps based on results
  // 7. Prepare winner variant for Phase 1 deployment
}
```

**First experiment variants**:
- variant-control: Current description (~350 tokens)
- variant-a-detailed: Extensive patterns (~500 tokens)
- variant-b-simple: Bullet points (~200 tokens)
- variant-c-conversational: Natural language (~300 tokens)
- variant-d-code-like: Technical syntax (~400 tokens)

**Expected timeline**: 30-60 minutes for full experiment run.

**Integration workflow**:
1. Load Phase 0 test query set (100 queries from AGENTOPT-0001)
2. Load all 5 variants from variant system (AGENTOPT-0002)
3. For each variant: run testing harness (AGENTOPT-0003) on all 100 queries
4. Aggregate metrics for each variant
5. Run statistical analyzer (AGENTOPT-0004) on results
6. Generate experiment report using results aggregator (AGENTOPT-0005)
7. Identify winner and prepare for Phase 1

## Dependencies
- AGENTOPT-0001 (test query set - must provide 100 queries)
- AGENTOPT-0002 (variant system - must provide 5 loadable variants)
- AGENTOPT-0003 (testing harness - must execute single variant on query set)
- AGENTOPT-0004 (statistical analysis - must compare variant results)
- AGENTOPT-0005 (results aggregator - must synthesize findings)

## Risk Assessment
- **Risk**: No clear winner in first experiment
  - **Mitigation**: Generate more diverse variants, try different mutation strategies in Phase 1, increase query set diversity
- **Risk**: All variants worse than control
  - **Mitigation**: Deploy control to Phase 1, iterate on variant design strategy, gather user feedback on description usability
- **Risk**: Integration issues between components
  - **Mitigation**: Test each component independently first, validate interfaces match, create adapter layer if needed
- **Risk**: Experiment takes longer than expected (>2 hours)
  - **Mitigation**: Reduce query set to 50 per variant for testing, use rule-based simulation only, optimize harness execution

## Files/Packages Affected
- packages/maproom-mcp/test/tool-description-optimization/experiment.ts (create)
- .agents/projects/AGENTOPT_ai-agent-query-optimization/experiments/ (create for results storage)
- .agents/projects/AGENTOPT_ai-agent-query-optimization/tickets/ (documentation updates)

## Planning References
- Plan: planning/plan.md lines 465-493, 557-568
- Architecture: planning/architecture.md lines 804-1210
