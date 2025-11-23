# Analysis: Test Workflow Stabilization

## Problem Definition

The Test workflow in GitHub Actions is failing with multiple unrelated issues that prevent successful CI runs. Each failure blocks releases and development progress. The current state requires systematic identification and resolution of all failure points.

## Current State

**Successfully Fixed Issues:**
1. ✅ **CIFIX-4001**: Husky prepare script failing in CI
   - Root cause: `prepare: "husky"` script ran in CI where husky binary wasn't available
   - Solution: Conditional execution `[ -z "$CI" ] && husky || exit 0`
   - Status: VERIFIED WORKING in latest runs

2. ✅ **SQL Syntax Errors**: Multiple COMMENT statements using `||` concatenation
   - Root cause: PostgreSQL psql doesn't support multi-line string concatenation in COMMENT statements
   - Fixed 5 instances (1 COMMENT ON TABLE, 4 COMMENT ON FUNCTION)
   - Status: All SQL syntax errors resolved

**Current Active Failure:**
3. ❌ **Missing Database Function**: `compute_git_blob_sha`
   - Test: `packages/maproom-mcp/tests/run-blob-sha-tests.cjs`
   - Error: Function `maproom.compute_git_blob_sha` doesn't exist in database schema
   - Impact: Test suite fails, blocking workflow completion

**Potential Future Issues:**
- Unknown until current failure is resolved
- Pattern suggests there may be schema/test mismatches
- Need iterative approach to discover and fix remaining issues

## Problem Context

### Why This Matters
- **Blocks Releases**: Cannot publish new versions without passing tests
- **Blocks Development**: Developers cannot merge PRs confidently
- **Compounds Over Time**: Each new change risks introducing more failures

### Historical Context
- Husky issue: Introduced when CI environment changed or husky setup modified
- SQL syntax errors: Likely introduced during schema enhancement (context cache feature)
- Missing function: Test expects schema feature that wasn't committed

## Existing Solutions & Approaches

### Industry Standard: CI/CD Stability
1. **Progressive Fixes**: Fix one issue, verify, move to next
2. **Ticket-Based Workflow**: Each fix gets dedicated ticket for tracking
3. **Automated Verification**: Run workflow after each fix to discover next issue
4. **Root Cause Analysis**: Don't just fix symptoms, understand why

### Current Workflow Process
1. Identify failure from GitHub Actions logs
2. Create ticket documenting issue and fix approach
3. Implement fix using `/single-ticket` workflow
4. Push changes and trigger new workflow run
5. Check for next failure
6. Repeat until all tests pass

## Research Findings

### Test Failure Pattern Analysis
Looking at the test file structure in `packages/maproom-mcp/tests/`:
- `connection-fallback.test.cjs` - ✅ PASSING
- `run-blob-sha-tests.cjs` - ❌ FAILING (missing function)

The blob SHA test is comprehensive (227 lines) and tests:
- Function existence
- Known hash values matching Rust implementation
- Determinism (same input = same output)
- Unicode handling
- Large content handling
- Newline handling (LF vs CRLF)

This suggests the function was intentionally added to the test suite but the corresponding SQL function was never committed to `init.sql`.

### Schema vs Test Mismatch
Common causes:
1. **Local Development Schema Drift**: Developer tested locally with uncommitted schema changes
2. **Feature Branch Merge Issues**: Schema changes from one branch, tests from another
3. **Migration Not Applied**: Function exists in migration file not yet run in CI

### Similar Issues in Other Codebases
- **Verification Query Assumes Non-Existent Table**: The workflow's "Verify test database usage" step queries `maproom.symbols` table which also doesn't exist
- This is a non-fatal error (has `|| echo "Note: No test data found"`)
- But indicates broader schema/assumption mismatches

## Key Insights

1. **Schema Inconsistency**: Tests expect schema features not present in `init.sql`
2. **Multiple Failure Modes**: Issues are independent (husky, SQL syntax, missing functions)
3. **Iterative Discovery**: Can only find next failure after fixing current one
4. **Test Environment Sensitivity**: CI environment differs from local development

## Risk Assessment

### High Risk
- **More Hidden Failures**: Unknown number of additional issues waiting
- **Schema Divergence**: Other missing tables/functions/columns

### Medium Risk
- **Test Suite Incompleteness**: Tests may expect features not in schema
- **Time Investment**: Each iteration requires CI run (1+ minute)

### Low Risk
- **Breaking Local Development**: Fixes target CI-specific issues
- **Regression**: Each fix is isolated and testable

## Constraints

1. **Must Be Iterative**: Cannot discover all failures upfront
2. **CI Feedback Loop**: Must push to trigger workflow
3. **Systematic Approach**: Need ticket for each fix for tracking
4. **Test Workflow Only**: Focus on Test workflow, not Build/Publish workflows

## Success Criteria

- Test workflow passes completely (green checkmark)
- All tests in `packages/maproom-mcp/tests/` pass
- Schema initialization completes without errors
- No fatal errors in any workflow step
