# Ticket: VECSTORE-1003: Context Assembly Methods

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - db tests: 129 passed
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
Add chunk context retrieval methods to the `VectorStore` trait: `get_chunk_by_id()`, `get_file_chunks()`, and `get_chunk_context()`. These enable retrieving full chunk data and surrounding context for display.

## Background
Context assembly is needed for displaying search results with surrounding code context. The existing `context/assembler.rs` has sophisticated context assembly logic (400+ lines), but the basic data retrieval methods are not in the trait.

**Approach**: Implement simplified context methods in the trait; keep the sophisticated `ContextAssembler` as higher-level code that uses these trait methods.

**Current State**:
- PostgreSQL: **NO** dedicated context functions in `queries.rs` - must be written
- SQLite: Some chunk retrieval exists but not standardized
- Trait: No context methods defined

**Reference**: Plan Phase 2 - Context Assembly Methods (VECSTORE-1003)

## Acceptance Criteria
- [ ] `ChunkFull`, `ChunkSummary`, `ChunkContext` types defined in `db/mod.rs`
- [ ] `get_chunk_by_id()` method added to trait and implemented
- [ ] `get_file_chunks()` method added to trait and implemented
- [ ] `get_chunk_context()` method added to trait and implemented
- [ ] PostgreSQL query functions written in `queries.rs`
- [ ] Both `PostgresStore` and `SqliteStore` implementations work
- [ ] `get_chunk_context` returns surrounding chunks (by line number)
- [ ] Contract tests pass for both backends

## Technical Requirements

### Domain Types
Add to `crates/maproom/src/db/mod.rs`:

```rust
/// Full chunk data for display/context - read-only view of a chunk
pub struct ChunkFull {
    pub id: i64,
    pub file_id: i64,
    pub blob_sha: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
    pub preview: String,
    pub content: String,      // Full chunk content
    pub file_path: String,    // Denormalized from file table
    pub worktree_id: i64,
}

/// Lightweight chunk reference for lists/navigation
pub struct ChunkSummary {
    pub id: i64,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub file_path: String,
}

/// Context around a chunk - surrounding and related chunks
pub struct ChunkContext {
    pub chunk: ChunkFull,
    pub file_path: String,
    pub surrounding_chunks: Vec<ChunkSummary>,  // Chunks before/after by line number
    pub related_chunks: Vec<ChunkSummary>,      // Future: chunks related by edges
}
```

### Trait Method Signatures
Add to `VectorStore` trait:

```rust
/// Get a single chunk by ID with full content
async fn get_chunk_by_id(&self, chunk_id: i64) -> anyhow::Result<Option<ChunkFull>>;

/// Get all chunks for a file
async fn get_file_chunks(&self, file_id: i64) -> anyhow::Result<Vec<ChunkSummary>>;

/// Get chunk with surrounding context (N chunks before/after by line number)
async fn get_chunk_context(&self, chunk_id: i64, surrounding: usize) -> anyhow::Result<Option<ChunkContext>>;
```

### PostgreSQL Queries (NEW - must be written)

**File: `crates/maproom/src/db/queries.rs`**

```rust
pub async fn get_chunk_by_id(
    client: &impl GenericClient,
    chunk_id: i64,
) -> anyhow::Result<Option<ChunkFull>> {
    // SELECT c.*, f.relpath as file_path
    // FROM chunks c
    // JOIN files f ON c.file_id = f.id
    // WHERE c.id = $1
}

pub async fn get_file_chunks(
    client: &impl GenericClient,
    file_id: i64,
) -> anyhow::Result<Vec<ChunkSummary>> {
    // SELECT id, symbol_name, kind, start_line, end_line, relpath
    // FROM chunks c
    // JOIN files f ON c.file_id = f.id
    // WHERE c.file_id = $1
    // ORDER BY start_line
}

pub async fn get_chunk_context(
    client: &impl GenericClient,
    chunk_id: i64,
    surrounding: usize,
) -> anyhow::Result<Option<ChunkContext>> {
    // 1. Get the target chunk
    // 2. Get file_id from chunk
    // 3. Get surrounding chunks by line number:
    //    SELECT * FROM chunks
    //    WHERE file_id = $file_id
    //      AND (end_line < $chunk_start_line OR start_line > $chunk_end_line)
    //    ORDER BY ABS(start_line - $chunk_start_line)
    //    LIMIT $surrounding * 2
}
```

### SQLite Implementation

Create equivalent functions in `sqlite/` module (new file or add to existing):

```rust
// crates/maproom/src/db/sqlite/context.rs (new file)

pub fn get_chunk_by_id(conn: &Connection, chunk_id: i64) -> anyhow::Result<Option<ChunkFull>> {
    // Same query logic as PostgreSQL
}

pub fn get_file_chunks(conn: &Connection, file_id: i64) -> anyhow::Result<Vec<ChunkSummary>> {
    // Same query logic as PostgreSQL
}

pub fn get_chunk_context(
    conn: &Connection,
    chunk_id: i64,
    surrounding: usize,
) -> anyhow::Result<Option<ChunkContext>> {
    // Same logic as PostgreSQL
}
```

## Implementation Notes

### Simplified Context vs Full ContextAssembler
This ticket implements **simplified** context retrieval:
- `get_chunk_context()` returns N chunks before/after by line number
- Does NOT implement: strategy patterns, token budgets, semantic expansion
- The existing `context/assembler.rs` can use these trait methods internally

### `related_chunks` Field
For this ticket, `related_chunks` can be empty. Graph-based relationship traversal (callers/callees via `chunk_edges`) is a future enhancement.

### Type Relationships
- `ChunkRecord` → Write path (insertion)
- `ChunkFull` → Read path (full retrieval)
- `ChunkSummary` → Read path (lightweight lists)
- `SearchHit` → Search results (includes score)

### Content Field
`ChunkFull.content` contains the full source code of the chunk. This may need to be:
- Stored in `chunks.preview` (current)
- Or fetched from a separate content storage
- Check existing schema for content storage approach

## Dependencies
- None - Context methods are independent of search methods

## Risk Assessment
- **Risk**: Content field not available in current schema
  - **Mitigation**: Check schema, may need to use `preview` or add content column
- **Risk**: Performance with large files (many chunks)
  - **Mitigation**: Use LIMIT and indexed queries

## Files/Packages Affected
- `crates/maproom/src/db/mod.rs` (types + trait)
- `crates/maproom/src/db/queries.rs` (PostgreSQL queries)
- `crates/maproom/src/db/postgres/mod.rs` (PostgresStore impl)
- `crates/maproom/src/db/sqlite/mod.rs` (SqliteStore impl)
- `crates/maproom/src/db/sqlite/context.rs` (NEW - SQLite queries)
