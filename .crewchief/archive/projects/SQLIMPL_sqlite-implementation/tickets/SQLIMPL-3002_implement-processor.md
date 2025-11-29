# Ticket: SQLIMPL-3002: Implement Processor

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 82 incremental tests passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the incremental processor for indexing, updating, and removing files. This handles the actual database operations when files change.

## Background
The processor at `src/incremental/processor.rs` has 3 stubbed methods for file operations. These orchestrate parsing, chunking, and database updates when files are added, modified, or deleted.

This ticket implements Plan Phase 3, Ticket 3002: "Implement Processor".

## Acceptance Criteria
- [x] `index_new_file()` parses file, creates chunks, inserts to DB
- [x] `update_file()` deletes old chunks, creates and inserts new chunks
- [x] `remove_file()` deletes all chunks for a removed file
- [x] Database correctly reflects file state after each operation
- [x] Processor tests (from Phase 1) now pass

## Technical Requirements
- Use existing tree-sitter parsing infrastructure
- Use existing chunk creation logic
- Transaction support for atomic updates
- Clean up related data (embeddings, edges) when removing chunks

## Implementation Notes

### Current Stubs (to implement)
```rust
// src/incremental/processor.rs:258
// index_new_file() - stub

// src/incremental/processor.rs:339
// update_file() - stub

// src/incremental/processor.rs:384
// remove_file() - stub
```

### Target Implementation Patterns

#### Index New File
```rust
pub async fn index_new_file(&self, path: &Path, repo_id: i64, worktree_id: i64) -> Result<()> {
    // 1. Read file content
    let content = tokio::fs::read_to_string(path).await?;

    // 2. Parse with tree-sitter (use existing parser)
    let chunks = self.parser.parse_file(path, &content)?;

    // 3. Insert file record
    let file_id = self.store.run(|conn| {
        conn.execute(
            "INSERT INTO files (repo_id, relpath, blob_sha) VALUES (?, ?, ?)",
            params![repo_id, path.to_string_lossy(), compute_sha(&content)]
        )?;
        Ok(conn.last_insert_rowid())
    }).await?;

    // 4. Insert chunks
    for chunk in chunks {
        self.store.insert_chunk(&chunk).await?;
    }

    Ok(())
}
```

#### Update File
```rust
pub async fn update_file(&self, path: &Path, file_id: i64) -> Result<()> {
    // 1. Delete old chunks for this file
    self.store.run(move |conn| {
        conn.execute(
            "DELETE FROM chunks WHERE file_id = ?",
            [file_id]
        )?;
        Ok(())
    }).await?;

    // 2. Re-index the file (same as index_new_file but with existing file_id)
    let content = tokio::fs::read_to_string(path).await?;
    let chunks = self.parser.parse_file(path, &content)?;

    for chunk in chunks {
        self.store.insert_chunk(&chunk).await?;
    }

    // 3. Update file hash
    self.detector.store_hash_in_db(file_id, &compute_sha(&content)).await?;

    Ok(())
}
```

#### Remove File
```rust
pub async fn remove_file(&self, file_id: i64) -> Result<()> {
    self.store.run(move |conn| {
        // Delete chunks first (foreign key)
        conn.execute("DELETE FROM chunks WHERE file_id = ?", [file_id])?;

        // Delete chunk_edges referencing these chunks
        conn.execute(
            "DELETE FROM chunk_edges WHERE src_chunk_id IN
             (SELECT id FROM chunks WHERE file_id = ?)",
            [file_id]
        )?;

        // Delete file record
        conn.execute("DELETE FROM files WHERE id = ?", [file_id])?;

        Ok(())
    }).await
}
```

## Dependencies
- SQLIMPL-3001 (Change Detector - for hash operations)
- Phase 1 Complete (tests compile)

## Risk Assessment
- **Risk**: Partial updates if transaction fails mid-operation
  - **Mitigation**: Use transactions; rollback on error
- **Risk**: Orphaned embeddings/edges after chunk deletion
  - **Mitigation**: Delete related records in same transaction

## Files/Packages Affected
- `crates/maproom/src/incremental/processor.rs` (primary)
