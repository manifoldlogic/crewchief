# Ticket: SCHMAFIX-0001: Add Missing Migration 0017 to Rust Runner

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - migration execution verified
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
Add migration 0017 (`fix_index_size_limits.sql`) to the Rust migration runner in `crates/maproom/src/db/queries.rs`. This migration exists in the filesystem but was never added to the migrations array, creating a numbering gap that blocks SCHMAFIX execution.

## Background
During SCHMAFIX ticket review, we discovered that migration file `crates/maproom/migrations/0017_fix_index_size_limits.sql` exists (47 lines, fixes index size limit errors) but is NOT included in the migration runner. The migrations array in `queries.rs` ends at version 16.

This creates a critical problem: SCHMAFIX wants to add migrations 0018-0020, but there's a gap at 0017. We must add migration 0017 FIRST to maintain sequential migration numbering.

Migration 0017 addresses PostgreSQL index size limit errors by implementing a two-index strategy:
1. Drops problematic covering index `idx_chunks_search_covering`
2. Creates partial covering index for small previews (95% of data)
3. Creates basic fallback index for large previews (100% of data)

The migration uses `CREATE INDEX CONCURRENTLY`, requiring special handling outside transaction context.

**This is Phase 0 (prerequisites)** - it must be completed before any Phase 1 ticket can begin.

**Planning Reference**: `.crewchief/projects/SCHMAFIX_schema-migration-integration/planning/tickets-review-report.md` (CRITICAL-2: Missing Migration 0017)

## Acceptance Criteria
- [ ] Migration 0017 added to migrations array in `crates/maproom/src/db/queries.rs`
- [ ] Migration 0017 positioned after version 16 in the array
- [ ] Migration 0017 set to `use_concurrent_handler = true` (contains CONCURRENT operations)
- [ ] Relative path `./../../migrations/0017_fix_index_size_limits.sql` is correct
- [ ] Code compiles without errors: `cargo build` succeeds
- [ ] Migration 0017 runs successfully on test database
- [ ] After running migrations, `SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1` returns 17

## Technical Requirements

### File to Modify
- **Location**: `crates/maproom/src/db/queries.rs`
- **Line**: Around line 139 (after migration 16, before closing `];`)

### Array Entry Format
```rust
(
    17,
    "0017_fix_index_size_limits.sql",
    include_str!("./../../migrations/0017_fix_index_size_limits.sql"),
    true,  // use_concurrent_handler = true
),
```

### Why concurrent = true
Migration 0017 contains these CONCURRENT operations:
- Line 19: `DROP INDEX IF EXISTS maproom.idx_chunks_search_covering;`
- Line 27: `CREATE INDEX CONCURRENTLY idx_chunks_search_small_preview ...`
- Line 37: `CREATE INDEX CONCURRENTLY idx_chunks_search_basic ...`

`CREATE INDEX CONCURRENTLY` cannot run inside a transaction, so we must use the concurrent handler which executes statements one-by-one outside transaction context.

### Migration Purpose
From the migration file header:
- **Problem**: `idx_chunks_search_covering` fails when preview > 2704 bytes
- **Solution**: Two-index strategy (partial covering + basic fallback)
- Partial index handles 95%+ of data (small previews)
- Basic index handles 100% of data (all preview sizes)

## Implementation Notes

### Step-by-Step Process

1. **Open the file**:
   ```bash
   # File: crates/maproom/src/db/queries.rs
   # Location: around line 139
   ```

2. **Locate insertion point**:
   - Find migration 16 (lines 134-139)
   - Insert new entry after line 139, before closing `];` on line 140

3. **Add migration 0017 entry**:
   ```rust
   (
       16,
       "0016_add_updated_at_to_chunks.sql",
       include_str!("./../../migrations/0016_add_updated_at_to_chunks.sql"),
       false,
   ),
   // ADD THIS:
   (
       17,
       "0017_fix_index_size_limits.sql",
       include_str!("./../../migrations/0017_fix_index_size_limits.sql"),
       true,  // CONCURRENT index operations
   ),
   ```

4. **Verify relative path**:
   - Check that path matches other migrations: `./../../migrations/`
   - Confirm file exists at this path

5. **Compile and test**:
   ```bash
   # Build
   cargo build --bin crewchief-maproom

   # Run migration (test database)
   cargo run --bin crewchief-maproom -- db

   # Verify migration applied
   psql $DATABASE_URL -c "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1;"
   # Expected: 17
   ```

### Migration Execution Behavior

When migration 0017 runs:
1. **Statement 1** (in transaction): `SET statement_timeout = '10min';`
2. **Statement 2** (in transaction): `BEGIN;`
3. **Statement 3** (in transaction): `DROP INDEX IF EXISTS maproom.idx_chunks_search_covering;`
4. **Statement 4** (in transaction): `COMMIT;`
5. **Statement 5** (NO transaction): `CREATE INDEX CONCURRENTLY idx_chunks_search_small_preview ...`
6. **Statement 6** (NO transaction): `CREATE INDEX CONCURRENTLY idx_chunks_search_basic ...`
7. **Statement 7** (in transaction): `ANALYZE maproom.chunks;`
8. **Statement 8** (in transaction): `RESET statement_timeout;`

The concurrent handler in `queries.rs` splits the SQL on semicolons and executes each statement individually, which allows CONCURRENT operations to work correctly.

## Dependencies
- **None** - This is the first ticket and has no prerequisites
- Migration file already exists at `crates/maproom/migrations/0017_fix_index_size_limits.sql`

