# Ticket: SQLITE-1001: Schema Migration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create schema migrations to add junction table, embedding storage table, and vector index table while removing deprecated columns/tables (`worktree_ids JSON` and `vec_chunks`).

## Background
The schema needs proper structure for:
1. A junction table (`chunk_worktrees`) for proper worktree tracking
2. Deduplicated embedding storage (`code_embeddings` keyed by blob_sha)
3. New vector index (`vec_code`) synced from code_embeddings

> **Note**: No data migration is required. There are no existing SQLite databases with data. Fresh indexing populates all tables from scratch.

Implements: Plan Phase 1 - Schema Foundation

## Acceptance Criteria
- [x] Migration 2 creates `chunk_worktrees` junction table with composite primary key
- [x] Migration 3 creates `code_embeddings` table with blob_sha unique constraint
- [x] Migration 4 creates `vec_code` virtual table using vec0 (1536-dim)
- [x] Migration 5 drops the deprecated worktree_ids column
- [x] Migration 6 drops the deprecated `vec_chunks` table
- [x] All migrations run without error on fresh database
- [x] Proper indexes created for query performance

## Technical Requirements
Create these migrations using the Phase 0 migration system:

```sql
-- Migration 2: add_chunk_worktrees
CREATE TABLE chunk_worktrees (
  chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
  worktree_id INTEGER NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,
  PRIMARY KEY (chunk_id, worktree_id)
);
CREATE INDEX idx_chunk_worktrees_worktree ON chunk_worktrees(worktree_id);

-- Migration 3: add_code_embeddings
CREATE TABLE code_embeddings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  blob_sha TEXT NOT NULL UNIQUE,
  embedding BLOB,
  embedding_dim INTEGER NOT NULL DEFAULT 1536,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_embeddings_blob ON code_embeddings(blob_sha);

-- Migration 4: add_vec_code
CREATE VIRTUAL TABLE vec_code USING vec0(
  embedding float[1536]
);

-- Migration 5: drop_worktree_ids
-- For fresh databases: removes deprecated column from chunks table
-- SQLite 3.35.0+ supports ALTER TABLE DROP COLUMN directly
ALTER TABLE chunks DROP COLUMN worktree_ids;

-- Migration 6: drop_vec_chunks
-- Removes deprecated table (replaced by code_embeddings + vec_code)
DROP TABLE IF EXISTS vec_chunks;
```

## Implementation Notes
- Migration 4 (vec_code) requires sqlite-vec extension - if extension not available, migration should still succeed but table will be unusable
- Migration 5 (drop_worktree_ids) uses ALTER TABLE DROP COLUMN (requires SQLite 3.35.0+)
- Migration 6 (drop_vec_chunks) uses DROP TABLE IF EXISTS (safe for fresh databases where table doesn't exist)
- FTS5 sync remains manual INSERT (no trigger changes needed)
- **Code impact**: The existing `VectorStore::upsert_embeddings()` method in mod.rs uses vec_chunks. After Migration 6, this method will error. SQLITE-2001 adds the replacement `upsert_embedding()` method.
- **No data migration needed**: All tables populated via fresh indexing

## Dependencies
- SQLITE-0001 (Migration System) - migration infrastructure must exist
- SQLITE-0002 (Extension Verification) - need to know if vec_code creation will work

## Risk Assessment
- **Risk**: vec_code table creation fails without extension
  - **Mitigation**: Check extension availability; skip vec_code if not available
- **Risk**: DROP COLUMN not supported on SQLite < 3.35.0
  - **Mitigation**: Check SQLite version; use table recreation pattern if needed
- **Risk**: Code using VectorStore::upsert_embeddings breaks after Migration 6
  - **Mitigation**: SQLITE-2001 provides replacement method; update callers

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/migrations.rs` (add migrations 2-6)
- `crates/maproom/src/db/sqlite/schema.rs` (update schema documentation, remove vec_chunks creation)

## Implementation Complete

### Changes Made

1. **Added 5 new migrations to migrations.rs** (versions 2-6):
   - Migration 2: `add_chunk_worktrees` - Creates junction table with composite primary key and index
   - Migration 3: `add_code_embeddings` - Creates deduplicated embedding storage with unique blob_sha constraint
   - Migration 4: `add_vec_code` - Creates vec0 virtual table for vector indexing
   - Migration 5: `drop_worktree_ids` - Removes deprecated JSON column from chunks table
   - Migration 6: `drop_vec_chunks` - Removes deprecated vec_chunks virtual table

2. **Updated test suite**:
   - Modified `test_migration_fresh_database` to expect version 6 and verify new tables exist
   - Modified `test_migration_idempotent` to expect 6 migrations
   - Added `test_new_migrations_schema` to verify schema structure of new tables and dropped columns

3. **Updated schema.rs**:
   - Marked `init_schema` as deprecated with documentation note
   - Added `#[allow(dead_code)]` attribute since function is no longer used

### Test Results

All 5 migration tests pass successfully:
```
test db::sqlite::migrations::tests::test_migration_fresh_database ... ok
test db::sqlite::migrations::tests::test_migration_idempotent ... ok
test db::sqlite::migrations::tests::test_migration_rollback_on_failure ... ok
test db::sqlite::migrations::tests::test_migration_version_tracking ... ok
test db::sqlite::migrations::tests::test_new_migrations_schema ... ok
```

The new test `test_new_migrations_schema` specifically verifies:
- chunk_worktrees table has correct schema with composite primary key
- idx_chunk_worktrees_worktree index exists
- code_embeddings table has blob_sha UNIQUE constraint
- idx_embeddings_blob index exists
- vec_code virtual table exists
- worktree_ids column is dropped from chunks table
- vec_chunks table is dropped

### Compilation

Code compiles successfully with `cargo check --features sqlite`. No errors, only expected warnings from unfinished SQLite implementation methods.
