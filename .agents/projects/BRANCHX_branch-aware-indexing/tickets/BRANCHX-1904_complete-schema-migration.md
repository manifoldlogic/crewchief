# Ticket: BRANCHX-1904: Complete BRANCHX schema migration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - schema migration successful
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Execute complete schema migration to transform chunks table from file-based to content-addressed architecture. No data preservation needed - clean schema transformation.

## Background
BRANCHX implementation revealed that Rust code expects content-addressed schema (`blob_sha`, `relpath`, `content`, `worktree_ids`) but database was never migrated from file-based schema (`file_id`).

Since there are no production users, we can execute a clean migration:
- Drop/modify columns as needed
- Update constraints
- No data backfill required
- Fresh start with correct schema

**Reference**: `BRANCHX_IMPLEMENTATION_STATUS.md` - Root cause analysis

## Acceptance Criteria
- [ ] `chunks` table has `relpath` column (TEXT NOT NULL)
- [ ] `chunks` table has `content` column (TEXT NOT NULL)
- [ ] `chunks` table has `blob_sha` column (TEXT NOT NULL) - already exists
- [ ] `chunks` table has `worktree_ids` column (JSONB NOT NULL DEFAULT '[]') - already exists
- [ ] Primary conflict resolution changed from `(file_id, start_line, end_line)` to `(blob_sha, relpath)`
- [ ] Unique index created: `idx_chunks_blob_relpath ON chunks(blob_sha, relpath)`
- [ ] Old `file_id` column handling decided (keep for relations or drop)
- [ ] Migration script created and executed
- [ ] All existing migrations still apply cleanly
- [ ] Schema validated with `\d maproom.chunks`

## Technical Requirements

### Migration File

Create `/workspace/packages/maproom-mcp/migrations/005_complete_branchx_schema.sql`:

```sql
-- ============================================================================
-- Migration 005: Complete BRANCHX Schema Transformation
-- ============================================================================
--
-- Transforms chunks table from file-based to content-addressed architecture.
-- No data preservation - clean schema migration for development environment.
--
-- Changes:
-- 1. Add relpath and content columns
-- 2. Change unique constraint from (file_id, start_line, end_line) to (blob_sha, relpath)
-- 3. Make blob_sha and worktree_ids NOT NULL with proper defaults
--
-- Prerequisites:
-- - Migration 001 (blob_sha column exists)
-- - Migration 004 (worktree_ids column exists)
--
-- ============================================================================

BEGIN;

-- Step 1: Add new columns
ALTER TABLE maproom.chunks
ADD COLUMN IF NOT EXISTS relpath TEXT,
ADD COLUMN IF NOT EXISTS content TEXT;

COMMENT ON COLUMN maproom.chunks.relpath IS 'File path relative to repository root (enables content-addressed chunks)';
COMMENT ON COLUMN maproom.chunks.content IS 'Actual source code content of this chunk';

-- Step 2: For clean migration (no users), we can just make them NOT NULL
-- If there's existing data, truncate first since we don't need to preserve it
TRUNCATE TABLE maproom.chunks CASCADE;

-- Now set NOT NULL constraints
ALTER TABLE maproom.chunks
ALTER COLUMN relpath SET NOT NULL,
ALTER COLUMN content SET NOT NULL;

-- Step 3: Drop old unique constraint
ALTER TABLE maproom.chunks
DROP CONSTRAINT IF EXISTS chunks_file_id_start_line_end_line_key;

-- Step 4: Create new unique constraint for content-addressed approach
CREATE UNIQUE INDEX IF NOT EXISTS idx_chunks_blob_relpath
ON maproom.chunks(blob_sha, relpath);

COMMENT ON INDEX maproom.idx_chunks_blob_relpath IS
  'Unique constraint for BRANCHX: same content (blob_sha) in same file (relpath) = same chunk';

-- Step 5: Create index on relpath for queries
CREATE INDEX IF NOT EXISTS idx_chunks_relpath
ON maproom.chunks(relpath);

COMMENT ON INDEX maproom.idx_chunks_relpath IS
  'Fast lookup of chunks by file path';

-- Step 6: Ensure blob_sha is NOT NULL
-- Truncate already happened, so just set constraint
ALTER TABLE maproom.chunks
ALTER COLUMN blob_sha SET NOT NULL;

-- Step 7: Validation
DO $$
DECLARE
    relpath_count INTEGER;
    content_count INTEGER;
    index_exists BOOLEAN;
BEGIN
    -- Check columns exist and are NOT NULL
    SELECT COUNT(*) INTO relpath_count
    FROM information_schema.columns
    WHERE table_schema = 'maproom'
      AND table_name = 'chunks'
      AND column_name = 'relpath'
      AND is_nullable = 'NO';

    SELECT COUNT(*) INTO content_count
    FROM information_schema.columns
    WHERE table_schema = 'maproom'
      AND table_name = 'chunks'
      AND column_name = 'content'
      AND is_nullable = 'NO';

    -- Check unique index exists
    SELECT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'maproom'
          AND tablename = 'chunks'
          AND indexname = 'idx_chunks_blob_relpath'
    ) INTO index_exists;

    IF relpath_count = 0 THEN
        RAISE EXCEPTION 'Migration validation failed: relpath column missing or nullable';
    END IF;

    IF content_count = 0 THEN
        RAISE EXCEPTION 'Migration validation failed: content column missing or nullable';
    END IF;

    IF NOT index_exists THEN
        RAISE EXCEPTION 'Migration validation failed: idx_chunks_blob_relpath index missing';
    END IF;

    RAISE NOTICE 'Validation passed: BRANCHX schema migration complete';
    RAISE NOTICE '  - relpath column: NOT NULL';
    RAISE NOTICE '  - content column: NOT NULL';
    RAISE NOTICE '  - blob_sha column: NOT NULL';
    RAISE NOTICE '  - worktree_ids column: JSONB NOT NULL DEFAULT ''''[]''''';
    RAISE NOTICE '  - Unique constraint: (blob_sha, relpath)';
END $$;

COMMIT;

-- ============================================================================
-- Post-Migration Notes
-- ============================================================================
--
-- The chunks table now supports:
-- 1. Content-addressed storage (blob_sha)
-- 2. Multi-worktree tracking (worktree_ids JSONB array)
-- 3. File-based conflict resolution (blob_sha + relpath)
-- 4. Direct content storage (content column)
--
-- Old file_id column still exists for backward compatibility with:
-- - chunk_edges foreign keys
-- - Any code that hasn't been updated yet
--
-- Future work: Remove file_id dependency entirely
-- ============================================================================
```

