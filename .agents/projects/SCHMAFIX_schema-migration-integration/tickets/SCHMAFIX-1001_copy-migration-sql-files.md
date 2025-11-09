# Ticket: SCHMAFIX-1001: Copy and Adapt Migration SQL Files

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (SQL file creation only, compilation verified with cargo check)
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
Copy 3 migration SQL files from `packages/maproom-mcp/migrations/` to `crates/maproom/migrations/` as migrations 0018-0020, adapting headers, simplifying migration 001 for transaction safety, and ensuring idempotency.

## Background
Previous projects (BLOBSHA and BRANCHX) created migration SQL files to add blob_sha column and code_embeddings table, but these migrations exist only in the MCP package and were never integrated into the Rust migration runner. This creates a critical disconnect where the database schema doesn't match code expectations, causing MCP vector search to crash with "table does not exist" errors.

The Rust binary is the single source of truth for migrations because it runs standalone without Node.js. All migrations must be copied to the Rust codebase to ensure schema consistency across environments.

**Scope Change**: Migration 005 (complete_branchx_schema) is excluded because it contains `TRUNCATE TABLE` which was a one-time development cleanup, not a repeatable migration. Only migrations 001, 002, and 004 are included.

**Transaction Safety**: Migration 001 must be simplified to remove non-transactional features (`CREATE INDEX CONCURRENTLY` and batched commits with explicit COMMIT statements), enabling safe execution within transactions.

This ticket implements **Phase 1: Migration File Preparation** from the SCHMAFIX project plan.

## Acceptance Criteria
- [x] File `crates/maproom/migrations/0018_add_blob_sha.sql` exists (adapted from MCP 001)
- [x] File `crates/maproom/migrations/0019_create_code_embeddings.sql` exists (copied from MCP 002)
- [x] File `crates/maproom/migrations/0020_add_worktree_tracking.sql` exists (copied from MCP 004)
- [x] Migration 0018 simplified: `CREATE INDEX CONCURRENTLY` removed (use regular CREATE INDEX)
- [x] Migration 0018 simplified: Batched backfill with explicit COMMIT statements removed (use single UPDATE)
- [x] All SQL files use `IF NOT EXISTS` or `IF EXISTS` clauses for idempotency
- [x] All files have headers referencing SCHMAFIX-1001 ticket
- [x] Files compile successfully with Rust's `include_str!` macro (no syntax errors)

## Technical Requirements

### Source Files Location
- `packages/maproom-mcp/migrations/001_add_blob_sha.sql` → **ADAPT** (simplify)
- `packages/maproom-mcp/migrations/002_create_code_embeddings.sql` → **COPY AS-IS**
- `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql` → **COPY AS-IS**

### Target Location
- `crates/maproom/migrations/`

### Naming Convention
- `NNNN_description.sql` where NNNN is zero-padded migration number
- Example: `0018_add_blob_sha.sql`

### Idempotency Requirements
All DDL statements must use `IF NOT EXISTS` or `IF EXISTS` clauses:
- `CREATE TABLE IF NOT EXISTS`
- `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` (PostgreSQL syntax)
- `CREATE INDEX IF NOT EXISTS`
- `DROP INDEX IF EXISTS`

### Migration Content

**Migration 0018** (adapted from `001_add_blob_sha.sql`):
- Adds `blob_sha TEXT` column to chunks table
- Includes PostgreSQL function for blob SHA computation
- **SIMPLIFIED backfill**: Single UPDATE statement (no batched DO block)
- **SIMPLIFIED index**: Regular CREATE INDEX (no CONCURRENTLY)
- Makes blob_sha NOT NULL after backfill
- **Changes from source**:
  - Remove `CREATE INDEX CONCURRENTLY` (line 44) → Use `CREATE INDEX`
  - Remove batched DO block (lines 54-100) → Use simple UPDATE statement
  - Remove validation DO blocks (lines 119-157) → Migration runner will catch errors
  - Keep function definition, ADD COLUMN, index, and SET NOT NULL

**Migration 0019** (from `002_create_code_embeddings.sql`):
- Creates `code_embeddings` table with HNSW index for vector search
- Columns: id (PK), blob_sha (UNIQUE), embedding (vector), created_at
- HNSW index on embedding column using pgvector extension
- **COPY AS-IS** (no changes needed)

**Migration 0020** (from `004_add_worktree_tracking.sql`):
- Adds `worktree_ids JSONB` column to chunks table
- Creates `worktree_index_state` table for tracking branch indexing state
- Includes GIN index on worktree_ids for efficient filtering
- Includes backfill and initialization logic
- **COPY AS-IS** (uses IF NOT EXISTS, already transaction-safe)

