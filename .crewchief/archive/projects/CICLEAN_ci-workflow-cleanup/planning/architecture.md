# Solution Architecture: CI Workflow Cleanup

## Overview

Remove outdated Cargo feature flags and PostgreSQL references from CI workflow, aligning it with the SQLite-only reality of the codebase. This is a configuration-only change with no code modifications.

## Design Principles

1. **MVP-Focused**: Minimal changes to fix the immediate problem
2. **Pragmatic**: Remove what doesn't work, don't add complexity
3. **Consistent**: Match CI configuration to actual codebase capabilities
4. **Safe**: Preserve all currently passing checks

## High-Level Architecture

### Before (Current State)
```
CI Workflow (test.yml)
├── SQLite E2E Tests           ❌ FAILS (--features sqlite doesn't exist)
├── MCP SQLite Tests           ✅ PASSES (conditional fixture generation)
├── Rust SQLite Tests          ❌ FAILS (--features sqlite doesn't exist)
├── Rust PostgreSQL Tests      ❌ FAILS (--features postgres doesn't exist)
├── PostgreSQL Integration     ❌ FAILS (backend removed)
└── TypeScript Package Tests   ✅ PASSES (valid rejection tests)
```

### After (Target State)
```
CI Workflow (test.yml)
├── SQLite E2E Tests           ✅ PASSES (cargo build without features)
├── MCP SQLite Tests           ✅ PASSES (unchanged)
├── Rust Tests                 ✅ PASSES (cargo test without features)
└── TypeScript Package Tests   ✅ PASSES (unchanged)
```

## Key Design Decisions

