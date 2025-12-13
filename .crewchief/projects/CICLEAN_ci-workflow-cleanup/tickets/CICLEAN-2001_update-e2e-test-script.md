# Ticket: CICLEAN-2001: Update E2E test script to remove feature flag

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
- code-editor
- verify-ticket
- commit-ticket

## Summary
Remove `--features sqlite` flag from cargo build command in E2E test script and update error messages to provide correct fixture generation command.

## Background
The E2E test script `tests/e2e/test_sqlite_flow.sh` attempts to build the binary with:
```bash
cargo build --features sqlite --bin crewchief-maproom --release
```

This fails because the `sqlite` feature doesn't exist in Cargo.toml. The script has been failing 100% of the time, blocking E2E test execution in CI.

Additionally, error messages in the script instruct users to run the wrong command for fixture generation (with `--features sqlite` flag).

**Planning Reference**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/architecture.md`

## Acceptance Criteria
- [ ] `cargo build --features sqlite` changed to `cargo build` (line 73)
- [ ] Error message updated to remove `--features sqlite` from fixture generation command (line 61)
- [ ] Script executes successfully (builds binary without errors)
- [ ] Script syntax remains valid (shellcheck passes or no new warnings)

## Technical Requirements

### 1. Remove feature flag from binary build
**File**: `tests/e2e/test_sqlite_flow.sh`
**Line**: 73

```bash
# Before
cargo build --features sqlite --bin crewchief-maproom --release 2>/dev/null

# After
cargo build --bin crewchief-maproom --release 2>/dev/null
```

### 2. Update fixture generation error message
**File**: `tests/e2e/test_sqlite_flow.sh`
**Line**: 61

```bash
# Before
echo "Run: cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture"

# After
echo "Run: cargo test --test create_sqlite_fixture -- --ignored --nocapture"
```

### 3. Validate script syntax
After changes, validate with:
```bash
shellcheck tests/e2e/test_sqlite_flow.sh
```

Expected: No new errors/warnings introduced by changes

## Implementation Notes

**Why this change is critical**:
- E2E tests are the highest-value integration tests
- They test the full CLI workflow (build → run → verify)
- Currently 100% broken due to feature flag issue
- Fixing this unblocks CI E2E test job

**Testing approach**:
1. Make changes to script
2. Run script locally: `./tests/e2e/test_sqlite_flow.sh`
3. Verify binary builds successfully
4. Verify E2E tests execute and pass

**Build command comparison**:
- **Before**: `cargo build --features sqlite --bin crewchief-maproom --release`
  - Result: ❌ Error: feature 'sqlite' not found
- **After**: `cargo build --bin crewchief-maproom --release`
  - Result: ✅ Binary builds successfully (SQLite compiled unconditionally)

## Dependencies
- Depends on: CICLEAN-1002 (CI workflow fixture generation command fixed)
- Blocks: CICLEAN-3001 (validation depends on working E2E script)

## Risk Assessment

- **Risk**: E2E tests still fail after removing flag
  - **Mitigation**: Feature flag is the root cause; removal fixes the issue
  - **Impact**: Medium (E2E tests won't run)
  - **Probability**: Very low (flag is the problem)

- **Risk**: Script breaks with syntax errors
  - **Mitigation**: Use shellcheck validation; minimal change scope
  - **Impact**: Medium (script won't execute)
  - **Probability**: Very low (simple string replacement)

- **Risk**: Binary build fails for other reasons
  - **Mitigation**: Local testing before commit; cargo check in CI
  - **Impact**: High (blocks E2E tests)
  - **Probability**: Low (build works without features)

## Files/Packages Affected
- `tests/e2e/test_sqlite_flow.sh` - Remove feature flag from build command (lines 61, 73)