### Execution Steps

1. **Backup current schema** (optional, but good practice):
```bash
PGPASSWORD=maproom pg_dump -h maproom-postgres -U maproom -d maproom --schema-only > schema_backup_pre_branchx.sql
```

2. **Apply migration**:
```bash
PGPASSWORD=maproom psql -h maproom-postgres -U maproom -d maproom -f /workspace/packages/maproom-mcp/migrations/005_complete_branchx_schema.sql
```

3. **Verify schema**:
```bash
PGPASSWORD=maproom psql -h maproom-postgres -U maproom -d maproom -c "\d maproom.chunks"
```

4. **Expected output**:
```
Column         | Type          | Nullable | Default
---------------|---------------|----------|----------
id             | bigint        | not null | nextval(...)
file_id        | bigint        | not null |
blob_sha       | text          | not null |
relpath        | text          | not null |
symbol_name    | text          |          |
content        | text          | not null |
start_line     | integer       | not null |
end_line       | integer       | not null |
kind           | text          |          |
worktree_ids   | jsonb         | not null | '[]'::jsonb
updated_at     | timestamp     |          | now()
...

Indexes:
    "chunks_pkey" PRIMARY KEY, btree (id)
    "idx_chunks_blob_relpath" UNIQUE, btree (blob_sha, relpath)
    "idx_chunks_blob_sha" btree (blob_sha)
    "idx_chunks_relpath" btree (relpath)
    "idx_chunks_worktree_ids" gin (worktree_ids)
```

## Implementation Notes

### Design Decisions

**Keep file_id column (for now)**:
- Still referenced by `chunk_edges` foreign keys
- May be referenced by other code not yet discovered
- Can be removed in future cleanup ticket after validating nothing uses it

**Truncate existing data**:
- User confirmed no production users
- Clean slate ensures schema consistency
- Avoids complex backfill logic
- Future indexing will populate correctly

