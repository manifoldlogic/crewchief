# Ticket: INC_INDEX-3002: Incremental File Processing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement incremental file processing that updates individual files atomically with transaction integrity, correctly maintains chunk edges, and handles failures gracefully without corrupting the index.

## Background
The incremental indexing pipeline requires a processor that can handle individual file updates as they are detected by the file watcher and queued. Unlike full scans that process entire directories, incremental processing must handle three distinct change types (new, modified, deleted) while maintaining database consistency through transactions. This is critical for real-time indexing where updates must be fast, atomic, and reliable.

The processor must coordinate with the existing parser infrastructure to re-parse changed files, update the chunk database atomically, and ensure chunk edges (relationships between code symbols) remain consistent after updates.

## Acceptance Criteria
- [ ] Single file updates are atomic (all-or-nothing transaction semantics)
- [ ] Transaction integrity maintained (rollback on failure prevents corruption)
- [ ] Edges updated correctly after file changes (chunk_edges table stays consistent)
- [ ] Failures don't corrupt data (graceful error handling with rollback)
- [ ] New files are indexed and added to the database
- [ ] Modified files have old chunks deleted and new chunks inserted
- [ ] Deleted files have all chunks and edges removed
- [ ] Performance: File updates complete in <5s for typical files

## Technical Requirements
- Implement `IncrementalProcessor` struct with transaction-based processing
- Handle three change types: `ChangeType::New`, `ChangeType::Modified`, `ChangeType::Deleted`
- Integrate with existing `ParserFactory` for file parsing
- Use PostgreSQL transactions (BEGIN...COMMIT/ROLLBACK) for atomicity
- Implement `EdgeUpdater` for maintaining chunk relationships
- Process tasks from the `UpdateQueue` (from INC_INDEX-3001)
- Delete old chunks before inserting new ones (for modified files)
- Update file records with new content hashes and timestamps
- Recompute and update chunk edges after file changes
- Rollback transactions on any processing error

## Implementation Notes

### Architecture Reference
See `/workspace/crewchief_context/maproom/INC_INDEX/INC_INDEX_ARCHITECTURE.md`:
- Lines 116-174: IncrementalProcessor design
- Lines 176-200: EdgeUpdater design

### Key Components

**1. IncrementalProcessor (crates/maproom/src/incremental/processor.rs)**
```rust
pub struct IncrementalProcessor {
    parser_factory: ParserFactory,
    db: DatabaseConnection,
}
```

Methods:
- `process(task: UpdateTask) -> Result<()>` - Main entry point
- `index_new_file(path, hash) -> Result<()>` - Add new file
- `update_file(path, old_hash, new_hash) -> Result<()>` - Update existing file
- `remove_file(path) -> Result<()>` - Remove deleted file
- `update_edges(path) -> Result<()>` - Update relationships

**2. EdgeUpdater (crates/maproom/src/incremental/edge_updater.rs)**
```rust
pub struct EdgeUpdater {
    db: DatabaseConnection,
}
```

Methods:
- `update_edges(changed_file: &Path) -> Result<()>` - Main entry point
- `get_file_chunks(file: &Path) -> Result<Vec<ChunkId>>` - Find affected chunks
- `compute_edges(chunks: &[ChunkId]) -> Result<Vec<Edge>>` - Recompute relationships
- `insert_edge(edge: Edge) -> Result<()>` - Add edge to database

**3. Transaction Flow for Modified Files**
```
BEGIN TRANSACTION
  1. DELETE old chunks (WHERE file_id = X AND content_hash = old_hash)
  2. INSERT new chunks (parsed from updated file)
  3. UPDATE file record (SET content_hash = new_hash, last_modified = NOW())
  4. DELETE old edges (WHERE src_chunk_id IN affected_chunks OR dst_chunk_id IN affected_chunks)
  5. INSERT new edges (recomputed from new chunks)
COMMIT TRANSACTION
```

**4. Error Handling**
- Wrap all operations in transactions
- Use `?` operator to propagate errors
- Transaction auto-rollback on error
- Log failures for dead letter queue processing

**5. Integration Points**
- Receives tasks from `UpdateQueue` (INC_INDEX-3001)
- Uses existing `ParserFactory` for file parsing
- Updates `maproom.files`, `maproom.chunks`, `maproom.chunk_edges` tables
- Reports processing results back to caller

## Dependencies
- **INC_INDEX-3001** (Update Queue) - Required: Queue provides tasks to process
- Existing parser infrastructure (`ParserFactory`, language parsers)
- PostgreSQL database with `maproom` schema
- sqlx for database operations

## Risk Assessment
- **Risk**: Partial updates could corrupt the index if transactions fail mid-flight
  - **Mitigation**: Use PostgreSQL transactions with automatic rollback on error

- **Risk**: Edge updates could become inconsistent if chunk IDs change
  - **Mitigation**: Delete all edges for affected chunks before recomputing new edges

- **Risk**: Large files could cause transaction timeouts
  - **Mitigation**: Set reasonable transaction timeout limits, consider chunked processing for very large files

- **Risk**: Concurrent updates to the same file could cause conflicts
  - **Mitigation**: Queue deduplication (INC_INDEX-3001) should prevent this; use row-level locking if needed

- **Risk**: Parser failures could fail entire transaction
  - **Mitigation**: Catch parser errors, log to dead letter queue, and continue processing other files

## Files/Packages Affected
- `crates/maproom/src/incremental/processor.rs` - New file: IncrementalProcessor implementation
- `crates/maproom/src/incremental/edge_updater.rs` - New file: EdgeUpdater implementation
- `crates/maproom/src/incremental/mod.rs` - Update: Add module exports
- `crates/maproom/tests/incremental/processor_test.rs` - New file: Unit tests for processor
- `crates/maproom/tests/incremental/edge_updater_test.rs` - New file: Unit tests for edge updater
- `crates/maproom/tests/integration/incremental_test.rs` - New file: Integration tests
- `crates/maproom/Cargo.toml` - Update: Add any new dependencies (if needed)
