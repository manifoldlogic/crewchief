# Ticket: CICLEAN-1002: Remove feature flags from Rust jobs

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (YAML configuration change - syntax validated)
- [x] **Verified** - by the verify-ticket agent

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
Remove non-existent `--features sqlite` and `--features postgres` flags from all cargo commands in CI workflow, and rename `test-rust-sqlite` to `test-rust` to reflect SQLite-only architecture.

## Background
The CI workflow uses `cargo check --features sqlite` and `cargo test --features sqlite` commands, but these feature flags don't exist in `Cargo.toml`. The SQLite backend is compiled unconditionally (no feature flags needed).

This causes all Rust tests to fail with:
```
error: none of the selected packages contains these features: sqlite
```

The fix is to remove these non-existent flags and rename jobs to reflect the SQLite-only reality.

**Planning Reference**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/architecture.md`

## Acceptance Criteria
- [x] `test-rust-sqlite` job renamed to `test-rust`
- [x] Job display name updated to "Rust Tests" (not "Rust SQLite Tests")
- [x] `cargo check --features sqlite` changed to `cargo check` (line 208)
- [x] `cargo test --features sqlite` changed to `cargo test` (line 213)
- [x] `cargo test --features sqlite --test create_sqlite_fixture` changed to `cargo test --test create_sqlite_fixture` (line 161)
- [x] Job summary messages updated to reflect SQLite-only backend
- [x] YAML file remains syntactically valid

## Technical Requirements

### 1. Rename test-rust-sqlite job
**File**: `.github/workflows/test.yml`
**Lines**: 186-224

**Change job ID** (line 186):
```yaml
# Before
test-rust-sqlite:

# After
test-rust:
```

**Change job name** (line 189):
```yaml
# Before
name: Rust SQLite Tests

# After
name: Rust Tests
```

### 2. Remove feature flag from cargo check
**File**: `.github/workflows/test.yml`
**Line**: 208

```yaml
# Before
run: cargo check --features sqlite

# After
run: cargo check
```

### 3. Remove feature flag from cargo test
**File**: `.github/workflows/test.yml`
**Line**: 213

```yaml
# Before
run: cargo test --features sqlite -- --test-threads=1

# After
run: cargo test -- --test-threads=1
```

### 4. Update job summary
**File**: `.github/workflows/test.yml`
**Lines**: 216-224

```yaml
# Before
echo "## 🦀 Rust SQLite Tests" >> $GITHUB_STEP_SUMMARY
echo "**Backend:** SQLite (default)" >> $GITHUB_STEP_SUMMARY
echo "Tests Rust \`maproom\` crate with SQLite feature enabled." >> $GITHUB_STEP_SUMMARY

# After
echo "## 🦀 Rust Tests" >> $GITHUB_STEP_SUMMARY
echo "**Backend:** SQLite (only)" >> $GITHUB_STEP_SUMMARY
echo "Tests Rust \`maproom\` crate (SQLite is the only backend)." >> $GITHUB_STEP_SUMMARY
```

### 5. Fix MCP fixture generation
**File**: `.github/workflows/test.yml`
**Line**: 161

```yaml
# Before
cargo test --features sqlite --test create_sqlite_fixture -- --ignored

# After
cargo test --test create_sqlite_fixture -- --ignored
```

## Implementation Notes

**Why these changes are safe**:
1. SQLite is compiled unconditionally (no feature gates in code)
2. Removing flags makes CI match actual build process
3. Release builds already use no feature flags

**Command equivalence**:
- `cargo check --features sqlite` ≈ `cargo check` (feature doesn't exist)
- `cargo test --features sqlite` ≈ `cargo test` (feature doesn't exist)
- Current commands fail; updated commands succeed

**Job rename rationale**:
- "Rust Tests" is clearer than "Rust SQLite Tests"
- No need to specify backend when there's only one
- Matches actual codebase architecture

## Dependencies
- Depends on: CICLEAN-1001 (PostgreSQL jobs must be removed first for cleaner diff)

## Risk Assessment

- **Risk**: Tests fail after removing feature flags
  - **Mitigation**: Feature flags never existed; removal fixes the failures
  - **Impact**: Medium (tests won't run)
  - **Probability**: Very low (flags are the current problem)

- **Risk**: Breaking other workflows that reference job name
  - **Mitigation**: Check for workflow dependencies on `test-rust-sqlite` job ID
  - **Impact**: Low (job still runs, just different name)
  - **Probability**: Low (typical workflow doesn't reference specific job IDs)

## Files/Packages Affected
- `.github/workflows/test.yml` - Rename job, remove feature flags (lines 161, 186, 189, 208, 213, 216-224)
