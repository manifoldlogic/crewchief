# Ticket: COMPFIX-2003: Error Scenario Testing

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (manual validation ticket)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- verify-ticket
- commit-ticket

## Summary

Manually test all documented error scenarios to verify that validation catches failures before agent execution, error messages are actionable with clear troubleshooting steps, and no API credits are wasted on invalid setups. This confirms the fail-fast validation strategy works correctly in real-world failure modes.

## Background

The primary goal of pre-flight validation is to catch setup failures BEFORE spawning agents and wasting API credits. The validation infrastructure (COMPFIX-1001 through COMPFIX-1004) implements checks for:
- Database connectivity
- Base branch indexing
- Worktree scanning
- MCP configuration
- File permissions

However, these checks haven't been tested in actual failure scenarios. This ticket validates that:
1. All error conditions are caught by validation (not during agent execution)
2. Error messages match documentation (COMPFIX-2001)
3. Troubleshooting steps are actionable
4. No API calls are made when validation fails

This is a **manual testing ticket** performed by the verify-ticket agent to simulate real-world failures.

**Reference:** Section "Error Scenario Testing" in `planning/plan.md` (lines 216-232)

## Acceptance Criteria

- [ ] Database failure scenario tested: validation fails immediately, clear error message, no agents spawned
- [ ] Base branch not indexed scenario tested: validation fails with fix instructions
- [ ] Worktree scan failure scenario tested: competition stops, error is actionable
- [ ] MCP config malformed scenario tested: validation catches before agent spawn
- [ ] Permission denied scenario tested: validation fails with permission error
- [ ] All error messages match documentation from COMPFIX-2001
- [ ] No API credits wasted in any failure scenario (verified: no Anthropic API calls made)
- [ ] Results documented in `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenarios.md`

## Technical Requirements

### Error Scenarios to Test

#### 1. Database Unreachable

**Setup:**
```bash
# Stop PostgreSQL container
docker stop maproom-postgres

# Or set invalid DATABASE_URL
export MAPROOM_DATABASE_URL="postgresql://invalid:invalid@localhost:9999/fake"
```

**Execute:**
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

**Expected Outcome:**
- ❌ Validation fails in Phase 1: Setup
- ❌ Error message: "Database connection failed - check MAPROOM_DATABASE_URL"
- ❌ Troubleshooting steps shown (docker ps, psql test)
- ✅ No worktrees created
- ✅ No agents spawned
- ✅ No API calls made
- ✅ Exit code non-zero

**Verify:**
```bash
# Check no Anthropic API calls in logs
grep -i "anthropic" optimizer-run.log
# Should be empty

# Check no worktrees created
ls .crewchief/worktrees/
# Should be empty or only from previous runs

# Check exit code
echo $?
# Should be non-zero (1)
```

#### 2. Base Branch Not Indexed

**Setup:**
```bash
# Clear database for test repo
psql $MAPROOM_DATABASE_URL -c "DELETE FROM chunks WHERE repo_id = (SELECT id FROM repos WHERE name = 'crewchief-test')"

# Or modify competition config to use non-existent branch
```

**Execute:**
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

**Expected Outcome:**
- ❌ Validation fails in Phase 1: Setup (after database check)
- ❌ Error message: "Base branch not indexed - run: crewchief-maproom scan..."
- ❌ Includes one-time setup note
- ✅ No worktrees created
- ✅ No agents spawned
- ✅ No API calls made

**Verify:**
- Error message matches documentation
- Fix command is correct and executable
- User can follow instructions to resolve

#### 3. Worktree Scan Fails

**Setup:**
```bash
# Make worktree directory read-only to cause scan failure
# This requires creating worktree first, so we need to intercept mid-process

# Alternative: Mock scan failure in code temporarily
# Or: Fill disk space to cause write failures
```

**Execute:**
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

**Expected Outcome:**
- ❌ Validation fails in Phase 1: Setup (during scan phase)
- ❌ Error message: "Scan failed for <variant>: <reason>"
- ❌ Includes troubleshooting (check path, check permissions, check database)
- ✅ No agents spawned (even if some scans succeeded)
- ✅ No API calls made