## Blocks
- **SCHMAFIX-1001**: Copy Migration SQL Files (cannot proceed until 0017 is in runner)
- **ALL other SCHMAFIX tickets**: Phase 1-6 all depend on Phase 0 completion

## Risk Assessment

### Risk 1: Migration Fails on Existing Database
- **Description**: Migration 0017 might fail if indexes already exist or data incompatible
- **Likelihood**: Low (migration has DROP IF EXISTS safety)
- **Impact**: Medium (blocks SCHMAFIX)
- **Mitigation**:
  - Test on development database first
  - Migration uses `IF EXISTS` for safe cleanup
  - CONCURRENT operations are non-blocking

### Risk 2: CONCURRENT Index Creation Slow
- **Description**: Creating indexes on large tables takes time
- **Likelihood**: High (expected behavior)
- **Impact**: Low (one-time operation, non-blocking)
- **Mitigation**:
  - This is expected and acceptable
  - CONCURRENT allows other operations during creation
  - Statement timeout set to 10 minutes

### Risk 3: Path Wrong, Compilation Fails
- **Description**: Relative path to migration file incorrect
- **Likelihood**: Very Low (path matches pattern of other migrations)
- **Impact**: Low (caught at compile time)
- **Mitigation**:
  - Verify path matches other migrations: `./../../migrations/`
  - `cargo build` will catch incorrect paths immediately
  - No runtime risk

### Risk 4: Migration Already Applied Manually
- **Description**: Someone might have run migration 0017 manually on database
- **Likelihood**: Low (not in runner, unlikely to be run manually)
- **Impact**: None (migration runner checks `schema_migrations` table)
- **Mitigation**:
  - Migration runner skips already-applied migrations
  - Safe to run even if version 17 already exists in database

## Files/Packages Affected

### Files to Modify
- `/workspace/crates/maproom/src/db/queries.rs` (1 line addition to migrations array)

### Files to Reference
- `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql` (the migration SQL)

### No Changes Needed
- Migration file already exists and is correct
- No schema changes needed
- No test updates needed (testing the migration runner itself)

## Testing Strategy

### Compilation Test
```bash
cargo build --bin crewchief-maproom
# Expected: Success (no compilation errors)
```

### Migration Execution Test
```bash
# Run migration
cargo run --bin crewchief-maproom -- db

# Expected output:
# ✅ Migration tracking table exists
# ⏭️  Skipping migration 1-16 (already applied)
# 🔄 Applying migration 17: 0017_fix_index_size_limits.sql
# ✅ Migration 17 applied successfully
```

### Verification Query
```bash
psql $DATABASE_URL -c "SELECT version, filename FROM schema_migrations WHERE version = 17;"
# Expected:
# version |            filename
#---------+-------------------------------
#      17 | 0017_fix_index_size_limits.sql
```

### Index Verification
```bash
psql $DATABASE_URL -c "\d maproom.chunks" | grep idx_chunks_search
# Expected:
# "idx_chunks_search_basic" btree (file_id, start_line, kind)
# "idx_chunks_search_small_preview" btree (file_id, kind, start_line) INCLUDE (symbol_name, preview) WHERE length(preview::text) <= 2000
```

## Success Metrics

### Immediate Success
- [ ] Code compiles: `cargo build` exits 0
- [ ] Migration runs: No errors during `cargo run -- db`
- [ ] Version recorded: `schema_migrations` table contains version 17
- [ ] Indexes created: Both new indexes exist in database

### Downstream Success
- [ ] SCHMAFIX-1001 can proceed (no migration numbering conflict)
- [ ] Future migrations (0018-0020) can be added sequentially
- [ ] No gaps in migration sequence

## Notes

### Why This Ticket is Critical
This ticket fixes a **broken invariant**: the migration runner MUST contain all migrations in sequential order. Having migration file 0017 on disk but not in the runner creates:
1. **Numbering conflict**: Can't add 0018 when 0017 is missing
2. **State mismatch**: Filesystem and runner disagree on migration history
3. **Blocks SCHMAFIX**: Phase 1 cannot start until this is resolved

### Historical Context
Migration 0017 was created as part of the IDXSIZE project (Index Size Limits) to fix PostgreSQL errors when `preview` column exceeded 2704 bytes. The migration was written and committed to the filesystem but was never added to the Rust migration runner, creating this gap.

**References**:
- Original project: `.crewchief/projects/IDXSIZE_index-size-limits/` (likely archived)
- Planning docs: Referenced in migration file comments
- Issue: Index size limit errors on large preview data

### Phase 0 Designation
This ticket is designated **Phase 0** (prerequisites) because it must be completed before any other SCHMAFIX work can begin. The phase numbering is:
- **Phase 0**: SCHMAFIX-0001 (this ticket) - fix migration gap
- **Phase 1**: SCHMAFIX-1001 to 1003 - copy new migrations
- **Phase 2**: SCHMAFIX-2001 to 2003 - update runner
- **Phase 3+**: Testing and validation

### Estimated Effort
**30 minutes** - This is a simple single-line addition with verification:
- 5 min: Open file, add entry
- 5 min: Compile and fix any syntax issues
- 10 min: Run migration on test database
- 10 min: Verify indexes created, test queries

### Related Documentation
- SCHMAFIX planning: `.crewchief/projects/SCHMAFIX_schema-migration-integration/planning/`
- Ticket review: `.crewchief/projects/SCHMAFIX_schema-migration-integration/planning/tickets-review-report.md`
- Migration runner: `crates/maproom/src/db/queries.rs` (current file)
- Migration system: `docs/architecture/DATABASE_ARCHITECTURE.md`
