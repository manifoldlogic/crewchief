# Ticket: VECSTORE-1005: Index State Management Methods

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

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
Add index state tracking methods to the `VectorStore` trait: `get_last_indexed_tree()` and `update_index_state()`. These enable tracking what has been indexed for incremental updates.

## Background
The indexer needs to track the last indexed git tree SHA to support incremental indexing. PostgreSQL already has these functions in `index_state.rs`, but they need to be exposed through the trait and implemented for SQLite.

**Current State**:
- PostgreSQL: `db/index_state.rs` has `get_last_indexed_tree()` and `update_index_state()` - **EXISTS**
- SQLite: No index state tracking - must be added
- Trait: No index state methods defined

**Reference**: Plan Phase 4 - Index State Methods (VECSTORE-1005)

## Acceptance Criteria
- [ ] `get_last_indexed_tree()` method added to trait and implemented
- [ ] `update_index_state()` method added to trait and implemented
- [ ] PostgresStore wraps existing `index_state.rs` queries
- [ ] SqliteStore has equivalent implementation
- [ ] State persists across connections
- [ ] Contract tests pass for both backends

## Technical Requirements

### Trait Method Signatures
Add to `VectorStore` trait in `crates/maproom/src/db/mod.rs`:

```rust
/// Get the last indexed git tree SHA for a worktree
async fn get_last_indexed_tree(&self, worktree_id: i64) -> anyhow::Result<Option<String>>;

/// Update index state after successful indexing
async fn update_index_state(
    &self,
    worktree_id: i64,
    tree_sha: &str,
    files_indexed: i64,
    chunks_indexed: i64,
) -> anyhow::Result<()>;
```

### PostgreSQL Implementation (wrap existing)

**File: `crates/maproom/src/db/postgres/mod.rs`**

The PostgreSQL queries already exist in `db/index_state.rs`. Wrap them:

```rust
async fn get_last_indexed_tree(&self, worktree_id: i64) -> anyhow::Result<Option<String>> {
    let client = self.pool.get().await?;
    super::index_state::get_last_indexed_tree(&client, worktree_id).await
}

async fn update_index_state(
    &self,
    worktree_id: i64,
    tree_sha: &str,
    files_indexed: i64,
    chunks_indexed: i64,
) -> anyhow::Result<()> {
    let client = self.pool.get().await?;
    super::index_state::update_index_state(&client, worktree_id, tree_sha, files_indexed, chunks_indexed).await
}
```

### SQLite Implementation (NEW)

**Schema Addition** (`crates/maproom/src/db/sqlite/schema.rs`):

```sql
CREATE TABLE IF NOT EXISTS index_state (
    id INTEGER PRIMARY KEY,
    worktree_id INTEGER NOT NULL UNIQUE,
    tree_sha TEXT NOT NULL,
    files_indexed INTEGER NOT NULL DEFAULT 0,
    chunks_indexed INTEGER NOT NULL DEFAULT 0,
    last_indexed TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (worktree_id) REFERENCES worktrees(id)
);
```

**Query Functions** (`crates/maproom/src/db/sqlite/index_state.rs` - NEW):

```rust
pub fn get_last_indexed_tree(conn: &Connection, worktree_id: i64) -> anyhow::Result<Option<String>> {
    let result = conn.query_row(
        "SELECT tree_sha FROM index_state WHERE worktree_id = ?",
        [worktree_id],
        |row| row.get(0),
    );

    match result {
        Ok(sha) => Ok(Some(sha)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn update_index_state(
    conn: &Connection,
    worktree_id: i64,
    tree_sha: &str,
    files_indexed: i64,
    chunks_indexed: i64,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO index_state (worktree_id, tree_sha, files_indexed, chunks_indexed, last_indexed)
         VALUES (?, ?, ?, ?, datetime('now'))
         ON CONFLICT(worktree_id) DO UPDATE SET
             tree_sha = excluded.tree_sha,
             files_indexed = excluded.files_indexed,
             chunks_indexed = excluded.chunks_indexed,
             last_indexed = excluded.last_indexed",
        rusqlite::params![worktree_id, tree_sha, files_indexed, chunks_indexed],
    )?;
    Ok(())
}
```

**Store Implementation** (`crates/maproom/src/db/sqlite/mod.rs`):

```rust
async fn get_last_indexed_tree(&self, worktree_id: i64) -> anyhow::Result<Option<String>> {
    self.run(move |conn| index_state::get_last_indexed_tree(conn, worktree_id)).await
}

async fn update_index_state(
    &self,
    worktree_id: i64,
    tree_sha: &str,
    files_indexed: i64,
    chunks_indexed: i64,
) -> anyhow::Result<()> {
    let tree_sha = tree_sha.to_string();
    self.run(move |conn| {
        index_state::update_index_state(conn, worktree_id, &tree_sha, files_indexed, chunks_indexed)
    }).await
}
```

## Implementation Notes

### PostgreSQL index_state.rs
Review the existing `db/index_state.rs` to understand:
- Exact function signatures
- Table schema used
- Any additional fields tracked

The trait methods may need to align with existing PostgreSQL implementation.

### Persistence Testing
Index state must persist across connections:
1. Insert state
2. Close connection
3. Reopen connection
4. Verify state still present

### Migration
SQLite needs a migration to add the `index_state` table. Add to the migration sequence in `sqlite/migrations.rs` or `sqlite/schema.rs`.

## Dependencies
- None - Index state is independent of search methods

## Risk Assessment
- **Risk**: PostgreSQL index_state.rs has different signature
  - **Mitigation**: Review existing code, adapt trait signature if needed
- **Risk**: SQLite schema conflicts with existing tables
  - **Mitigation**: Check for existing index_state table, use IF NOT EXISTS

## Files/Packages Affected
- `crates/maproom/src/db/mod.rs` (trait methods)
- `crates/maproom/src/db/postgres/mod.rs` (PostgresStore impl)
- `crates/maproom/src/db/sqlite/mod.rs` (SqliteStore impl)
- `crates/maproom/src/db/sqlite/schema.rs` (table DDL)
- `crates/maproom/src/db/sqlite/index_state.rs` (NEW - SQLite queries)