### Header Format
Each file should have a header comment with:
- Ticket ID: `SCHMAFIX-1001`
- Purpose: Brief description of what the migration does
- Warnings: Any special considerations (e.g., "Backfill query may take time on large databases")
- Note: For migration 0018, add note about simplifications

Example for migration 0018:
```sql
-- SCHMAFIX-1001: Add blob_sha column to chunks table
-- Purpose: Enable content-addressed chunk storage for deduplication
-- Warning: Backfill query may take 30-60 seconds on large databases
-- Source: packages/maproom-mcp/migrations/001_add_blob_sha.sql (simplified)
-- Changes: Removed CONCURRENT index and batched backfill for transaction safety
```

## Implementation Notes

### Step-by-Step Process

1. **Copy migration 002 (code_embeddings) AS-IS**
   - Read `packages/maproom-mcp/migrations/002_create_code_embeddings.sql`
   - Copy to `crates/maproom/migrations/0019_create_code_embeddings.sql`
   - Add header comment with SCHMAFIX-1001 reference
   - No other changes needed

2. **Copy migration 004 (worktree tracking) AS-IS**
   - Read `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql`
   - Copy to `crates/maproom/migrations/0020_add_worktree_tracking.sql`
   - Add header comment with SCHMAFIX-1001 reference
   - Verify IF NOT EXISTS clauses present
   - No other changes needed

3. **Adapt migration 001 (blob_sha) - CRITICAL CHANGES**
   - Read `packages/maproom-mcp/migrations/001_add_blob_sha.sql`
   - Copy to `crates/maproom/migrations/0018_add_blob_sha.sql`
   - **Make these simplifications**:

**Change 1: Simplify index creation** (original line 44):
```sql
-- BEFORE (line 44 - NOT transaction-safe):
CREATE INDEX CONCURRENTLY idx_chunks_blob_sha
ON maproom.chunks(blob_sha);

-- AFTER (transaction-safe):
CREATE INDEX IF NOT EXISTS idx_chunks_blob_sha
ON maproom.chunks(blob_sha);
```

**Change 2: Simplify backfill** (original lines 54-100):
```sql
-- BEFORE (lines 54-100 - NOT transaction-safe, complex batching):
DO $$
DECLARE
  batch_size INT := 1000;
  ...
  LOOP
    UPDATE ...
    ...
    COMMIT;  -- ERROR: Cannot COMMIT inside transaction
  END LOOP;
END $$;

-- AFTER (transaction-safe, simple):
-- Backfill all existing chunks with blob SHA
UPDATE maproom.chunks
SET blob_sha = maproom.compute_git_blob_sha(preview)
WHERE blob_sha IS NULL;
```

**Change 3: Remove validation DO blocks** (lines 119-157):
```sql
-- REMOVE: Validation queries (lines 119-157)
-- Rationale: Migration runner catches errors automatically
-- Can be added back as separate validation script if needed
```

4. **Add header comments to all 3 files**
   - Add SCHMAFIX-1001 ticket reference
   - Add purpose and warning sections
   - For migration 0018, note simplifications made
   - Document source file path

5. **Verify idempotency**
   - Check all CREATE TABLE statements have `IF NOT EXISTS`
   - Check all ADD COLUMN statements use IF NOT EXISTS
   - Check all CREATE INDEX statements have `IF NOT EXISTS`
   - Check all DROP statements have `IF EXISTS`

6. **Test compilation**
   - Ensure files can be read by Rust's `include_str!` macro
   - Run `cargo check` to verify no syntax errors
   - No invalid UTF-8 characters

### Safety Checks

**Transaction Safety** (Critical for migration 0018):
- ✅ No CREATE INDEX CONCURRENTLY (use regular CREATE INDEX)
- ✅ No explicit COMMIT statements in DO blocks
- ✅ No long-running loops with per-batch commits
- ✅ Simple UPDATE statement for backfill (transaction-safe)

**Data Preservation**:
- ✅ No DROP statements without IF EXISTS
- ✅ No DELETE or TRUNCATE statements
- ✅ All changes are additive (ADD COLUMN, CREATE TABLE, CREATE INDEX)

