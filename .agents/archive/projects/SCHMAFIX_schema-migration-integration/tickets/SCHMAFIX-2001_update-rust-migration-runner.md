# Ticket: SCHMAFIX-2001: Update Rust Migration Runner

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (compilation and linting tests passed)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update `crates/maproom/src/db/queries.rs` to include migrations 0018-0020 in the migrations array, enabling the Rust binary to execute the new schema migrations. Migration 0017 was added as a prerequisite in SCHMAFIX-0001.

## Background
The Rust migration runner in `queries.rs` contains a hardcoded array of migrations. After SCHMAFIX-0001 added migration 0017, the array contains migrations 0000-0017. Each migration is represented as a tuple: `(version, filename, sql_content, concurrent_flag)`. The SQL content is embedded at compile time using `include_str!` macro.

To integrate the new migrations created in SCHMAFIX-1001, we must add 3 new entries to this array (migrations 0018-0020). The Rust binary is the single source of truth for migrations and runs standalone without Node.js dependencies.

**Scope Change**: Migration 005 (complete_branchx_schema) was excluded from SCHMAFIX-1001 due to destructive TRUNCATE statement. Only 3 migrations are added, not 4.

This ticket implements **Phase 2: Rust Migration Runner Update** from the SCHMAFIX project plan.

## Acceptance Criteria
- [ ] File `crates/maproom/src/db/queries.rs` includes migration 0018 in the migrations array
- [ ] File includes migration 0019 in the migrations array
- [ ] File includes migration 0020 in the migrations array
- [ ] All 3 new migrations use `concurrent = false` (run in transactions for safety)
- [ ] Code compiles without errors (`cargo build` succeeds)
- [ ] Clippy linting passes with no warnings (`cargo clippy` succeeds)

## Technical Requirements

### Location
- File: `crates/maproom/src/db/queries.rs`
- Approximate lines: 28-140 (migrations array definition)

### Migration Array Format
The migrations array has type `Vec<(i32, &str, &str, bool)>`:
- **Field 1 (i32)**: Migration version number (18, 19, 20, 21)
- **Field 2 (&str)**: Filename for logging ("0018_add_blob_sha.sql", etc.)
- **Field 3 (&str)**: SQL content via `include_str!("./../../migrations/NNNN_name.sql")`
- **Field 4 (bool)**: Concurrent flag - MUST be `false` for transactional safety

### File Path Resolution
- Each `include_str!` path is relative to the `queries.rs` file location
- Path format: `./../../migrations/NNNN_name.sql`
- The path `./../../migrations/` resolves from `src/db/queries.rs` to `migrations/`
- Rust compiler will validate SQL file existence at compile time

### Migration Order
Migrations must be added in sequential order (after version 17):
1. Migration 18: `0018_add_blob_sha.sql`
2. Migration 19: `0019_create_code_embeddings.sql`
3. Migration 20: `0020_add_worktree_tracking.sql`

### Concurrent Flag
All 3 new migrations MUST use `concurrent = false`:
- `false` = Run in transaction (rollback on failure)
- `true` = Run outside transaction (for operations requiring no active transaction)
- Rationale: Migration 0018 was simplified in SCHMAFIX-1001 to be transaction-safe. All 3 migrations modify tables and need transaction safety.

## Implementation Notes

### Step-by-Step Process

1. **Verify SCHMAFIX-0001 completion**
   - Confirm migration 0017 is already in the migrations array
   - Migrations array should end at version 17 before this ticket starts

2. **Open the migration runner file**
   ```bash
   # Read the queries.rs file
   cat crates/maproom/src/db/queries.rs
   ```

3. **Locate the migrations vector**
   - Search for the `migrations` variable (around lines 28-140)
   - Find the last migration entry (should be version 17 from SCHMAFIX-0001)

4. **Add 3 new migration tuples**
   Insert these lines after the last existing migration (version 17):
   ```rust
   (18, "0018_add_blob_sha.sql", include_str!("./../../migrations/0018_add_blob_sha.sql"), false),
   (19, "0019_create_code_embeddings.sql", include_str!("./../../migrations/0019_create_code_embeddings.sql"), false),
   (20, "0020_add_worktree_tracking.sql", include_str!("./../../migrations/0020_add_worktree_tracking.sql"), false),
   ```

