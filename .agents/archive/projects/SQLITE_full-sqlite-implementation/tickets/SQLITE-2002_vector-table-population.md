# Ticket: SQLITE-2002: Vector Table Population

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
Implement synchronization between the `code_embeddings` table and the `vec_code` virtual table for vector similarity search.

## Background
The `vec_code` table is a sqlite-vec virtual table that provides KNN search. It needs to be populated from `code_embeddings` and kept in sync. The rowid mapping enables joining search results back to chunks.

Implements: Plan Phase 2 - Embedding Storage

## Acceptance Criteria
- [x] `sync_embedding_to_vec(embedding_id)` inserts embedding into vec_code with matching rowid
- [x] `sync_embeddings_batch()` efficiently syncs multiple embeddings
- [x] Rowid in vec_code matches id in code_embeddings for JOIN operations
- [x] Existing embeddings are synced on startup if needed (via `sync_all_embeddings_to_vec`)
- [x] Sync is skipped gracefully if sqlite-vec extension not available
- [x] Unit tests verify rowid mapping is correct

## Technical Requirements
Extend `embeddings.rs` with vec_code sync:

```rust
impl SqliteStore {
    /// Sync single embedding to vector index
    async fn sync_embedding_to_vec(&self, embedding_id: i64, embedding: &[f32]) -> Result<()> {
        if !self.has_vec_extension() {
            return Ok(());  // Skip silently
        }

        self.run(move |conn| {
            // Delete existing if any (for updates)
            conn.execute("DELETE FROM vec_code WHERE rowid = ?1", params![embedding_id])?;

            // Insert with explicit rowid
            conn.execute(
                "INSERT INTO vec_code(rowid, embedding) VALUES (?1, ?2)",
                params![embedding_id, vec_to_blob(&embedding)],
            )?;
            Ok(())
        }).await
    }

    /// Sync all embeddings that aren't in vec_code yet
    pub async fn sync_all_embeddings_to_vec(&self) -> Result<usize> {
        if !self.has_vec_extension() {
            return Ok(0);
        }

        self.run(|conn| {
            // Find embeddings not yet in vec_code
            let mut stmt = conn.prepare(
                "SELECT e.id, e.embedding FROM code_embeddings e
                 WHERE NOT EXISTS (SELECT 1 FROM vec_code v WHERE v.rowid = e.id)"
            )?;

            let mut count = 0;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?;

            for row in rows {
                let (id, blob) = row?;
                conn.execute(
                    "INSERT INTO vec_code(rowid, embedding) VALUES (?1, ?2)",
                    params![id, blob],
                )?;
                count += 1;
            }

            Ok(count)
        }).await
    }
}
```

Update `upsert_embedding` to also sync:
```rust
pub async fn upsert_embedding(...) -> Result<i64> {
    let embedding_id = // ... insert into code_embeddings ...

    // Sync to vec_code
    self.sync_embedding_to_vec(embedding_id, embedding).await?;

    Ok(embedding_id)
}
```

## Implementation Notes
- Use explicit rowid insertion to ensure mapping between tables
- Delete-then-insert pattern handles updates (vec_code doesn't support UPDATE)
- Batch sync should use transaction for performance
- Check extension availability before any vec_code operations
- The join pattern is: chunks.blob_sha → code_embeddings.blob_sha → vec_code.rowid = code_embeddings.id

## Dependencies
- SQLITE-2001 (Embedding Module) - base embedding operations
- SQLITE-0002 (Extension Verification) - need `has_vec_extension()`

## Risk Assessment
- **Risk**: Rowid mismatch causes incorrect search results
  - **Mitigation**: Use explicit rowid insertion; add verification test
- **Risk**: Large sync (millions of embeddings) is slow
  - **Mitigation**: Batch in transactions of 1000; show progress for large syncs

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/embeddings.rs` (add sync methods)
- `crates/maproom/src/db/sqlite/mod.rs` (possibly add startup sync)

## Implementation Summary

Implemented vector table synchronization between `code_embeddings` and `vec_code` tables:

### Changes to embeddings.rs
1. Added `sync_embedding_to_vec(conn, embedding_id, embedding)` - syncs single embedding to vec_code with explicit rowid matching
2. Added `sync_all_embeddings_to_vec(conn)` - batch syncs all unsynced embeddings, returns count
3. Simplified `upsert_embedding` to only insert into code_embeddings (vec_code sync moved to async wrapper)
4. Updated `upsert_embeddings_batch` to return Vec<(i64, Vec<f32>)> for subsequent syncing
5. Added comprehensive unit tests covering:
   - Basic sync with rowid verification
   - Update handling (delete-then-insert pattern)
   - Batch sync and idempotency
   - Graceful degradation when extension unavailable

### Changes to mod.rs
1. Added async `sync_embedding_to_vec(embedding_id, embedding)` wrapper with extension check
2. Added async `sync_all_embeddings_to_vec()` wrapper for startup/manual sync
3. Updated `upsert_embedding` to auto-sync after inserting into code_embeddings
4. Updated `upsert_embeddings_batch_new` to sync all embeddings after batch insert
5. Enhanced `migrate()` to verify vec extension after migrations and set availability flags
6. Added integration tests verifying:
   - Auto-sync on upsert
   - Update without duplicates
   - Batch sync workflow
   - Rowid mapping correctness

### Key Design Decisions
- Delete-then-insert pattern for updates (vec_code virtual tables don't support UPDATE)
- Explicit rowid insertion ensures vec_code.rowid = code_embeddings.id for JOIN operations
- Extension availability checked once after migration and cached
- Graceful degradation: sync operations skip silently when extension unavailable
- Separation of concerns: sync logic in embeddings.rs, async wrappers in mod.rs

All 18 SQLite tests pass, including 8 new tests for vector table synchronization.