**Verify:**
- Scan failure is caught
- Error includes variant name
- Error includes underlying cause (permission, disk space, etc.)
- Competition stops immediately (fail-fast)

#### 4. MCP Config Malformed

**Setup:**
```bash
# Create worktree manually with malformed .mcp.json
mkdir -p .crewchief/test-worktree
echo "{ invalid json" > .crewchief/test-worktree/.mcp.json

# Or modify variant injection to create invalid JSON
```

**Execute:**
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

**Expected Outcome:**
- ❌ Validation fails in Phase 2: Validation (per-variant checks)
- ❌ Error message: "MCP config invalid: <parse error>"
- ❌ Or: "Maproom MCP server not configured"
- ✅ No agents spawned
- ✅ No API calls made

**Verify:**
- JSON parse errors are caught
- Missing maproom server detected
- Error message explains what's wrong
- Points to example .mcp.json structure

#### 5. Permission Denied

**Setup:**
```bash
# Create worktree but make it read-only
mkdir -p .crewchief/test-worktree
chmod 444 .crewchief/test-worktree

# Competition will fail when trying to write test file
```

**Execute:**
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

**Expected Outcome:**
- ❌ Validation fails in Phase 2: Validation (file permissions check)
- ❌ Error message: "Permission error: EACCES"
- ❌ Troubleshooting includes chmod command
- ✅ No agents spawned
- ✅ No API calls made

**Verify:**
- Permission errors are caught
- Error explains which operation failed (read vs write)
- Fix command is provided (chmod)

### Documentation Template

Create file: `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenarios.md`

```markdown
# Error Scenario Testing Results

**Date:** 2025-11-XX
**Tester:** verify-ticket agent
**Environment:** [describe]

## Summary

| Scenario | Caught by Validation | Error Message Quality | No API Waste | Pass/Fail |
|----------|----------------------|-----------------------|--------------|-----------|
| Database unreachable | ✅/❌ | ✅/❌ | ✅/❌ | ✅/❌ |
| Base branch not indexed | ✅/❌ | ✅/❌ | ✅/❌ | ✅/❌ |
| Worktree scan fails | ✅/❌ | ✅/❌ | ✅/❌ | ✅/❌ |
| MCP config malformed | ✅/❌ | ✅/❌ | ✅/❌ | ✅/❌ |
| Permission denied | ✅/❌ | ✅/❌ | ✅/❌ | ✅/❌ |

## Scenario 1: Database Unreachable

### Setup Steps
```bash
docker stop maproom-postgres
```

### Execution
```bash
pnpm tsx scripts/run-genetic-optimizer.ts 2>&1 | tee error-scenario-1.log
```

### Actual Output
```
[paste actual console output]
```

### Validation
- ✅/❌ Caught before agent spawn
- ✅/❌ Error message matches documentation
- ✅/❌ Troubleshooting steps present
- ✅/❌ No API calls made (verify in logs)
- ✅/❌ Exit code non-zero

### Error Message Quality
- **Clarity**: [rate 1-5]
- **Actionability**: [rate 1-5]
- **Match docs**: ✅/❌

### Issues Found
[Any discrepancies or problems]

### Recommendations
[Improvements to error handling or messaging]

## [Repeat for each scenario]

## Conclusion

**Overall Result:** ✅ PASSED / ❌ FAILED

**Key Findings:**
1. [Finding 1]
2. [Finding 2]
3. [...]

**Issues to Fix:**
1. [Issue 1]
2. [Issue 2]
3. [...]

**Documentation Accuracy:**
- Error messages match docs: ✅/❌
- Troubleshooting steps work: ✅/❌
- Examples are accurate: ✅/❌
```

## Implementation Notes

### Testing Best Practices

1. **Isolate each scenario**: Reset environment between tests
2. **Document everything**: Save console output, logs, configs
3. **Verify no API waste**: Check logs for Anthropic API calls
4. **Test recovery**: After fixing error, verify competition succeeds
5. **Cross-reference docs**: Compare error messages to COMPFIX-2001

### Verification Checklist

For EACH error scenario:

- [ ] Error caught by validation (not during agent execution)
- [ ] Error message is human-readable (not raw stack trace)
- [ ] Error message explains WHAT failed
- [ ] Error message explains WHY it might have failed
- [ ] Error message includes HOW to fix it
- [ ] Troubleshooting steps are executable commands
- [ ] No agents were spawned (check logs)
- [ ] No API calls were made (check logs)
- [ ] Exit code is non-zero
- [ ] Error message matches documentation

### Environment Reset

Between scenarios:

```bash
# Restart PostgreSQL if stopped
docker start maproom-postgres
sleep 3

# Restore correct DATABASE_URL
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"

# Clean up test worktrees
rm -rf .crewchief/test-worktree/
chmod -R 755 .crewchief/worktrees/ 2>/dev/null || true

# Verify base branch indexed
crewchief-maproom status --repo crewchief --worktree main

# Clear previous logs
rm -f error-scenario-*.log
```

### API Waste Verification

**How to verify no API calls were made:**

1. **Check logs for Anthropic API patterns:**
   ```bash
   grep -i "anthropic\|api-key\|claude\|messages" error-scenario-*.log
   ```

2. **Check for agent spawn logs:**
   ```bash
   grep -i "spawning\|agent started\|running agent" error-scenario-*.log
   ```

3. **Verify competition directory not created:**
   ```bash
   ls -la .crewchief/competitions/
   # Should not have new directories from failed runs
   ```

4. **Check Anthropic dashboard (if available):**
   - Look at API usage metrics
   - Verify no calls during test time window

### Error Message Quality Criteria

Rate each error message on:

1. **Clarity (1-5):**
   - 5: Crystal clear, anyone can understand
   - 3: Understandable with some technical knowledge
   - 1: Cryptic, unclear what failed

2. **Actionability (1-5):**
   - 5: Includes exact commands to fix
   - 3: Explains what to check
   - 1: No guidance on how to fix

3. **Completeness (1-5):**
   - 5: Covers all aspects (what, why, how to fix)
   - 3: Covers some aspects
   - 1: Minimal information

**Minimum acceptable:** All ratings ≥ 4

### Troubleshooting Test Issues

**If validation doesn't catch error:**
- Check: Is validation enabled in competition runner?
- Check: Is error occurring AFTER validation phase?
- Bug: Validation logic needs fixing

**If error message is unclear:**
- Document: What would make it clearer?
- Update: COMPFIX-2001 documentation ticket
- Fix: Update error message in code

**If test environment is broken:**
- Reset: Follow environment reset steps
- Verify: Run successful E2E test (COMPFIX-2002)
- Continue: Retry failed scenario

## Dependencies

- **Prerequisite tickets:**
  - COMPFIX-1001 (Pre-Flight Validation Module) - REQUIRED (contains validation logic)
  - COMPFIX-1002 (Scan Orchestration Module) - REQUIRED (for scan failures)
  - COMPFIX-1003 (Enhanced Competition Runner) - REQUIRED (for fail-fast behavior)
  - COMPFIX-2001 (Update Documentation) - REQUIRED (for error message comparison)

- **External dependencies:**
  - Ability to start/stop PostgreSQL
  - Ability to modify file permissions
  - Access to logs and console output

- **Complements:**
  - COMPFIX-2002 (End-to-End Validation) - tests success paths, this tests failure paths

## Risk Assessment

- **Risk**: Simulating failures might corrupt development environment
  - **Mitigation**: Use test database, clean up after each scenario
  - **Recovery**: Document environment reset steps

- **Risk**: Some failure modes hard to simulate
  - **Mitigation**: Use mocks or code modifications if needed
  - **Alternative**: Document theoretical behavior if simulation impossible

- **Risk**: Error messages might evolve, causing docs mismatch
  - **Mitigation**: Update both code and docs in same commit
  - **Process**: Run this test suite after any error handling changes

## Files/Packages Affected

**New files:**
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenarios.md`
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenario-1.log`
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenario-2.log`
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenario-3.log`
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenario-4.log`
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenario-5.log`

**No code modifications** - testing only
