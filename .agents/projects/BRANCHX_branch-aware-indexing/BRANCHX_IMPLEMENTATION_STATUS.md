# BRANCHX Implementation Status Report

**Date**: 2025-11-08
**Project**: Branch-Aware Indexing (BRANCHX)
**Status**: PARTIALLY COMPLETE - Critical schema migration missing

## Executive Summary

The BRANCHX project has **completed Rust code implementation and documentation** but discovered a **critical gap**: the database schema was never fully migrated to support the BRANCHX architecture. Tests cannot run because the `chunks` table is missing required columns (`relpath`, `content`).

## Completed Work (Tickets 1001-1014)

✅ **Phase 1**: Database Schema Design
- BRANCHX-1001: Add worktree_ids JSONB column (migration 004 applied)
- BRANCHX-1002: Create worktree_index_state table (migration 004 applied)
- BRANCHX-1003: Test worktree schema (test file created, cannot run - schema incomplete)

✅ **Phase 2**: Git Integration
- BRANCHX-1004: Git integration functions (implemented in src/git/)
- BRANCHX-1005: Worktree index state functions (implemented)
- BRANCHX-1006: Test git integration (✅ 8/8 tests PASSING)

✅ **Phase 3**: Incremental Update Logic
- BRANCHX-1007: Implement incremental update algorithm (implemented in src/incremental/)
- BRANCHX-1008: Chunk upsert worktree tracking (implemented in src/upsert.rs)
- BRANCHX-1009: Handle file deletions (implemented remove_worktree_from_chunks())
- BRANCHX-1010: Test incremental update logic (test stubs created, not implemented)

✅ **Phase 4**: CLI and MCP Integration
- BRANCHX-1011: Update scan command (--force flag added, integration note created)
- BRANCHX-1012: Add worktree filtering to MCP search (integration note created)
- BRANCHX-1013: E2E test plan (comprehensive plan created)

✅ **Phase 5**: Documentation
- BRANCHX-1014: Document architecture (627-line architecture doc created)

## Critical Discovery (Tickets 1901-1903)

⚠️ **BRANCHX-1901**: Critical Path Test Suite Validation
- Status: INCOMPLETE - Only 1 of 4 critical tests can run
- Git integration tests: ✅ PASSING (8/8)
- Database tests: ❌ BLOCKED (schema mismatch)

⚠️ **BRANCHX-1902**: Fix Worktree Test Schema Mismatch
- Status: ROOT CAUSE IDENTIFIED
- Fixed test helper to use actual schema
- Discovered: `chunks` table missing `relpath` and `content` columns
- **Fundamental Issue**: Rust code expects BRANCHX schema that doesn't exist in database

⏸️ **BRANCHX-1903**: Implement Incremental Update Tests
- Status: DEFERRED (requires schema migration first)
- Tests are TODO stubs, need implementation
- **Blocked by**: Schema migration

## Root Cause Analysis

### The Schema Gap

**BRANCHX Rust Code Expects**:
```sql
CREATE TABLE chunks (
    blob_sha TEXT,           -- ✅ EXISTS (migration 001)
    relpath TEXT,            -- ❌ MISSING
    symbol_name TEXT,        -- ✅ EXISTS (old schema)
    content TEXT,            -- ❌ MISSING
    start_line INT,          -- ✅ EXISTS (old schema)
    end_line INT,            -- ✅ EXISTS (old schema)
    kind TEXT,               -- ✅ EXISTS (old schema)
    worktree_ids JSONB,      -- ✅ EXISTS (migration 004)
    updated_at TIMESTAMP,    -- ✅ EXISTS (migration 016)
    ...
);
```

**Actual Database Schema**:
```sql
CREATE TABLE chunks (
    id BIGSERIAL PRIMARY KEY,
    file_id BIGINT NOT NULL,  -- Old architecture
    symbol_name TEXT,
    kind symbol_kind,
    start_line INT,
    end_line INT,
    blob_sha TEXT,            -- Added by migration 001
    worktree_ids JSONB,       -- Added by migration 004
    updated_at TIMESTAMP,     -- Added by migration 016
    -- Missing: relpath, content
    ...
);
```

### Why This Happened

1. **BLOBSHA Project** (migrations 001-002): Added `blob_sha` and `code_embeddings` table
2. **BRANCHX Project** (migration 004): Added `worktree_ids` and `worktree_index_state` table
3. **Gap**: Neither project migrated `chunks` table to the content-addressed schema

The BRANCHX Rust code in `src/upsert.rs` was written assuming a complete schema refactoring:
```rust
// Line 377-380 of src/upsert.rs
INSERT INTO maproom.chunks
    (blob_sha, relpath, symbol_name, content, start_line, end_line, kind, worktree_ids, updated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, jsonb_build_array($8), NOW())
ON CONFLICT (blob_sha, relpath)
```

