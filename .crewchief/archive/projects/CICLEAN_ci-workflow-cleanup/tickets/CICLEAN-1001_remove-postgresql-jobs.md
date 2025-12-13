# Ticket: CICLEAN-1001: Remove PostgreSQL jobs from CI workflow

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (workflow configuration change, validated with yamllint)
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
Remove PostgreSQL test jobs from `.github/workflows/test.yml` that reference non-existent PostgreSQL backend and service containers that can never pass.

## Background
The CI workflow still contains two PostgreSQL-specific test jobs (`test-postgres` and `test-rust-postgres`) that attempt to test PostgreSQL support. However, PostgreSQL support was completely removed from the codebase in favor of SQLite-only architecture. These jobs fail 100% of the time because:

1. PostgreSQL backend code doesn't exist
2. The `--features postgres` flag doesn't exist in Cargo.toml
3. Service containers cannot connect to non-existent backend

This creates misleading CI failures and wastes GitHub Actions resources (~5-7 minutes per job per PR).

**Planning Reference**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/plan.md`

## Acceptance Criteria
- [x] `test-postgres` job removed (lines 234-360 in test.yml)
- [x] `test-rust-postgres` job removed (lines 362-401 in test.yml)
- [x] YAML file remains syntactically valid after changes
- [x] No references to PostgreSQL jobs remain in workflow file
- [x] Section divider comments updated to remove "PostgreSQL Tests" section

## Technical Requirements

### 1. Remove test-postgres job
**File**: `.github/workflows/test.yml`
**Lines**: 234-360

Delete the entire `test-postgres` job including:
- Job definition and name
- PostgreSQL service container configuration (`services: postgres:`)
- Database URL environment variable setup
- TypeScript PostgreSQL integration tests
- Job summary output

### 2. Remove test-rust-postgres job
**File**: `.github/workflows/test.yml`
**Lines**: 362-401

Delete the entire `test-rust-postgres` job including:
- Job definition and name
- PostgreSQL service container configuration
- Cargo check with `--features postgres` flag
- Cargo test with `--features postgres` flag
- Job summary output

### 3. Update section dividers
**File**: `.github/workflows/test.yml`
**Lines**: 226-231

Remove the section divider:
```yaml
# =============================================================================
# POSTGRESQL TESTS (Integration)
# =============================================================================
# Requires PostgreSQL service container. Tests integration workflows.
```

### 4. Validate YAML syntax
After deletions, validate YAML syntax:
```bash
yamllint .github/workflows/test.yml
```

## Implementation Notes

**Deletion strategy**:
1. Identify exact line ranges for each job (including all nested configuration)
2. Delete `test-postgres` job first (lines 234-360)
3. Delete `test-rust-postgres` job second (lines 362-401 - note line numbers shift after first deletion)
4. Remove PostgreSQL section divider (lines 226-231)
5. Validate YAML syntax

**What NOT to remove**:
- TypeScript PostgreSQL rejection tests in `test-typescript` job (these test SQLite-only validation logic)
- Any SQLite-related jobs or configuration
- The `test-rust-sqlite` job (this will be renamed in CICLEAN-1002)

**Expected impact**:
- CI run time reduced by ~5-10 minutes per PR
- Failure rate drops from 66% to expected 0%
- Clearer job organization (no misleading PostgreSQL jobs)

## Dependencies
None (first ticket in Phase 1)

## Risk Assessment

- **Risk**: YAML syntax error breaks CI workflow
  - **Mitigation**: Use yamllint validation before commit; test in separate branch
  - **Impact**: High (blocks all PRs)
  - **Probability**: Low (straightforward deletion)

- **Risk**: Accidentally remove SQLite jobs
  - **Mitigation**: Carefully verify line numbers; only delete jobs with "postgres" in name
  - **Impact**: High (removes valid tests)
  - **Probability**: Very low (clear naming differences)

## Files/Packages Affected
- `.github/workflows/test.yml` - Remove PostgreSQL jobs (lines 234-360, 362-401, 226-231)
