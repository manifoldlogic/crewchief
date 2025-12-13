# Ticket: CICLEAN-3001: Local validation and verification

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
- bash-agent
- verify-ticket
- commit-ticket

## Summary
Perform comprehensive local validation of all changes to ensure CI will pass before pushing. Run the same checks that CI runs to verify no regressions.

## Background
This is the final validation phase to ensure all previous changes work together correctly. We need to verify:

1. Rust code compiles without feature flags
2. Rust tests pass without feature flags
3. E2E script can build binary and run successfully
4. TypeScript MCP tests still pass
5. YAML syntax is valid

This comprehensive check prevents CI failures and gives confidence the changes are correct.

**Planning Reference**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/quality-strategy.md`

## Acceptance Criteria
- [ ] `yamllint .github/workflows/test.yml` passes (or file is valid YAML)
- [ ] `cargo check` passes in `crates/maproom/` directory
- [ ] `cargo test -- --test-threads=1` passes in `crates/maproom/` directory
- [ ] `./tests/e2e/test_sqlite_flow.sh` executes successfully
- [ ] `pnpm test` passes in `packages/maproom-mcp/` directory
- [ ] No compilation errors in any package
- [ ] Summary of validation results documented

## Technical Requirements

### 1. Validate YAML syntax
**Location**: Repository root
**Command**:
```bash
yamllint .github/workflows/test.yml
```

**Expected**: No errors (warnings acceptable for line length, etc.)

**Alternative if yamllint not available**:
- Manually verify YAML structure is correct
- Use online YAML validator
- Check that workflow file is syntactically valid

### 2. Validate Rust compilation
**Location**: `crates/maproom/`
**Command**:
```bash
cd crates/maproom
cargo check
```

**Expected**:
```
Finished dev [unoptimized + debuginfo] target(s) in X.XXs
```

### 3. Run Rust tests
**Location**: `crates/maproom/`
**Command**:
```bash
cd crates/maproom
cargo test -- --test-threads=1
```

**Expected**: All tests pass, no failures

**Note**: `--test-threads=1` required for SQLite tests (database locking)

### 4. Run E2E test script
**Location**: Repository root
**Command**:
```bash
./tests/e2e/test_sqlite_flow.sh
```

**Expected**:
- Binary builds successfully
- All E2E tests pass
- Script exits with code 0

### 5. Run MCP TypeScript tests
**Location**: `packages/maproom-mcp/`
**Command**:
```bash
cd packages/maproom-mcp
pnpm test
```

**Expected**: All TypeScript tests pass

### 6. Document validation results
Create a summary showing:
- Which validations passed/failed
- Any warnings or issues encountered
- Confirmation that all critical paths work

## Implementation Notes

**Validation order**:
1. YAML syntax (fastest, catches syntax errors early)
2. Rust compilation (checks code builds)
3. Rust tests (checks functionality)
4. E2E tests (checks full integration)
5. TypeScript tests (checks MCP layer)

**Critical paths validated**:
- ✅ Rust compiles without `--features sqlite` flag
- ✅ Rust tests run without `--features sqlite` flag
- ✅ E2E binary builds without `--features sqlite` flag
- ✅ Test fixture generation works without `--features sqlite`
- ✅ TypeScript tests still pass (no regressions)

**What to do if validation fails**:
1. Review error messages carefully
2. Check which ticket introduced the issue
3. Fix the problem before proceeding
4. Re-run validation until all checks pass

**Success criteria**:
All validation commands must complete successfully. This confirms:
- CI workflow is valid YAML
- All code changes work correctly
- No regressions in existing functionality
- Changes achieve the intended goal (remove feature flags)

## Dependencies
- Depends on: CICLEAN-1001 (workflow changes)
- Depends on: CICLEAN-1002 (feature flag removal)
- Depends on: CICLEAN-1003 (documentation updates)
- Depends on: CICLEAN-2001 (E2E script fix)
- Depends on: CICLEAN-2002 (test helper updates)
- Depends on: CICLEAN-2003 (docs updates)

This is the final ticket - validates all previous changes.

## Risk Assessment

- **Risk**: Validation reveals regressions
  - **Mitigation**: Fix issues before marking ticket complete
  - **Impact**: High (blocks project completion)
  - **Probability**: Low (changes are well-scoped)

- **Risk**: Tests fail for environmental reasons
  - **Mitigation**: Use clean worktree; ensure dependencies installed
  - **Impact**: Medium (delays validation)
  - **Probability**: Low (standard test environment)

- **Risk**: Long test execution time
  - **Mitigation**: Run tests in parallel where possible; use `--test-threads=1` only where required
  - **Impact**: Low (one-time validation cost)
  - **Probability**: High (comprehensive test suite)

## Files/Packages Affected
No files modified - this ticket validates changes from previous tickets:
- `.github/workflows/test.yml` (validated)
- `crates/maproom/` (compiled and tested)
- `tests/e2e/test_sqlite_flow.sh` (executed)
- `packages/maproom-mcp/` (tested)
- `docs/testing/SQLITE_INTEGRATION_TESTS.md` (indirectly validated)
