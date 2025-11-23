# TESTFIX: Test Workflow Stabilization

## Project Summary

**Goal**: Achieve 100% passing Test workflow through systematic identification and resolution of all failure points.

**Approach**: Iterative fix-verify-push cycles, creating one ticket per failure until the workflow passes completely.

**Current Status**: Phase 1 - Fixing missing `compute_git_blob_sha` database function

## Problem Statement

The GitHub Actions Test workflow is failing with multiple unrelated issues that prevent successful CI runs. Each failure blocks releases and development progress. The failures are independent and can only be discovered by fixing the current one and checking the next workflow run.

**Root Causes Identified**:
1. ✅ **FIXED**: Husky prepare script failing in CI (CIFIX-4001)
2. ✅ **FIXED**: SQL syntax errors in COMMENT statements (5 instances)
3. ❌ **ACTIVE**: Missing database function `compute_git_blob_sha`
4. ⏳ **UNKNOWN**: Additional failures to be discovered

## Proposed Solution

### Iterative Fix-Push-Verify Loop

```
┌─────────────────────────────────────────────────┐
│ 1. Check latest workflow run for failures      │
│ 2. Create ticket for identified failure        │
│ 3. Implement fix using /single-ticket workflow │
│ 4. Verify and commit fix                       │
│ 5. Push to trigger new CI run                  │
│ 6. Repeat until workflow passes                │
└─────────────────────────────────────────────────┘
```

### Key Principles
- **One ticket per issue**: Each fix independently tracked
- **Push after each fix**: Immediate CI feedback
- **Root cause focus**: Fix problems, not symptoms
- **Schema truth**: init.sql is source of truth

## Execution Agents

### Primary Agents
- **database-engineer**: SQL schema and function implementations
- **verify-ticket**: Validate fixes meet acceptance criteria
- **commit-ticket**: Create proper conventional commits

### Supporting Agents
- **ticket-creator**: Generate tickets for discovered failures
- **general-implementation-agent**: Non-SQL fixes
- **github-actions-specialist**: Workflow configuration issues

## Planning Documents

### Core Planning
- **[analysis.md](planning/analysis.md)**: Problem space analysis, current state, research findings
- **[architecture.md](planning/architecture.md)**: Solution design, iteration cycle, technology choices
- **[plan.md](planning/plan.md)**: Phase organization, agent assignments, deliverables

### Quality & Security
- **[quality-strategy.md](planning/quality-strategy.md)**: Testing approach, verification gates, MVP mindset
- **[security-review.md](planning/security-review.md)**: Risk assessment, mitigations, security sign-off

## Current Phase: Phase 1

**Ticket**: TESTFIX-1001 (to be created)
**Issue**: Missing database function `compute_git_blob_sha`
**Test**: `packages/maproom-mcp/tests/run-blob-sha-tests.cjs` expects this function

**Next Steps**:
1. Create TESTFIX-1001 ticket
2. Search for function implementation or reference
3. Add function to init.sql
4. Verify locally and in CI
5. Push and check for next failure

## Success Criteria

### Hard Requirements
✅ Test workflow shows green checkmark
✅ All tests pass in `packages/maproom-mcp/tests/`
✅ Schema initialization completes without errors
✅ Dependency installation completes without errors

### Completion Metrics
- Workflow passes 3 consecutive times
- All tickets marked complete
- Documentation updated with lessons learned

## Timeline Estimate

**Per Fix**: 20-30 minutes (ticket creation + implementation + CI run)
**Expected Fixes**: 2-4 remaining
**Total Estimate**: 1-3 hours to completion

**Note**: Assumes standard failures; fundamental issues could extend timeline significantly.

## Documentation

All work documented in:
- **Tickets**: `.agents/projects/TESTFIX_test-workflow-stabilization/tickets/`
- **Planning**: `.agents/projects/TESTFIX_test-workflow-stabilization/planning/`
- **Lessons Learned**: `.github/CLAUDE.md` (after completion)

## Related Projects

- **CIFIX** (CI Workflow Fixes): Previous stabilization work
  - CIFIX-4001: Husky CI fix (completed, working)
  - SQL syntax fixes (completed)

This project continues the stabilization work started in CIFIX, using the same systematic approach.
