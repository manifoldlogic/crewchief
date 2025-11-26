# Ticket: SQLFIX-1002: Fix Schema Initialization

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Verification is manual schema inspection using `sqlite3` CLI or in-memory test
- Full test coverage comes in SQLFIX-1004

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix the SQLite schema initialization to align with the ChunkRecord struct. Add the missing `ts_doc_text` column to the chunks table.

## Background
The SQLite schema in `schema.rs` is missing the `ts_doc_text` column that exists in the `ChunkRecord` struct. This will cause runtime INSERT failures when `insert_chunk` tries to insert data.

**Current ChunkRecord struct** (db/mod.rs lines 55-70):
```rust
pub struct ChunkRecord {
    pub file_id: i64,
    pub blob_sha: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
    pub preview: String,
    pub ts_doc_text: String,        // NOT in schema!
    pub recency_score: f32,
    pub churn_score: f32,
    pub metadata: Option<serde_json::Value>,
    pub worktree_id: i64,
}
```

**Current insert_chunk code** (sqlite/mod.rs lines 201-213) does NOT insert `ts_doc_text` - this is fine since it's not in the schema, but the struct has it, suggesting it should be stored.

**Plan Reference**: Phase 2 - Runtime Functionality (Ticket 1002)

## Acceptance Criteria
- [ ] `chunks` table includes `ts_doc_text TEXT` column
- [ ] `migrate()` is idempotent (safe to call multiple times)
- [ ] Schema can be created in `:memory:` SQLite without errors
- [ ] FTS5 table columns match what `insert_chunk` provides

## Technical Requirements

### 1. Current Schema (schema.rs lines 58-77)
```sql
CREATE TABLE IF NOT EXISTS chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    blob_sha TEXT NOT NULL,
    symbol_name TEXT,
    kind TEXT NOT NULL,
    signature TEXT,
    docstring TEXT,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    preview TEXT NOT NULL,
    recency_score REAL NOT NULL,
    churn_score REAL NOT NULL,
    metadata JSON,
    worktree_ids JSON NOT NULL,
    UNIQUE(file_id, start_line, end_line)
)
```

### 2. Required Schema Change
Add `ts_doc_text` column after `preview`:
```sql
CREATE TABLE IF NOT EXISTS chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    blob_sha TEXT NOT NULL,
    symbol_name TEXT,
    kind TEXT NOT NULL,
    signature TEXT,
    docstring TEXT,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    preview TEXT NOT NULL,
    ts_doc_text TEXT,              -- ADD THIS LINE
    recency_score REAL NOT NULL,
    churn_score REAL NOT NULL,
    metadata JSON,
    worktree_ids JSON NOT NULL,
    UNIQUE(file_id, start_line, end_line)
)
```

### 3. FTS5 Table - No Changes Needed
The current FTS5 definition is correct:
```sql
CREATE VIRTUAL TABLE IF NOT EXISTS fts_chunks USING fts5(
    content,           -- Receives chunk.preview data
    docstring,         -- Receives chunk.docstring data
    symbol_name,       -- Receives chunk.symbol_name data
    content='chunks',
    content_rowid='id'
);
```

**Note**: The `content` column name is just a column name in the FTS table. The `insert_chunk` code correctly populates it with `chunk.preview`:
```rust
// From sqlite/mod.rs line 244
conn.execute(
    "INSERT OR REPLACE INTO fts_chunks(rowid, content, docstring, symbol_name) VALUES (?1, ?2, ?3, ?4)",
    params![id, chunk.preview, chunk.docstring, chunk.symbol_name],
)?;
```

### 4. Update insert_chunk to Include ts_doc_text
After fixing the schema, update `insert_chunk` (lines 201-230) to include `ts_doc_text`:

```rust
conn.execute(
    "INSERT INTO chunks (
       file_id, blob_sha, symbol_name, kind, signature, docstring,
       start_line, end_line, preview, ts_doc_text, recency_score,
       churn_score, metadata, worktree_ids
     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
     ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
       blob_sha = excluded.blob_sha,
       symbol_name = excluded.symbol_name,
       kind = excluded.kind,
       signature = excluded.signature,
       docstring = excluded.docstring,
       preview = excluded.preview,
       ts_doc_text = excluded.ts_doc_text,
       metadata = excluded.metadata,
       worktree_ids = json_insert(chunks.worktree_ids, '$[#]', ?15)
     ",
    params![
        chunk.file_id,
        chunk.blob_sha,
        chunk.symbol_name,
        chunk.kind,
        chunk.signature,
        chunk.docstring,
        chunk.start_line,
        chunk.end_line,
        chunk.preview,
        chunk.ts_doc_text,        // ADD
        chunk.recency_score,
        chunk.churn_score,
        metadata_json,
        worktree_ids_json,
        chunk.worktree_id
    ],
)?;
```

### 5. Ensure Idempotency
All CREATE statements already use `IF NOT EXISTS`:
- `CREATE TABLE IF NOT EXISTS`
- `CREATE VIRTUAL TABLE IF NOT EXISTS`

Verify indices also use `IF NOT EXISTS` if any are added.

## Implementation Notes

### Verification Steps
```bash
# After fixing schema.rs, verify schema creates correctly
cargo build --features sqlite

# Test schema creation in memory (after SQLFIX-1003 is done)
cargo test --features sqlite test_connect_and_migrate
```

### Schema Inspection
```bash
# View current schema definition
cat crates/maproom/src/db/sqlite/schema.rs
```

### Column Alignment Check
| ChunkRecord Field | Schema Column | Status |
|-------------------|---------------|--------|
| file_id | file_id | ✅ |
| blob_sha | blob_sha | ✅ |
| symbol_name | symbol_name | ✅ |
| kind | kind | ✅ |
| signature | signature | ✅ |
| docstring | docstring | ✅ |
| start_line | start_line | ✅ |
| end_line | end_line | ✅ |
| preview | preview | ✅ |
| ts_doc_text | (missing) | ❌ ADD |
| recency_score | recency_score | ✅ |
| churn_score | churn_score | ✅ |
| metadata | metadata | ✅ |
| worktree_id | worktree_ids (JSON array) | ✅ (mapped) |

## Dependencies
- **SQLFIX-1001**: SQLite compilation must pass first

## Risk Assessment
- **Risk**: Schema changes may not apply to existing databases
  - **Mitigation**: This is MVP; document that existing SQLite DBs should be deleted and recreated
- **Risk**: Adding column could break existing queries
  - **Mitigation**: Column is nullable (`TEXT` without `NOT NULL`), so existing rows would have NULL

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/schema.rs`
- `crates/maproom/src/db/sqlite/mod.rs` (insert_chunk function)
