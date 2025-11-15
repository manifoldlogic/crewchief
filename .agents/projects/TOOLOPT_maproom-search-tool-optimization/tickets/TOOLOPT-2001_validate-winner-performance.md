# Ticket: TOOLOPT-2001: Run validation benchmark comparing control vs winner variant

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Execute benchmark test comparing variant-control (17.7% baseline) against variant-a-detailed (19.6% winner) to confirm performance gain replicates outside genetic optimizer environment.

## Background
Before deploying to production, need to validate that the +1.9% performance gain observed in genetic optimization replicates in standalone testing. This ensures the improvement is real and not an artifact of the optimization environment.

This ticket is part of Phase 2 (Production Deployment) of the TOOLOPT project, which aims to deploy the proven winner tool description to production with validation.

## Acceptance Criteria
- [ ] Benchmark test executed with 5 iterations for statistical reliability
- [ ] variant-a-detailed scores ≥19.0% (allows 0.6% margin for variance)
- [ ] Test results documented with:
  - [ ] Individual iteration scores
  - [ ] Average scores for both variants
  - [ ] Performance delta (expected: ~+1.9%)
  - [ ] Quality signals analysis
- [ ] Test output saved for PR evidence
- [ ] Results confirm deployment readiness

## Technical Requirements
- Test command:
  ```bash
  npx tsx src/search-optimization/run-comparison.ts \
    variant-control variant-a-detailed \
    --tasks=impl-worktree-001 --iterations=5
  ```
- Source variants: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/`
- Test framework: existing comparison runner
- Success threshold: ≥19.0% (0.6% margin below 19.6%)

## Implementation Notes
- Run in same environment as genetic optimizer
- 5 iterations minimum for variance assessment
- Document any anomalies or unexpected results
- If score <19.0%: investigate before proceeding
- If 17.0-18.9%: don't deploy, investigate variance
- Test runtime: ~30 minutes
- Expected results:
  - variant-control: ~17.7%
  - variant-a-detailed: ≥19.0%
  - Delta: ~+1.9%

## Dependencies
- None (can run independently)
- Phase 1 documentation complete (context)

## Risk Assessment
- **Risk**: Performance doesn't replicate
  - **Mitigation**: Use 5 iterations, set threshold at 19.0% (0.6% margin for variance)
- **Risk**: Test environment differs from genetic optimizer
  - **Mitigation**: Use same setup as genetic runs, same task set, same evaluation criteria

## Files/Packages Affected
- `/workspace/packages/cli/src/search-optimization/run-comparison.ts` (test runner)
- `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/variant-control.json`
- `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/variant-a-detailed.json`
- Test output files (new, for PR evidence)

## Estimated Time
45 minutes (30 min test runtime + 15 min analysis)