**Unique constraint (blob_sha, relpath)**:
- Enables BRANCHX upsert logic in `src/upsert.rs`
- Same content in same file = same chunk
- Different files with same content = different chunks
- Supports worktree_ids array for multi-branch tracking

### Code Compatibility

After migration, this code will work:
```rust
// src/upsert.rs line 377
INSERT INTO maproom.chunks
    (blob_sha, relpath, symbol_name, content, start_line, end_line, kind, worktree_ids, updated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, jsonb_build_array($8), NOW())
ON CONFLICT (blob_sha, relpath)
DO UPDATE SET
  worktree_ids = CASE
    WHEN chunks.worktree_ids ? $8::TEXT THEN chunks.worktree_ids
    ELSE chunks.worktree_ids || jsonb_build_array($8)
  END,
  updated_at = NOW()
RETURNING id
```

### Testing After Migration

1. Run worktree tests:
```bash
cargo test --test upsert_worktree -- --ignored --nocapture
```

Expected: All 5 tests pass

2. Verify schema:
```bash
PGPASSWORD=maproom psql -h maproom-postgres -U maproom -d maproom -c "
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema = 'maproom' AND table_name = 'chunks'
ORDER BY ordinal_position;
"
```

3. Test upsert manually:
```sql
-- Should succeed
INSERT INTO maproom.chunks (blob_sha, relpath, content, symbol_name, start_line, end_line, kind, worktree_ids)
VALUES ('abc123', 'src/test.rs', 'fn test() {}', 'test', 1, 1, 'function', '[1]'::jsonb);

-- Should succeed (idempotent)
INSERT INTO maproom.chunks (blob_sha, relpath, content, symbol_name, start_line, end_line, kind, worktree_ids)
VALUES ('abc123', 'src/test.rs', 'fn test() {}', 'test', 1, 1, 'function', '[1]'::jsonb)
ON CONFLICT (blob_sha, relpath) DO NOTHING;

-- Should succeed (add worktree 2)
INSERT INTO maproom.chunks (blob_sha, relpath, content, symbol_name, start_line, end_line, kind, worktree_ids)
VALUES ('abc123', 'src/test.rs', 'fn test() {}', 'test', 1, 1, 'function', '[2]'::jsonb)
ON CONFLICT (blob_sha, relpath)
DO UPDATE SET worktree_ids = chunks.worktree_ids || '[2]'::jsonb;

-- Verify
SELECT blob_sha, relpath, worktree_ids FROM maproom.chunks WHERE blob_sha = 'abc123';
-- Expected: worktree_ids = [1, 2]
```

## Dependencies
- Migration 001 applied (blob_sha column)
- Migration 002 applied (code_embeddings table)
- Migration 004 applied (worktree_ids column, worktree_index_state table)
- BRANCHX-1902 investigation complete

## Risk Assessment
- **Risk**: Breaking existing code that uses file_id
  - **Mitigation**: Keep file_id column, add TODO to remove it later, search codebase for file_id usage
- **Risk**: Migration fails mid-transaction
  - **Mitigation**: Wrapped in BEGIN/COMMIT, automatically rolls back on error
- **Risk**: Indexes slow down inserts
  - **Mitigation**: GIN and btree indexes are efficient, measure if needed
- **Risk**: TRUNCATE CASCADE affects other tables
  - **Mitigation**: Expected behavior - chunk_edges will cascade delete (no users, fresh start)

## Files/Packages Affected
- `packages/maproom-mcp/migrations/005_complete_branchx_schema.sql` (new)
- Database: `maproom.chunks` table schema
- Potentially: Any code using `file_id` column (to be identified)

## Success Metrics
- Migration executes without errors
- Schema validation passes (relpath, content, blob_sha all NOT NULL)
- Unique constraint `idx_chunks_blob_relpath` exists
- Worktree upsert tests pass (all 5 tests)
- Manual upsert test succeeds

## Follow-Up Work
- BRANCHX-1903: Implement incremental update tests (now unblocked)
- BRANCHX-1901: Re-run critical path test suite (should pass 4/4 tests)
- Future: Remove file_id column after validating nothing uses it
- Future: Update any remaining code using old schema patterns
