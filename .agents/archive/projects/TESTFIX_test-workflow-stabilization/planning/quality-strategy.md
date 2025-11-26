# Quality Strategy: Test Workflow Stabilization

## Testing Philosophy

**Goal**: Ensure each fix works in CI environment before moving to next issue

**NOT**: Achieve 100% test coverage or perfect testing
**YES**: Confidence that fixes resolve actual CI failures without regression

## Test Strategy

### Per-Fix Verification

Each ticket fix must pass these gates:

**1. Local Syntax Verification**
```bash
# For SQL changes
psql -f packages/maproom-mcp/config/init.sql --set ON_ERROR_STOP=1

# For TypeScript changes
pnpm build

# For package.json changes
jq . package.json  # Validate JSON syntax
```

**2. CI Simulation (When Possible)**
```bash
# Test with CI environment variable set
CI=true pnpm install  # For husky-related changes

# Test with test database
TEST_MAPROOM_DATABASE_URL=postgresql://localhost:5432/maproom_test pnpm test
```

**3. Actual CI Run**
```bash
git push
gh run list --limit 1  # Check status
gh run view <run-id> --log  # View logs if failed
```

### Critical Paths

**Must Verify**:
1. **Schema Initialization**: init.sql applies without errors
2. **Dependency Installation**: pnpm install completes successfully
3. **Test Execution**: All tests in packages/maproom-mcp/tests/ pass

**Can Skip**:
- Performance benchmarks
- Load testing
- Edge case scenarios not exercised by existing tests

## Integration Points

### GitHub Actions Workflow
**File**: `.github/workflows/test.yml`

**Key Steps to Monitor**:
1. Checkout code
2. Setup Node.js
3. Setup pnpm
4. Install dependencies (husky runs here)
5. Start PostgreSQL service
6. Initialize test database schema (init.sql runs here)
7. Run tests (test files execute here)

### Database Schema
**File**: `packages/maproom-mcp/config/init.sql`

**Validation Points**:
- No SQL syntax errors
- All required functions exist
- All tables referenced by tests exist
- Extensions (pgvector, pg_trgm) load successfully

### Test Suite
**Files**: `packages/maproom-mcp/tests/*.cjs`

**Test Dependencies**:
- Database connection
- Schema initialization complete
- Required functions/tables exist
- Environment variables set correctly

## Risk Mitigation Through Testing

### High-Risk Changes
**Schema modifications in init.sql**
- **Why Risky**: Can break existing functionality
- **Mitigation**: Apply locally first, check for errors
- **Validation**: Run full test suite locally before pushing

### Medium-Risk Changes
**Package.json script modifications**
- **Why Risky**: Affects all pnpm operations
- **Mitigation**: Test with CI=true locally
- **Validation**: Check prepare/postinstall scripts work

### Low-Risk Changes
**Test file modifications**
- **Why Low Risk**: Doesn't affect schema or dependencies
- **Mitigation**: Basic syntax check
- **Validation**: Run test file directly

## MVP Testing Mindset

### What We DON'T Need

❌ **Unit tests for each fix**
- Reason: Fixes are to existing code/schema
- Alternative: Verify with existing test suite

❌ **Regression test suite**
- Reason: GitHub Actions workflow IS the regression test
- Alternative: Monitor workflow success rate

❌ **Test coverage metrics**
- Reason: Not measuring new code coverage
- Alternative: Ensure affected tests pass

### What We DO Need

✅ **Verification that fix works in CI**
- Method: Push and check workflow run

✅ **No new errors introduced**
- Method: All previous passing tests still pass

✅ **Root cause addressed**
- Method: Failure doesn't recur in future runs

## Test Execution

### Pre-Commit Checks
```bash
# Run before committing each fix
1. Syntax validation (SQL, JSON, TypeScript)
2. Local build verification (pnpm build)
3. Schema application check (if init.sql modified)
```

### Post-Push Verification
```bash
# After pushing fix
1. Wait for workflow to complete (~/1 minute)
2. Check workflow status: gh run list --limit 1
3. If failed: View logs and create next ticket
4. If passed: Check for next failure OR declare success
```

### Iteration Cycle
```
Fix → Verify Locally → Commit → Push → Check CI → Next Issue
  ↑                                                      │
  └──────────────────────────────────────────────────────┘
                  (Repeat until passes)
```

## Acceptance Criteria for Project Completion

### Hard Requirements
1. ✅ Test workflow shows green checkmark (passing)
2. ✅ All tests in `packages/maproom-mcp/tests/` pass
3. ✅ Schema initialization completes without errors
4. ✅ Dependency installation completes without errors

### Soft Requirements
1. Documentation updated (lessons learned in CLAUDE.md)
2. Future prevention measures identified
3. Team understands root causes

## Monitoring Post-Stabilization

### Short-Term (1 week)
- Check workflow success rate daily
- Alert on any new failures
- Quick fixes for regressions

### Long-Term (ongoing)
- Include workflow status in PR checks
- Require passing tests before merge
- Periodic review of test suite health

## Test Philosophy

**Pragmatic over Perfect**
- Tests prevent rework, not ceremonial
- CI workflow is the ultimate integration test
- Fast feedback trumps comprehensive coverage

**Ship Without Anxiety**
- Each fix independently verified
- No fear of cascading failures
- Rollback strategy clear (revert commits)

**Learn and Improve**
- Document what went wrong
- Update process to prevent recurrence
- Share knowledge with team
