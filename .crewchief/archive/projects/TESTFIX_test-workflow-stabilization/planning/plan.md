# Execution Plan: Test Workflow Stabilization

## Project Overview

**Goal**: Achieve 100% passing Test workflow through iterative fix-verify-push cycles

**Approach**: Create ticket for each failure → Fix → Verify → Commit → Push → Check next failure → Repeat

**Success Metric**: Green checkmark on Test workflow in GitHub Actions

## Phase Organization

### Phase 0: Completed Fixes (Baseline)
**Status**: ✅ COMPLETE

**Tickets Completed**:
- CIFIX-4001: Disable husky in CI environments
  - Fixed `prepare: "husky"` → `prepare: "[ -z \"$CI\" ] && husky || exit 0"`
  - Verified working in CI logs
- SQL Syntax Fixes: Fixed 5 COMMENT statement concatenations
  - Converted multi-line `||` format to single-line strings
  - All SQL syntax errors resolved

**Outcome**: Prepare script and schema initialization now pass

### Phase 1: Current Failure - Missing Database Function
**Status**: 🔄 IN PROGRESS

**Ticket**: TESTFIX-1001
**Issue**: Function `maproom.compute_git_blob_sha` doesn't exist
**Test**: `packages/maproom-mcp/tests/run-blob-sha-tests.cjs`

**Implementation Steps**:
1. Create ticket TESTFIX-1001 with:
   - Description of missing function
   - Test expectations (SHA-256 hash of git blob format)
   - Implementation approach
   - Acceptance criteria
2. Implement function in init.sql:
   - Search for existing implementation or reference
   - Create SQL function matching test expectations
   - Add appropriate comments
3. Verify locally:
   - Apply init.sql to test database
   - Run test file directly
   - Confirm function exists and produces correct hashes
4. Commit and push
5. Monitor workflow run

**Agent Assignment**: database-engineer (SQL function implementation)

**Deliverables**:
- Updated init.sql with compute_git_blob_sha function
- Passing run-blob-sha-tests.cjs
- Commit with proper message referencing TESTFIX-1001

**Risk**: Function implementation complexity unknown until investigated

### Phase 2: Discovery - Next Failure (TBD)
**Status**: ⏳ PENDING (after Phase 1)

**Trigger**: Phase 1 committed and pushed

**Actions**:
1. Check latest workflow run: `gh run list --limit 1`
2. If failed:
   - View logs: `gh run view <run-id> --log`
   - Identify failure type and location
   - Create TESTFIX-1002 ticket
   - Proceed to implementation
3. If passed:
   - Move to Phase 3 (Verification)

**Possible Failure Types**:
- Missing database tables (e.g., `maproom.symbols`)
- Missing database columns
- Test environment issues
- Dependency problems
- Other schema mismatches

**Agent Assignment**: TBD based on failure type

### Phase 3: Additional Failures (If Any)
**Status**: ⏳ PENDING

**Iterative Process**:
```
For each failure found:
1. Create TESTFIX-100X ticket
2. Assign to appropriate agent:
   - database-engineer: Schema/SQL issues
   - general-implementation-agent: Code/config issues
   - github-actions-specialist: Workflow issues
3. Implement fix
4. Verify and commit
5. Push and check next run
6. Repeat until no failures
```

**Exit Conditions**:
- ✅ SUCCESS: Test workflow passes completely
- ❌ ABORT: Max iterations reached (10) without resolution
- ❌ ABORT: Fundamental architectural issue discovered

**Estimated Iterations**: 2-4 additional failures expected

### Phase 4: Final Verification
**Status**: ⏳ PENDING (after all fixes)

**Trigger**: Test workflow shows green checkmark

**Verification Steps**:
1. Confirm workflow status: All steps passing
2. Verify test results: All tests in test suite pass
3. Check logs: No warnings or concerning messages
4. Run workflow multiple times: Ensure consistency

**Deliverables**:
- Documented list of all fixes applied
- Updated CLAUDE.md with lessons learned
- Project marked as complete

## Testing Milestones

### Milestone 1: Schema Initialization Passes
**Current Status**: ✅ COMPLETE (SQL syntax fixes done)
**Criteria**: init.sql applies without errors
**Verification**: Check "Initialize test database schema" step logs

### Milestone 2: Dependencies Install Successfully
**Current Status**: ✅ COMPLETE (husky fix done)
**Criteria**: pnpm install completes without fatal errors
**Verification**: Check "Install dependencies" step logs

### Milestone 3: Test Suite Passes
**Current Status**: ❌ FAILING (missing function)
**Criteria**: All tests in `packages/maproom-mcp/tests/` pass
**Verification**: Check "Run tests" step shows all passing

### Milestone 4: Workflow Completes
**Current Status**: ❌ FAILING
**Criteria**: No step failures, green checkmark
**Verification**: GitHub Actions UI shows passing status

