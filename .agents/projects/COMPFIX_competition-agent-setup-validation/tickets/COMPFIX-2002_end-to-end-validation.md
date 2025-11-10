# Ticket: COMPFIX-2002: End-to-End Validation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (manual validation ticket)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- verify-ticket
- commit-ticket

## Summary

Execute all three genetic optimizer configurations (standard, premium, ultra) end-to-end to verify the competition framework works correctly with real agent execution, validate that agents have tool access, measure actual timing metrics, and confirm at least 50% of agents use semantic search tools.

## Background

All previous tickets (COMPFIX-1001 through COMPFIX-2001) implemented the validation infrastructure, but haven't been tested with actual agent execution in a full competition. This ticket is the critical validation step that proves:

1. **Validation works**: All pre-flight checks pass in real environments
2. **Agents have tools**: At least 50% use `mcp__maproom__search` (vs 0% previously)
3. **Timing is acceptable**: Setup overhead < 5 minutes for 12 variants
4. **Results are meaningful**: Competition scores reflect actual tool usage, not all ~18% baseline

This is a **manual execution ticket** performed by the verify-ticket agent. The goal is to run actual optimizer scripts and document real-world behavior, not write automated tests.

**Reference:** Section "End-to-End Validation" in `planning/plan.md` (lines 196-215)

## Acceptance Criteria

- [ ] Standard optimizer (5 variants) completes successfully without errors
- [ ] Premium optimizer (8 variants) completes successfully without errors
- [ ] Ultra optimizer (12 variants) completes successfully without errors
- [ ] All validation phases log clearly: Setup → Validation → Execution
- [ ] At least 50% of agents use `mcp__maproom__search` tool across all runs
- [ ] No agents have 0 searches (confirms all have tool access)
- [ ] Actual setup time < 5 minutes for ultra configuration
- [ ] Competition reports show meaningful score variation (not all ~18%)
- [ ] Timing metrics documented: per-variant scan time, validation time, execution time
- [ ] Console output matches documentation examples
- [ ] All three runs saved to `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/`

## Technical Requirements

### Optimizer Configurations to Test

**1. Standard Optimizer (5 variants)**
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

**Expected:**
- Variants: 5 (e.g., control, detailed-a, simple-a, detailed-b, simple-b)
- Setup time: ~1-2 minutes
- Execution time: ~2-3 minutes per generation
- Total first generation: ~3-5 minutes

**2. Premium Optimizer (8 variants)**
```bash
pnpm tsx scripts/run-genetic-optimizer-premium.ts
```

**Expected:**
- Variants: 8
- Setup time: ~2-3 minutes
- Execution time: ~3-4 minutes per generation
- Total first generation: ~5-7 minutes

**3. Ultra Optimizer (12 variants)**
```bash
pnpm tsx scripts/run-genetic-optimizer-ultra.ts
```

**Expected:**
- Variants: 12
- Setup time: ~3-4 minutes
- Execution time: ~4-5 minutes per generation
- Total first generation: ~7-9 minutes

### Metrics to Collect

For EACH optimizer run, document:

1. **Setup Phase Timing**
   - Database validation: X ms
   - Base branch check: X ms
   - Worktree creation: X ms per variant
   - Variant injection: X ms per variant
   - Worktree scanning: X ms per variant (and total)
   - Environment validation: X ms per variant (and total)
   - **Total setup time**: X minutes

2. **Agent Execution Timing**
   - Per-agent execution: X seconds each
   - Total parallel time: X minutes
   - **Total execution time**: X minutes

3. **Tool Usage Statistics**
   - Total agents: X
   - Agents using search: X (percentage)
   - Total search calls: X
   - Average searches per agent: X
   - Min/max searches: X / X

4. **Competition Results**
   - Winner variant: X
   - Winner score: X
   - Score distribution: min X, max X, mean X, stddev X
   - Tool usage correlation: Did agents with more searches score better?

5. **Validation Status**
   - All validation checks: PASSED / FAILED
   - Any warnings: (list)
   - Any errors: (list)

### Success Criteria

**Quantitative:**
- ✅ All 3 optimizer levels complete without errors
- ✅ At least 50% of agents use search (vs 0% in broken state)
- ✅ No agents have 0 searches (all have tool access)
- ✅ Setup time < 5 minutes for 12 variants
- ✅ Score distribution shows variance (not all agents ~18%)

**Qualitative:**
- ✅ Console output is clear and actionable
- ✅ Error messages (if any) match documentation
- ✅ Validation phases are clearly visible
- ✅ Reports include setup metrics
- ✅ User experience is smooth (no confusing states)

### Validation Results Documentation

Create file: `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/e2e-results.md`

**Template:**