4. **Verify relative paths**
   - Confirm that `./../../migrations/` correctly resolves from `src/db/queries.rs` to `migrations/`
   - The Rust compiler will validate these paths at compile time

5. **Build and test**
   ```bash
   # Verify compilation succeeds (validates include_str! paths)
   cd crates/maproom
   cargo build

   # Ensure no linting warnings
   cargo clippy

   # Optional: Check formatting
   cargo fmt --check
   ```

### Verification Steps

**Compilation Validation**:
- `cargo build` must succeed (confirms include_str! paths are correct)
- No "file not found" errors
- No syntax errors in SQL files

**Linting Validation**:
- `cargo clippy` must pass with no warnings
- Common issues: trailing commas, unnecessary clones, unused variables

**Path Verification**:
- Migration 18: `./../../migrations/0018_add_blob_sha.sql` exists
- Migration 19: `./../../migrations/0019_create_code_embeddings.sql` exists
- Migration 20: `./../../migrations/0020_add_worktree_tracking.sql` exists
- Migration 21: `./../../migrations/0021_complete_branchx_schema.sql` exists

### Common Pitfalls

**Incorrect Path Separators**:
- ❌ `include_str!("../../migrations/0018_add_blob_sha.sql")` (wrong separator count)
- ✅ `include_str!("./../../migrations/0018_add_blob_sha.sql")` (correct)

**Wrong Concurrent Flag**:
- ❌ `(18, "0018_add_blob_sha.sql", include_str!("..."), true)` (unsafe)
- ✅ `(18, "0018_add_blob_sha.sql", include_str!("..."), false)` (transaction-safe)

**Out of Order**:
- Migrations must be sequential: 0, 1, 2, ..., 17, 18, 19, 20, 21
- Do not skip numbers or reorder

## Dependencies

### Blockers
- **SCHMAFIX-0001** (BLOCKER) - Migration 0017 must be added to queries.rs FIRST
  - Status: Must be completed before this ticket starts
  - Impact: Without migration 0017, numbering will be incorrect (array ends at 16, not 17)

- **SCHMAFIX-1001** (BLOCKER) - Migration SQL files must exist before updating the runner
  - Status: Must be completed after SCHMAFIX-0001
  - Impact: Without SQL files (0018, 0019, 0020), include_str! will fail at compile time

### External Dependencies
- Rust toolchain (cargo, rustc)
- SQL files created in SCHMAFIX-1001 (3 files: 0018, 0019, 0020)

## Risk Assessment

**Risk**: Incorrect include_str! path
- **Mitigation**: Verify relative path syntax, test compile to validate paths
- **Impact**: Compilation failure with "file not found" error
- **Likelihood**: Low (path format is consistent with existing migrations)
- **Recovery**: Fix path and recompile (5 minutes)

**Risk**: Wrong concurrent flag (true instead of false)
- **Mitigation**: All 3 migrations MUST use `false` for transaction safety
- **Impact**: Migration failures won't rollback, leaving database in inconsistent state
- **Likelihood**: Low (explicitly documented in requirements)
- **Severity**: High (data integrity risk)
- **Recovery**: Update flag, rebuild, rerun migrations

**Risk**: Migrations added out of order
- **Mitigation**: Add sequentially after version 17 (18, 19, 20)
- **Impact**: Migration runner may skip migrations or apply in wrong order
- **Likelihood**: Very low (sequential numbering is enforced)
- **Recovery**: Reorder migrations, rebuild

**Risk**: Syntax error in SQL files
- **Mitigation**: SQL files were validated in SCHMAFIX-1001
- **Impact**: Runtime migration failure
- **Likelihood**: Very low (already tested)
- **Recovery**: Fix SQL file, rebuild