## Security Checkpoints

### Per-Fix Security Review
**When**: Before committing each fix
**Check**:
- No SQL injection vulnerabilities
- No hardcoded secrets
- No unsafe script execution

**Responsible**: Implementation agent + verify-ticket agent

### Post-Push Security Check
**When**: After each workflow run
**Check**:
- No secrets exposed in logs
- No unexpected network calls
- Database remains isolated

**Responsible**: Manual review of workflow logs

## Agent Assignments

### Existing Agents

**database-engineer**:
- TESTFIX-1001: Add compute_git_blob_sha function
- Any future schema-related fixes
- SQL function implementations

**general-implementation-agent**:
- Non-SQL code fixes
- Test file modifications
- Configuration changes

**github-actions-specialist**:
- Workflow file changes (if needed)
- CI environment issues

**verify-ticket**:
- Verify all fixes meet acceptance criteria
- Check for regressions
- Validate implementation quality

**commit-ticket**:
- Create proper commit messages
- Reference ticket IDs
- Follow conventional commit format

### Workflow Orchestration

**ticket-creator**:
- Create new tickets for each discovered failure
- Extract issue details from workflow logs
- Define acceptance criteria

**Coordination**:
- User drives iteration cycle
- Checks workflow status after each push
- Triggers ticket creation for next failure

## Deliverables

### Per-Ticket Deliverables
1. Ticket file in `.crewchief/projects/TESTFIX_*/tickets/`
2. Code/schema changes addressing issue
3. Passing verification from verify-ticket agent
4. Commit with conventional format and ticket reference

### Project Deliverables
1. All Test workflow steps passing
2. Documentation of fixes in ticket files
3. Updated CLAUDE.md with:
   - Common failure patterns
   - Prevention strategies
   - Lessons learned
4. Clean git history showing progression

## Risk Management

### Identified Risks

**Risk 1: Infinite Loop**
- **Scenario**: Fix creates new failure
- **Probability**: Low
- **Mitigation**: Root cause analysis for each fix
- **Contingency**: Max 10 iterations, then reassess

**Risk 2: Fundamental Issue**
- **Scenario**: Workflow requires major architectural change
- **Probability**: Very Low
- **Mitigation**: Early investigation of failure patterns
- **Contingency**: Escalate to architectural review

**Risk 3: Time Investment**
- **Scenario**: Too many failures to fix iteratively
- **Probability**: Medium
- **Mitigation**: Batch related fixes when appropriate
- **Contingency**: Prioritize blocking issues first

### Contingency Plans

**If 5+ failures remain after 10 iterations**:
1. Pause iterative approach
2. Analyze all failures together
3. Look for common root cause
4. Consider batch fix or architectural change

**If fundamental schema mismatch discovered**:
1. Document the mismatch
2. Evaluate options: Fix schema OR fix tests
3. Make architectural decision
4. Implement comprehensive fix

**If time budget exceeded**:
1. Document progress so far
2. Mark highest-priority fixes as Phase 1
3. Defer lower-priority fixes to Phase 2
4. Ship with known limitations if acceptable

## Success Criteria

### Hard Requirements (Must Have)
✅ Test workflow shows green checkmark
✅ All tests in `packages/maproom-mcp/tests/` pass
✅ Schema initialization completes without errors
✅ Dependency installation completes without errors

### Soft Requirements (Should Have)
- Documentation of lessons learned
- Prevention strategies identified
- Clean commit history
- No warnings in workflow logs

### Exclusions (Not Required)
- 100% test coverage
- Performance optimization
- New test additions
- Refactoring beyond fixes

## Timeline Estimate

**Per Iteration**:
- Ticket creation: 5 minutes
- Implementation: 10-20 minutes
- Verification: 5 minutes
- CI run: 1-2 minutes
- Total: 20-30 minutes per fix

**Expected Total**:
- Remaining fixes: 2-4
- Total time: 40-120 minutes
- Buffer for unknowns: +50%
- **Estimated completion**: 1-3 hours

**Note**: Timeline assumes standard failures. Fundamental issues could extend significantly.

## Post-Completion

### Immediate Next Steps
1. Update .github/CLAUDE.md with common CI issues
2. Document workflow failure patterns
3. Create prevention checklist for future schema changes

### Long-Term Improvements
1. Add pre-commit hook for SQL syntax validation
2. Create local test script matching CI environment
3. Consider migration-based schema management
4. Add workflow status badge to README

## Project Completion Criteria

**Declare Success When**:
1. Latest Test workflow run shows all green
2. Workflow has passed 3 consecutive times
3. All tickets marked complete
4. Documentation updated

**Archive Project When**:
1. All success criteria met
2. Team reviewed and approved
3. Knowledge transferred to .github/CLAUDE.md
4. No active work remaining