```markdown
# End-to-End Validation Results

**Date:** 2025-11-XX
**Tester:** verify-ticket agent
**Environment:** [describe: OS, Node version, PostgreSQL version]

## Summary

- ✅/❌ Standard optimizer (5 variants)
- ✅/❌ Premium optimizer (8 variants)
- ✅/❌ Ultra optimizer (12 variants)

## Standard Optimizer (5 variants)

### Command
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

### Setup Phase Timing
- Database validation: Xms
- Base branch check: Xms
- Worktree creation: Xms per variant (Xms total)
- Variant injection: Xms per variant (Xms total)
- Worktree scanning: Xms per variant (Xms total)
- Environment validation: Xms per variant (Xms total)
- **Total setup time**: X.Xs

### Execution Phase Timing
- Agent execution: Xs per agent (X agents in parallel)
- **Total execution time**: X.Xs

### Tool Usage
- Total agents: X
- Agents using search: X (X%)
- Total search calls: X
- Average searches per agent: X
- Search usage breakdown:
  - variant-control: X searches
  - variant-a-detailed: X searches
  - [etc.]

### Competition Results
- Winner: [variant-name]
- Winner score: X
- Score distribution:
  - Min: X
  - Max: X
  - Mean: X
  - Stddev: X

### Observations
[Anything notable about the run]

### Console Output (excerpt)
```
[paste key sections of console output]
```

### Report File
[paste content of report.txt]

## [Repeat for Premium and Ultra]

## Conclusion

- Validation status: ✅ PASSED / ❌ FAILED
- Tool access: ✅ All agents have tools / ❌ Some agents missing tools
- Timing acceptable: ✅ YES / ❌ NO (actual: X minutes)
- Scores meaningful: ✅ YES / ❌ NO (distribution: X)

## Issues Found
[List any issues discovered during testing]

## Recommendations
[Any improvements suggested based on results]
```

## Implementation Notes

### Pre-Validation Setup

Before running optimizers:

1. **Ensure PostgreSQL is running:**
   ```bash
   cd packages/maproom-mcp/config
   docker compose ps
   # If not running:
   docker compose up -d
   ```

2. **Verify base branch is indexed:**
   ```bash
   crewchief-maproom status --repo crewchief --worktree main
   # Should show chunks > 0
   # If not, run:
   crewchief-maproom scan --repo crewchief --worktree main --root /workspace
   ```

3. **Check Anthropic API key:**
   ```bash
   echo $ANTHROPIC_API_KEY
   # Should be set
   ```

4. **Clean previous runs (optional):**
   ```bash
   rm -rf .crewchief/competitions/*
   rm -rf .crewchief/genetic-iterations/*
   ```

### During Execution

**Monitor these aspects:**

1. **Console output**: Save to file for analysis
   ```bash
   pnpm tsx scripts/run-genetic-optimizer.ts 2>&1 | tee optimizer-run.log
   ```

2. **System resources**: Monitor CPU, memory, disk
   ```bash
   htop  # In separate terminal
   ```

3. **Database activity**: Check PostgreSQL connections
   ```bash
   psql $MAPROOM_DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity"
   ```

4. **Worktree creation**: Verify worktrees are created
   ```bash
   ls -la .crewchief/worktrees/
   ```

### Post-Execution Analysis

**Extract metrics from reports:**

1. **Find competition reports:**
   ```bash
   find .crewchief/genetic-iterations/ -name "report.txt" -exec head -50 {} \;
   ```

2. **Count tool usage:**
   ```bash
   grep "mcp__maproom__search" .crewchief/genetic-iterations/*/gen-*/comp-*/report.txt
   ```

3. **Calculate timing:**
   - Parse setup phase logs for timestamps
   - Calculate duration between phases
   - Sum per-variant scan times

4. **Analyze scores:**
   - Extract winner and scores from reports
   - Calculate distribution (min, max, mean, stddev)
   - Compare to baseline (~18% without tools)

### Troubleshooting

**If validation fails:**

1. **Check error message**: Should match documentation (COMPFIX-2001)
2. **Verify environment**: Database, base branch, API key
3. **Check logs**: Look for stack traces or detailed errors
4. **Try smaller config**: If ultra fails, try standard first
5. **Manual verification**: Run individual validation checks

**If agents don't use search:**

1. **Check MCP config**: Verify .mcp.json in worktrees
2. **Check worktree indexing**: Run `maproom status --repo crewchief --worktree <variant>`
3. **Check agent logs**: Look for tool availability messages
4. **Verify tool descriptions**: Check variant injection worked

**If timing is too long:**

1. **Check embedding reuse**: Subsequent scans should be fast
2. **Check parallelism**: Agents should run in parallel
3. **Check database**: Slow queries?
4. **Check network**: Database connection latency?

## Dependencies

- **Prerequisite tickets:**
  - COMPFIX-1001 (Pre-Flight Validation Module) - REQUIRED
  - COMPFIX-1002 (Scan Orchestration Module) - REQUIRED
  - COMPFIX-1003 (Enhanced Competition Runner) - REQUIRED
  - COMPFIX-1004 (Security Controls) - REQUIRED
  - COMPFIX-2001 (Update Documentation) - Helpful for troubleshooting

- **External dependencies:**
  - PostgreSQL running
  - Base branch indexed
  - Anthropic API key configured
  - Sufficient API credits for 3 runs

- **Blocks:**
  - Project sign-off (this is final validation)

## Risk Assessment

- **Risk**: API credits insufficient for 3 full runs
  - **Mitigation**: Use shorter timeout (60s per agent) for testing
  - **Alternative**: Run only standard optimizer (5 variants) if budget constrained

- **Risk**: Long execution time (several hours)
  - **Mitigation**: Runs can be done asynchronously (overnight)
  - **Tracking**: Use `nohup` or `screen` for long runs

- **Risk**: Flaky failures due to API rate limits
  - **Mitigation**: Add delays between optimizer runs
  - **Retry**: Re-run failed configurations once

- **Risk**: Results don't show 50% tool usage
  - **Investigation**: This would indicate a bug in validation or MCP setup
  - **Escalation**: Report as blocker for project completion

## Files/Packages Affected

**New files:**
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/e2e-results.md`
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/optimizer-standard-run.log`
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/optimizer-premium-run.log`
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/optimizer-ultra-run.log`

**No code modifications** - validation only
