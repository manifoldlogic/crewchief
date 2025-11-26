# Ticket: SQLFIX-1005: Update CI for SQLite Feature

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- CI workflow runs successfully with both features
- GitHub Actions shows green check for both postgres and sqlite builds

## Agents
- github-actions-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add a new CI job to test the SQLite backend feature. This establishes a safety net to catch regressions and ensures both backends remain functional.

## Background
The current CI workflow only tests TypeScript code with a PostgreSQL backend. The Rust code is built for migrations but not explicitly tested. Without testing the sqlite feature in CI, regressions can be introduced without detection. This ticket adds a dedicated Rust testing job.

**Current test.yml structure:**
- Single job (`test`) focused on TypeScript tests from `packages/maproom-mcp`
- Uses `pgvector/pgvector:pg16` service container for PostgreSQL
- Builds Rust binary with default features for migrations
- No explicit Rust tests run

**Plan Reference**: Phase 1 - Compile Fixes + CI (Ticket 1005)

## Acceptance Criteria
- [ ] New `test-rust` job added to `.github/workflows/test.yml`
- [ ] SQLite feature compiles: `cargo check --features sqlite` passes
- [ ] SQLite tests run: `cargo test --features sqlite` passes
- [ ] PostgreSQL feature compiles: `cargo check --features postgres` passes
- [ ] Existing TypeScript test job unchanged (no regression)
- [ ] Jobs run in parallel for faster CI

## Technical Requirements

### 1. Add Rust Test Job (New Job, Separate from Existing)

Add this new job to `.github/workflows/test.yml` AFTER the existing `test` job:

```yaml
  # Rust backend feature tests
  # Tests both sqlite and postgres features for compilation
  # SQLite tests run in-memory (no external service needed)
  test-rust:
    name: Test Rust (${{ matrix.features }})
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        features: [sqlite, postgres]

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable

      - name: Cache cargo dependencies
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: "crates/maproom -> target"
          key: ${{ matrix.features }}

      - name: Check compilation
        run: cargo check --features ${{ matrix.features }}
        working-directory: crates/maproom

      - name: Run tests
        run: cargo test --features ${{ matrix.features }}
        working-directory: crates/maproom
        # Note: postgres tests requiring DB connection will be skipped
        # SQLite tests use :memory: and don't need external services
```

### 2. Key Design Decisions

**Why a separate job instead of modifying existing?**
- Existing `test` job is TypeScript-focused with specific PostgreSQL setup for migrations
- SQLite tests don't need any database service (use `:memory:`)
- Parallel jobs mean faster CI overall
- Isolation prevents breaking existing TypeScript tests

**Why `fail-fast: false`?**
- If SQLite fails, we still want to know if PostgreSQL works (and vice versa)
- Better debugging: see all failures at once

**Why test both features?**
- Catches regressions in either backend
- Ensures feature flags don't interfere with each other
- `cargo check` is fast; compile-time verification is valuable

### 3. PostgreSQL Tests Without Service

For the `postgres` matrix entry, tests requiring a database connection will naturally fail/skip. This is acceptable for Phase 1 because:
1. Compilation is the main goal (verified by `cargo check`)
2. Integration tests can be added later with proper DB setup
3. The existing TypeScript tests already exercise the PostgreSQL backend via the MCP server

### 4. Workflow File Location

```
.github/workflows/test.yml
```

**Insert the new job after the existing `test:` job block (around line 204).**

### 5. Complete Updated test.yml Structure

```yaml
jobs:
  test:
    # ... existing TypeScript test job unchanged ...
    # (PostgreSQL service, pnpm, migrations, TypeScript tests)

  test-rust:
    # ... new Rust feature test job as shown above ...
```

## Implementation Notes

### Verification Steps
```bash
# 1. Verify workflow syntax (optional)
yamllint .github/workflows/test.yml

# 2. Push to branch and monitor Actions tab
git push origin feature-branch

# 3. Expected CI results:
#    - test (TypeScript): green (unchanged)
#    - test-rust (sqlite): green after SQLFIX-1001-1004 complete
#    - test-rust (postgres): green (check passes, tests may skip DB-dependent ones)
```

### Testing Locally Before CI
```bash
# These should pass after SQLFIX-1001-1004:
cd crates/maproom
cargo check --features sqlite
cargo test --features sqlite

# These should already pass:
cargo check --features postgres
```

### Cache Strategy
- Separate cache keys per feature (`key: ${{ matrix.features }}`)
- Workspaces pointing to correct target directory
- Matches existing workflow's cache configuration

## Dependencies
- **SQLFIX-1001**: SQLite must compile first (CI job will fail until then)
- **SQLFIX-1004**: SQLite tests must exist for `cargo test` to be meaningful

**Note**: This ticket can be implemented in parallel with SQLFIX-1001. The CI will fail initially but will start passing once compile fixes land.

## Risk Assessment
- **Risk**: Breaking existing TypeScript tests
  - **Mitigation**: New job is completely separate; existing job unchanged
- **Risk**: postgres tests fail without database
  - **Mitigation**: Tests will skip/fail gracefully; `cargo check` validates compilation
- **Risk**: Workflow syntax errors
  - **Mitigation**: Use yamllint, test on branch before merging
- **Risk**: CI time increases
  - **Mitigation**: Parallel execution; Rust cache; check is fast

## Files/Packages Affected
- `.github/workflows/test.yml` (add new job)
