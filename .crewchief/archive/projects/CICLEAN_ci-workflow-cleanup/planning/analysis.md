# Problem Analysis: CI Workflow Cleanup

## Problem Definition

The CI test workflow (`.github/workflows/test.yml`) is failing on PR #19 due to outdated configuration that references removed Cargo features and PostgreSQL backend code that no longer exists in the codebase. This blocks PR merging and wastes CI resources on checks that cannot pass.

## Specific Failures

Based on the workflow configuration and codebase analysis:

### 1. Rust SQLite Tests Job (`test-rust-sqlite`)
**Lines 186-224 in test.yml**
- **Command**: `cargo check --features sqlite` (line 208)
- **Command**: `cargo test --features sqlite -- --test-threads=1` (line 213)
- **Problem**: The `sqlite` feature flag does not exist in `crates/maproom/Cargo.toml`
- **Evidence**: Cargo.toml only has `profiling = ["puffin"]` feature (lines 131-132)

### 2. Rust PostgreSQL Tests Job (`test-rust-postgres`)
**Lines 362-401 in test.yml**
- **Command**: `cargo check --features postgres` (line 383)
- **Command**: `cargo test --features postgres -- --test-threads=1` (line 388)
- **Problem**: The `postgres` feature flag does not exist in `crates/maproom/Cargo.toml`
- **Evidence**: No PostgreSQL dependencies in Cargo.toml (only rusqlite, r2d2_sqlite)

### 3. PostgreSQL Integration Tests (TypeScript)
**Lines 234-360 in test.yml**
- **Problem**: Entire job tests PostgreSQL integration, but PostgreSQL support has been completely removed
- **Evidence**: No PostgreSQL dependencies in Cargo.toml, SQLite is the only backend

### 4. SQLite E2E Tests Job (`test-sqlite-e2e`)
**Lines 67-102 in test.yml**
- **Failure Mode**: Cascade failure - binary cannot be built without feature flags
- **Script**: `./tests/e2e/test_sqlite_flow.sh`
- **Line 73**: `cargo build --features sqlite --bin crewchief-maproom --release`
- **Problem**: This build command fails, causing the entire E2E test to fail

### 5. MCP SQLite Tests (TypeScript) - PASSING ✓
**Lines 113-184 in test.yml**
- **Line 161**: `cargo test --features sqlite --test create_sqlite_fixture -- --ignored`
- **Status**: This job is currently PASSING
- **Why**: The fixture generation command is conditional (only runs if fixture missing)
- **Problem**: This is a ticking time bomb - will fail if fixture needs regeneration

### 6. TypeScript Package Tests Job (`test-typescript`)
**Lines 409-473 in test.yml**
- **Potential Problem**: Tests may have assertions expecting PostgreSQL-related text
- **Evidence**: `packages/maproom-mcp/tests/unit/resolve-database.test.ts` has PostgreSQL rejection tests (lines 60-63, 125-134)
- **Analysis**: These tests are actually VALID - they test that PostgreSQL URLs are properly rejected (SQLite-only validation)

## Root Cause Analysis

### 1. Feature Flags Removed
**When**: During SQLite-only migration (projects SQLVEC, SQLITE, SQLFIX)
**What Happened**:
- Cargo.toml was simplified to remove `sqlite` and `postgres` feature flags
- SQLite became the default and only backend (unconditional compilation)
- CI workflow was not updated to match this change

**Evidence**:
```toml
# crates/maproom/Cargo.toml (current)
[features]
profiling = ["puffin"]  # ONLY feature

# Dependencies (SQLite only)
rusqlite = { version = "0.29.0", features = ["bundled", "chrono"] }
r2d2 = "0.8"
r2d2_sqlite = "0.22"
```

### 2. PostgreSQL Support Completely Removed
**When**: During infrastructure simplification (projects SQLINFRA, SQLITE)
**What Happened**:
- All PostgreSQL dependencies removed from Cargo.toml
- PostgreSQL-specific code removed from codebase
- CI job remained in test.yml

**Evidence**:
- No `tokio-postgres`, `sqlx`, or `deadpool-postgres` dependencies
- Comment in Cargo.toml line 44: "SQLite database (the only backend)"
- MCP tests explicitly reject PostgreSQL URLs (resolve-database.test.ts)

### 3. Workflow Documentation is Misleading
**Lines 1-23 in test.yml**
```yaml
# =============================================================================
# CI Workflow: SQLite-First Testing Strategy
# =============================================================================
#
# DATABASE BACKENDS:
#   - SQLite (Default): Zero-configuration, runs without external services
#   - PostgreSQL (Integration): For team sharing/production validation
```