**Performance Considerations**:
- Index creation may take 10-30 seconds on large tables
- Backfill UPDATE in migration 0018 may lock table briefly
- Acceptable for one-time migration (better than batched approach that can't rollback)

## Dependencies

**BLOCKER**: SCHMAFIX-0001
- Migration 0017 must be added to queries.rs FIRST
- Otherwise migration numbering will be incorrect (0018 assumes 0017 exists)
- Wait for SCHMAFIX-0001 completion before starting this ticket

## Risk Assessment

**Risk**: Migration 0018 simplified version takes longer than batched version
- **Mitigation**: Acceptable tradeoff for transaction safety
- **Impact**: Table lock for 30-60 seconds during migration
- **Likelihood**: Medium (depends on chunk count)

**Risk**: Removing CONCURRENT from index creation causes table lock
- **Mitigation**: Acceptable for one-time migration, brief lock
- **Impact**: Writes blocked for 10-30 seconds
- **Likelihood**: High (expected behavior)

**Risk**: Migration SQL has syntax errors after simplification
- **Mitigation**: Test with `cargo check`, review SQL carefully
- **Impact**: Build failure, easy to fix
- **Likelihood**: Low (simple changes)

**Risk**: Missing `IF NOT EXISTS` could cause migration failures
- **Mitigation**: Systematic review of all DDL statements
- **Impact**: Migration fails if schema already exists
- **Likelihood**: Medium (manual migrations may have been run)

**Risk**: pgvector extension syntax incorrect in migration 0019
- **Mitigation**: Copying AS-IS from tested source
- **Impact**: Migration failure
- **Likelihood**: Very low (SQL already tested in BLOBSHA)

## Files/Packages Affected

### Files to Create
- `/workspace/crates/maproom/migrations/0018_add_blob_sha.sql` (adapted)
- `/workspace/crates/maproom/migrations/0019_create_code_embeddings.sql` (copied)
- `/workspace/crates/maproom/migrations/0020_add_worktree_tracking.sql` (copied)

### Files to Read (Source)
- `/workspace/packages/maproom-mcp/migrations/001_add_blob_sha.sql` (read, then adapt)
- `/workspace/packages/maproom-mcp/migrations/002_create_code_embeddings.sql` (read, copy as-is)
- `/workspace/packages/maproom-mcp/migrations/004_add_worktree_tracking.sql` (read, copy as-is)

### Files NOT Included
- `/workspace/packages/maproom-mcp/migrations/005_complete_branchx_schema.sql` - **EXCLUDED**
  - Contains `TRUNCATE TABLE` (destructive, one-time cleanup)
  - Not suitable for repeatable migration
  - Schema changes (relpath, content columns) deferred to future work

## Testing Strategy

**Compilation Test**:
```bash
# Verify Rust can read the SQL files
cd /workspace/crates/maproom
cargo check
```

**SQL Syntax Validation**:
- Review each migration for valid PostgreSQL syntax
- Check pgvector-specific syntax in migration 0019 (HNSW index)
- Verify migration 0018 simplifications are syntactically correct
- Verify all statements end with semicolons

**Idempotency Review**:
- Manually inspect each DDL statement
- Confirm `IF NOT EXISTS` or `IF EXISTS` clauses are present
- Document any exceptions with justification

**Transaction Safety Verification** (migration 0018):
- ✅ No CONCURRENTLY keywords
- ✅ No explicit COMMIT statements
- ✅ No complex DO blocks with loops
- ✅ Simple, atomic SQL statements

## Success Metrics

**Completion Criteria**:
- 3 migration files exist in Rust migrations directory (not 4)
- All files have proper headers with ticket reference
- All files pass Rust compilation (`cargo check` succeeds)
- All files use idempotent DDL statements
- Migration 0018 is transaction-safe (no CONCURRENT, no explicit COMMIT)

**Quality Criteria**:
- Clear header comments explaining purpose
- Migration 0018 header notes simplifications made
- No syntax errors or warnings
- Consistent formatting with existing migrations
- Simple, readable SQL (avoid unnecessary complexity)

## Related Planning Documents

- [SCHMAFIX Plan](../planning/plan.md) - Phase 1: Migration File Preparation
- [SCHMAFIX Architecture](../planning/architecture.md) - Migration Integration Strategy
- [SCHMAFIX Quality Strategy](../planning/quality-strategy.md) - Migration Testing Approach
- [SCHMAFIX Review Report](../planning/tickets-review-report.md) - CRITICAL-1, CRITICAL-3

## Estimated Effort
1-2 hours

**Breakdown**:
- Read and understand source migrations: 30 minutes
- Copy migrations 002 and 004: 15 minutes
- Simplify migration 001: 30 minutes (careful review needed)
- Update headers for all 3 files: 15 minutes
- Verify idempotency and syntax: 20 minutes
- Test compilation: 10 minutes

## Next Steps

After this ticket is complete:
- **SCHMAFIX-2001**: Update Rust migration runner to include 3 new migrations (0018-0020)
- **SCHMAFIX-3001**: Write comprehensive migration integration tests (expect 20 total migrations)
- **SCHMAFIX-3901**: Run migration integration tests and verify results