### Decision 1: Remove Feature Flags from All Commands
**Rationale**: Feature flags don't exist, SQLite is compiled unconditionally
**Impact**:
- Simplifies CI configuration
- Matches actual Cargo.toml structure
- Aligns with release builds (which don't use features)

**Changes**:
```yaml
# Before
cargo check --features sqlite
cargo test --features sqlite -- --test-threads=1

# After
cargo check
cargo test -- --test-threads=1
```

### Decision 2: Remove Entire PostgreSQL Test Jobs
**Rationale**: PostgreSQL support completely removed, job cannot pass
**Impact**:
- Reduces CI time (no service container startup)
- Eliminates misleading failures
- Simplifies workflow maintenance

**Jobs to Remove**:
- `test-postgres` (lines 234-360)
- `test-rust-postgres` (lines 362-401)

### Decision 3: Merge Rust SQLite Tests into Single "Rust Tests" Job
**Rationale**: No need for separate jobs when there's only one backend
**Impact**:
- Clearer job naming ("Rust Tests" vs "Rust SQLite Tests")
- Simpler workflow structure
- Matches actual codebase reality

**New Job Name**: `test-rust` (renamed from `test-rust-sqlite`)

### Decision 4: Update Workflow Documentation
**Rationale**: Misleading comments cause confusion
**Impact**:
- Clear communication of SQLite-only approach
- Prevents future attempts to add PostgreSQL
- Documents intentional architecture decision

**Changes**:
```yaml
# Before
# DATABASE BACKENDS:
#   - SQLite (Default): Zero-configuration, runs without external services
#   - PostgreSQL (Integration): For team sharing/production validation

# After
# DATABASE BACKEND:
#   - SQLite (Only): Zero-configuration, runs without external services
#   - No PostgreSQL support (intentionally removed for simplicity)
```

### Decision 5: Fix E2E Test Script Feature Flag
**Rationale**: Script uses `--features sqlite` which doesn't exist
**Impact**:
- E2E tests will pass
- Binary builds correctly
- Matches actual build process

**Change**:
```bash
# Before (tests/e2e/test_sqlite_flow.sh line 73)
cargo build --features sqlite --bin crewchief-maproom --release

# After
cargo build --bin crewchief-maproom --release
```

### Decision 6: Keep TypeScript PostgreSQL Rejection Tests
**Rationale**: These tests verify SQLite-only behavior (intentional validation)
**Impact**: No changes to TypeScript tests
**Evidence**: Tests in `packages/maproom-mcp/tests/unit/resolve-database.test.ts` verify that PostgreSQL URLs are properly rejected with error message "Only SQLite is supported"

## Component Changes

### 1. CI Workflow (`.github/workflows/test.yml`)

#### Changes to Existing Jobs

**A. test-sqlite-e2e (lines 67-102)**
- No changes needed (binary will build after E2E script fix)
- Job summary remains accurate

**B. test-mcp-sqlite (lines 113-184)**
- **Line 161**: Update fixture generation command
  ```yaml
  # Before
  cargo test --features sqlite --test create_sqlite_fixture -- --ignored

  # After
  cargo test --test create_sqlite_fixture -- --ignored
  ```
- Keep conditional check (only generate if missing)
- Job summary remains accurate

**C. test-rust-sqlite (lines 186-224) → test-rust**
- **Rename job**: `test-rust-sqlite` → `test-rust`
- **Line 189**: Update job name display: "Rust SQLite Tests" → "Rust Tests"
- **Line 208**: Remove feature flag: `cargo check --features sqlite` → `cargo check`
- **Line 213**: Remove feature flag: `cargo test --features sqlite -- --test-threads=1` → `cargo test -- --test-threads=1`
- **Lines 216-224**: Update job summary
  ```yaml
  echo "## 🦀 Rust Tests" >> $GITHUB_STEP_SUMMARY
  echo "**Backend:** SQLite (only)" >> $GITHUB_STEP_SUMMARY
  echo "Tests Rust \`maproom\` crate (SQLite is the only backend)." >> $GITHUB_STEP_SUMMARY
  ```

**D. test-typescript (lines 409-473)**
- No changes needed
- PostgreSQL rejection tests are valid (test SQLite-only behavior)

#### Jobs to Remove

**E. test-postgres (lines 234-360)**
- Delete entire job
- Removes PostgreSQL service container configuration
- Removes TypeScript integration tests for PostgreSQL

**F. test-rust-postgres (lines 362-401)**
- Delete entire job
- Removes PostgreSQL feature compilation check

#### Documentation Updates

**Header (lines 1-23)**
```yaml
# Before
# =============================================================================
# CI Workflow: SQLite-First Testing Strategy
# =============================================================================
#
# DATABASE BACKENDS:
#   - SQLite (Default): Zero-configuration, runs without external services
#   - PostgreSQL (Integration): For team sharing/production validation
#
# JOB ORGANIZATION:
#   1. SQLite Tests (Primary) - Fast, no dependencies
#      - test-sqlite-e2e: CLI end-to-end tests
#      - test-mcp-sqlite: TypeScript MCP server tests
#      - test-rust (sqlite): Rust library tests
#
#   2. PostgreSQL Tests (Integration) - Requires service container
#      - test-postgres: TypeScript PostgreSQL integration
#      - test-rust (postgres): Rust PostgreSQL feature tests

# After
# =============================================================================
# CI Workflow: SQLite-Only Testing Strategy
# =============================================================================
#
# DATABASE BACKEND:
#   - SQLite (Only): Zero-configuration, runs without external services
#   - No PostgreSQL support (intentionally removed for simplicity)
#
# JOB ORGANIZATION:
#   - test-sqlite-e2e: CLI end-to-end tests
#   - test-mcp-sqlite: TypeScript MCP server tests
#   - test-rust: Rust library tests
#   - test-typescript: TypeScript package tests (CLI, VSCode, daemon-client)
```

**Section Dividers (lines 63-65, 226-231)**
- Remove "SQLITE TESTS (Primary)" section divider (line 63-65)
- Remove "POSTGRESQL TESTS (Integration)" section divider (lines 226-231)
- Replace with simple "TEST JOBS" divider

### 2. E2E Test Script (`tests/e2e/test_sqlite_flow.sh`)

**Line 73**:
```bash
# Before
cargo build --features sqlite --bin crewchief-maproom --release 2>/dev/null

# After
cargo build --bin crewchief-maproom --release 2>/dev/null
```

**Line 61** (error message):
```bash
# Before
echo "Run: cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture"

# After
echo "Run: cargo test --test create_sqlite_fixture -- --ignored --nocapture"
```

### 3. SQLite Test Helper (`packages/maproom-mcp/tests/helpers/sqlite.ts`)

**Lines 49, 92** (error messages):
```typescript
// Before
`Run: cargo test --features sqlite --test create_sqlite_fixture -- --ignored`

// After
`Run: cargo test --test create_sqlite_fixture -- --ignored`
```

### 4. Documentation (`docs/testing/SQLITE_INTEGRATION_TESTS.md`)

**Lines 62, 148** (fixture generation instructions):
```bash
# Before
cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture

# After
cargo test --test create_sqlite_fixture -- --ignored --nocapture
```

## Technology Choices

### No New Technologies
This is purely a configuration cleanup:
- Same GitHub Actions runners
- Same Rust toolchain
- Same Node.js/pnpm versions
- Same test frameworks (cargo test, vitest)

### Removed Technologies
- PostgreSQL service containers (pgvector/pgvector:pg16)
- psql CLI tool (used for schema verification)
- PostgreSQL-specific environment variables

## Data Flow

### Current (Broken) Flow
```
GitHub PR trigger
  ↓
test-rust-sqlite job starts
  ↓
cargo check --features sqlite
  ↓
❌ ERROR: feature 'sqlite' not found
  ↓
Job fails, PR blocked
```

### Fixed Flow
```
GitHub PR trigger
  ↓
test-rust job starts
  ↓
cargo check
  ↓
✅ Compiles successfully
  ↓
cargo test -- --test-threads=1
  ↓
✅ Tests pass
  ↓
Job succeeds, PR proceeds
```

## Integration Points

### 1. GitHub Actions → Cargo
- **Current**: Passes `--features sqlite|postgres` flags
- **Fixed**: No feature flags passed
- **Integration**: Direct cargo invocation in workflow YAML

### 2. Cargo → Rust Codebase
- **Current**: Expects features to exist in Cargo.toml
- **Fixed**: Compiles unconditionally (SQLite always enabled)
- **Integration**: Standard Cargo build system

### 3. E2E Script → Binary
- **Current**: Builds with `--features sqlite` flag
- **Fixed**: Builds without feature flags
- **Integration**: Shell script invokes cargo build

### 4. MCP Tests → Fixture
- **Current**: Generates fixture with `--features sqlite` if missing
- **Fixed**: Generates fixture without features
- **Integration**: Conditional check in workflow YAML

## Reusable Components

All changes leverage existing infrastructure:
- GitHub Actions standard workflows
- Cargo standard build system
- Shell scripts (bash)
- Existing test frameworks (cargo test, vitest)

No new components or abstractions needed.

## Performance Impact

### CI Time Reduction
- **Before**: ~15 minutes (includes PostgreSQL container startup)
- **After**: ~8-10 minutes (no external services)
- **Savings**: ~30-40% faster CI runs

### Resource Reduction
- **Removed**: 2 PostgreSQL service containers per PR
- **Removed**: 2 full job executions per PR
- **Impact**: Lower GitHub Actions usage costs