**Risk**: Compilation warnings from clippy
- **Mitigation**: Run `cargo clippy` and fix any warnings
- **Impact**: Code quality issues, failed CI/CD checks
- **Likelihood**: Low (syntax is consistent with existing code)
- **Recovery**: Fix warnings and recompile (10-15 minutes)

## Files/Packages Affected

### Files to Modify
- `/workspace/crates/maproom/src/db/queries.rs` (add 4 lines to migrations array)

### Files to Reference
These files must exist (created in SCHMAFIX-1001):
- `/workspace/crates/maproom/migrations/0018_add_blob_sha.sql`
- `/workspace/crates/maproom/migrations/0019_create_code_embeddings.sql`
- `/workspace/crates/maproom/migrations/0020_add_worktree_tracking.sql`
- `/workspace/crates/maproom/migrations/0021_complete_branchx_schema.sql`

### Build Outputs
- `/workspace/crates/maproom/target/debug/maproom` (updated binary)
- Embedded SQL content in compiled binary

## Testing Strategy

### Compilation Tests
```bash
# Test 1: Verify compilation succeeds
cd /workspace/crates/maproom
cargo build
# Expected: "Finished dev [unoptimized + debuginfo] target(s)"

# Test 2: Verify no linting warnings
cargo clippy
# Expected: No warnings or errors

# Test 3: Check formatting (optional)
cargo fmt --check
# Expected: No formatting issues
```

### Manual Verification
1. **Read the modified file**
   - Verify 4 new migration entries exist
   - Verify all use `concurrent = false`
   - Verify sequential numbering (18, 19, 20, 21)

2. **Inspect compiled binary** (optional)
   ```bash
   # Verify SQL content is embedded
   strings target/debug/maproom | grep "CREATE TABLE IF NOT EXISTS code_embeddings"
   # Expected: Should find SQL from migration 19
   ```

### Integration Tests
- Actual migration execution will be tested in SCHMAFIX-1003
- This ticket focuses on compilation and code integration only

## Success Metrics

### Completion Criteria
- All 4 migrations added to `queries.rs`
- All migrations use `concurrent = false`
- Code compiles without errors
- Clippy passes with no warnings

### Quality Criteria
- Migrations in correct sequential order
- Consistent formatting with existing code
- Proper relative paths for include_str!
- No code duplication or redundancy

## Related Planning Documents

- [SCHMAFIX Plan](../planning/plan.md) - Phase 2: Rust Migration Runner Update
- [SCHMAFIX Architecture](../planning/architecture.md) - Rust Migration Runner section
- [SCHMAFIX Security Review](../planning/security-review.md) - Transaction boundaries and safety

## Estimated Effort
30 minutes - 1 hour

**Breakdown**:
- Read and understand queries.rs structure: 10 minutes
- Add 4 migration entries: 10 minutes
- Verify paths and concurrent flags: 10 minutes
- Test compilation and clippy: 10 minutes
- Documentation and review: 10 minutes

## Next Steps

After this ticket is complete:
- **SCHMAFIX-3001**: Write comprehensive migration integration tests
- **SCHMAFIX-3002**: Run migration integration tests and verify results
- **SCHMAFIX-4001**: Update CI/CD to run migration tests

## Notes

### Why Concurrent = False?
These migrations perform DDL operations (CREATE TABLE, ALTER TABLE, CREATE INDEX) that:
1. **Need atomic execution** - Either all changes succeed or all rollback
2. **May fail on duplicate schema** - Idempotent SQL needs transaction context
3. **Modify existing tables** - Requires consistent snapshot during changes

Setting `concurrent = false` ensures migrations run inside a transaction, providing:
- **Rollback on failure** - Database returns to previous state
- **Atomic operations** - No partial schema updates
- **Error recovery** - Safe to retry failed migrations

### Migration Execution Order
The migration runner executes migrations sequentially by version number:
1. Queries database for last applied migration version
2. Filters migrations array for versions > last applied
3. Executes each migration in order (18 → 19 → 20 → 21)
4. Records successful execution in migrations table
5. Stops on first failure, allowing retry from that point

This ticket ensures migrations 18-20 are available for execution in Phase 3 testing.