**Problem**: This documentation implies PostgreSQL is still supported, but it's not.

## Impact Assessment

### Immediate Impact
- **PR #19 blocked**: Cannot merge due to failing CI checks
- **Developer confusion**: Failing checks suggest backend issues when there are none
- **CI resource waste**: 3 jobs run PostgreSQL service containers unnecessarily
- **False negatives**: Developers may think code is broken when CI fails

### Ongoing Impact
- **Every PR will fail**: Until this is fixed, all PRs will encounter these failures
- **Reduced confidence**: Developers lose trust in CI when checks fail for wrong reasons
- **Time waste**: Investigating failures that aren't real bugs
- **Documentation debt**: Workflow comments mislead about PostgreSQL support

## Constraints

### Technical Constraints
1. **SQLite is the only backend**: Cannot re-add PostgreSQL support
2. **No feature flags needed**: SQLite is compiled unconditionally
3. **Binary names unchanged**: `crewchief-maproom` binary name stays the same
4. **Existing tests must pass**: Cannot break currently passing checks

### Process Constraints
1. **Must preserve MCP SQLite tests**: Only passing check must continue to pass
2. **Cannot skip validation**: Still need to verify Rust compilation and tests
3. **Minimal disruption**: Changes should be straightforward, not architectural
4. **Documentation accuracy**: Workflow comments must reflect reality

## Success Criteria

### Primary Criteria
1. ✅ All CI checks pass on a clean PR
2. ✅ Rust compilation verified (`cargo check` without feature flags)
3. ✅ Rust tests verified (`cargo test` without feature flags)
4. ✅ E2E tests run successfully (binary builds correctly)
5. ✅ TypeScript tests pass (no outdated assertions)

### Secondary Criteria
1. ✅ Workflow documentation is accurate (no PostgreSQL references)
2. ✅ No PostgreSQL service containers or jobs remain
3. ✅ CI runs faster (fewer jobs, no external services)
4. ✅ Clear job names reflect SQLite-only reality

## Existing Patterns in Codebase

### How Rust is Built Elsewhere
**Example**: `release-cli.yml` (lines 71-98)
```yaml
- name: Build Rust binaries
  uses: ./.github/workflows/reusable-rust-build.yml
```

**Pattern**: Uses reusable workflow, no feature flags

### How Tests Are Run Elsewhere
**Example**: `test.yml` MCP SQLite tests (line 167-168)
```yaml
- name: Build MCP package
  working-directory: packages/maproom-mcp
  run: pnpm build
```

**Pattern**: Build without feature flags, test via TypeScript

### Similar Cleanup Projects
**Archive**: `.crewchief/archive/projects/SQLINFRA_infrastructure-simplification/`
- Removed PostgreSQL service containers
- Simplified database configuration
- Updated documentation to reflect SQLite-only

**Archive**: `.crewchief/archive/projects/SQLFIX_sqlite-backend-fixes/`
- Fixed SQLite compilation issues
- Updated CI to use SQLite features
- Added SQLite-specific tests

**Pattern**: Both projects simplified infrastructure toward SQLite-only

## Gaps in Understanding

### Clarified
- ✅ Cargo.toml feature flags confirmed non-existent
- ✅ PostgreSQL completely removed from dependencies
- ✅ MCP tests only fail on fixture regeneration (conditional)
- ✅ E2E test script explicitly uses `--features sqlite` (line 73)

### No Major Unknowns
All necessary information is available in:
- `.github/workflows/test.yml` (CI configuration)
- `crates/maproom/Cargo.toml` (Rust dependencies and features)
- `tests/e2e/test_sqlite_flow.sh` (E2E test script)
- `packages/maproom-mcp/tests/unit/resolve-database.test.ts` (TypeScript tests)

## Related Work

### Completed Projects
1. **SQLVEC** (sqlite-vec-backend): Migrated to SQLite with sqlite-vec extension
2. **SQLITE** (full-sqlite-implementation): Implemented full SQLite backend
3. **SQLFIX** (sqlite-backend-fixes): Fixed SQLite compilation issues
4. **SQLINFRA** (infrastructure-simplification): Removed PostgreSQL infrastructure

### Current State
The codebase has been fully migrated to SQLite-only, but the CI workflow was not updated to reflect this reality. This project completes the migration by cleaning up the CI configuration.

## Assumptions

1. **SQLite-only is final**: No plans to re-add PostgreSQL support
2. **Feature flags unnecessary**: SQLite compilation is unconditional and correct
3. **Binary builds are correct**: The issue is only CI configuration, not code
4. **TypeScript tests are valid**: PostgreSQL rejection tests are intentional (SQLite-only validation)