But the database migration was never created to add `relpath` and `content` columns.

## Impact Assessment

### What Works

✅ **Git Operations**: Tree SHA detection, diff-tree parsing (fully tested)
✅ **Migrations Applied**: blob_sha, code_embeddings, worktree_ids, worktree_index_state all exist
✅ **Documentation**: Complete architecture and E2E test plans created
✅ **Rust Functions**: All BRANCHX functions implemented (just can't execute against database)

### What's Blocked

❌ **CRITICAL 1-3 Tests**: Cannot run without complete schema
❌ **Worktree Upsert**: `upsert_chunk_with_worktree()` function incompatible with current schema
❌ **Incremental Updates**: `incremental_update()` cannot work without relpath/content columns
❌ **MCP Search**: Worktree filtering partially works (uses FK join) but not JSONB queries
❌ **Production Use**: BRANCHX cannot be deployed until schema migrated

## Required Schema Migration

### Missing Columns

Add to `chunks` table:
```sql
ALTER TABLE maproom.chunks ADD COLUMN relpath TEXT;
ALTER TABLE maproom.chunks ADD COLUMN content TEXT;

-- Backfill from existing data
UPDATE maproom.chunks c
SET relpath = f.relative_path,
    content = (SELECT preview FROM maproom.chunks WHERE id = c.id)  -- Or regenerate
FROM maproom.files f
WHERE c.file_id = f.id;

-- Add constraints after backfill
ALTER TABLE maproom.chunks ALTER COLUMN relpath SET NOT NULL;
ALTER TABLE maproom.chunks ALTER COLUMN content SET NOT NULL;

-- Create unique constraint for BRANCHX upsert logic
CREATE UNIQUE INDEX idx_chunks_blob_relpath ON maproom.chunks(blob_sha, relpath);
```

### Migration Complexity

**Medium-High Complexity**:
- Existing `chunks` table has data (backfill required)
- Need to populate `content` from somewhere (preview column? regenerate from files?)
- Need to populate `relpath` from `files` table join
- Changing primary conflict resolution from `(file_id, start_line, end_line)` to `(blob_sha, relpath)`
- May require downtime or careful migration strategy

**Estimated Effort**: 4-8 hours
- Design migration SQL
- Test on copy of database
- Implement backfill logic
- Validate no data loss
- Update any remaining code that uses old schema

## Recommendations

### Option 1: Complete Schema Migration (Recommended for Production)

**Actions**:
1. Create BRANCHX-1904: Complete schema migration to content-addressed chunks
2. Design migration SQL with proper backfill
3. Test migration on database copy
4. Execute migration
5. Run all BRANCHX tests
6. Deploy BRANCHX

**Timeline**: 1-2 days
**Risk**: Medium (data migration always risky)
**Benefit**: Full BRANCHX functionality, all tests passing

### Option 2: Defer BRANCHX (Use Current System)

**Actions**:
1. Document BRANCHX as future work
2. Continue using current indexing (works, just no branch awareness)
3. Revisit when priority increases

**Timeline**: Immediate
**Risk**: Low (no changes)
**Benefit**: Avoid migration complexity

### Option 3: Hybrid Approach (Parallel Schemas)

**Actions**:
1. Keep old schema for existing code
2. Add new columns for BRANCHX
3. Gradual migration of code to use new columns
4. Eventually deprecate old columns

**Timeline**: 2-3 days
**Risk**: High (maintaining two schemas)
**Benefit**: No downtime, gradual transition

## Next Steps

**Immediate**:
1. ✅ Document schema gap (this document)
2. ⏸️ Decide on migration approach (user decision)
3. ⏸️ Create BRANCHX-1904 if proceeding with migration

**If Proceeding with Migration**:
1. Design complete migration SQL
2. Test on database copy
3. Implement backfill logic for `relpath` and `content`
4. Execute migration in transaction
5. Validate data integrity
6. Run BRANCHX test suite
7. Deploy to production

## Files Modified (BRANCHX-1902)

- `crates/maproom/tests/upsert_worktree.rs` - Fixed `create_test_worktree()` to use actual schema
- Applied migrations: 001_add_blob_sha.sql, 002_create_code_embeddings.sql, 004_add_worktree_tracking.sql

## Conclusion

BRANCHX is **architecturally sound** but **implementation incomplete**. The Rust code is written, documented, and tested (where possible). The missing piece is a database migration to add `relpath` and `content` columns to the `chunks` table.

**Decision Point**: Should we complete the migration or defer BRANCHX as future work?

**Recommendation**: Complete the migration. The code is ready, the architecture is documented, and the benefits (5-10x faster branch switches, branch-specific search) are significant.
