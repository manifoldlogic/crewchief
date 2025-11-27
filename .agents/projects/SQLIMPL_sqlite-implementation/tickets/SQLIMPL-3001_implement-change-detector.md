# Ticket: SQLIMPL-3001: Implement Change Detector

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the change detector for file hash storage and retrieval. This enables incremental indexing by tracking which files have changed since the last index.

## Background
The change detector at `src/incremental/detector.rs` has 4 stubbed methods for hash operations. These are genuine new implementations - no existing SqliteStore methods exist for this functionality.

This ticket implements Plan Phase 3, Ticket 3001: "Implement Change Detector".

## Acceptance Criteria
- [ ] `get_hash_from_db()` retrieves file hash from `files` table
- [ ] `store_hash_in_db()` updates file hash in `files` table
- [ ] `detect_move()` identifies renamed files by comparing blob_sha
- [ ] Additional hash-related method implemented (4th method at line 453)
- [ ] Hash storage and retrieval work correctly in tests
- [ ] Move detection correctly identifies file renames

## Technical Requirements
- **Verify schema first:** Confirm `files` table has `content_hash` or `blob_sha` column
- Use `SqliteStore::run()` pattern for all database access
- Parameterized queries for security
- Handle missing files gracefully (return None, not error)

## Implementation Notes

### Current Stubs (to implement)
```rust
// src/incremental/detector.rs:309
// get_hash_from_db() - stub

// src/incremental/detector.rs:407
// store_hash_in_db() - stub

// src/incremental/detector.rs:437
// detect_move() - stub

// src/incremental/detector.rs:453
// 4th method - stub
```

### Schema Reference
```sql
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    repo_id INTEGER,
    relpath TEXT,
    blob_sha TEXT  -- Git blob SHA for content identification
);
```

### Target Implementation Patterns

#### Get Hash from DB
```rust
pub async fn get_hash_from_db(&self, file_id: i64) -> Result<Option<String>> {
    self.store.run(move |conn| {
        conn.query_row(
            "SELECT blob_sha FROM files WHERE id = ?",
            [file_id],
            |row| row.get(0)
        ).optional()
        .map_err(Into::into)
    }).await
}
```

#### Store Hash in DB
```rust
pub async fn store_hash_in_db(&self, file_id: i64, hash: &str) -> Result<()> {
    let hash = hash.to_string();
    self.store.run(move |conn| {
        conn.execute(
            "UPDATE files SET blob_sha = ? WHERE id = ?",
            params![hash, file_id]
        )?;
        Ok(())
    }).await
}
```

#### Detect Move
```rust
pub async fn detect_move(&self, blob_sha: &str, repo_id: i64) -> Result<Option<String>> {
    // Find if this blob_sha exists under a different path
    // Returns the old path if file was renamed
    let sha = blob_sha.to_string();
    self.store.run(move |conn| {
        conn.query_row(
            "SELECT relpath FROM files WHERE blob_sha = ? AND repo_id = ? LIMIT 1",
            params![sha, repo_id],
            |row| row.get(0)
        ).optional()
        .map_err(Into::into)
    }).await
}
```

## Dependencies
- Phase 1 Complete (tests compile)

## Risk Assessment
- **Risk**: Schema may not have expected columns
  - **Mitigation**: Verify schema before implementing; add column if needed
- **Risk**: Move detection may have edge cases (same content, different files)
  - **Mitigation**: Document behavior; move detection is best-effort

## Files/Packages Affected
- `crates/maproom/src/incremental/detector.rs` (primary)
