# Ticket: MAPCLI-1002: Refactor Daemon to use VectorStore Trait

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Replace `PgPool` in DaemonState with `Arc<dyn VectorStore>` and remove all raw SQL queries from the daemon, using VectorStore trait methods instead. This enables the daemon to work with both PostgreSQL and SQLite backends.

## Background
The daemon currently uses `PgPool` (deadpool-postgres pool) and executes raw SQL queries for search operations. This tightly couples the daemon to PostgreSQL. By replacing `PgPool` with `Arc<dyn VectorStore>` and using trait methods, the daemon can serve JSON-RPC requests regardless of the underlying database backend.

The daemon's `execute_search()` function (lines 108-150) contains raw queries for repo lookup and chunk details that must be replaced with trait methods. The `SearchHit` struct returned by trait methods already contains all necessary fields.

**Plan Reference**: Phase 1: Daemon Refactoring (MAPCLI-1002) in plan.md

## Acceptance Criteria
- [ ] `DaemonState` struct uses `Arc<dyn VectorStore>` instead of `PgPool`
- [ ] `run()` function uses `get_store()` to initialize the store
- [ ] `ping` RPC returns "pong" with both backends
- [ ] `search` RPC (fts mode) returns results from SQLite
- [ ] `search` RPC (vector mode) returns results from SQLite
- [ ] `search` RPC (hybrid mode) returns results from SQLite
- [ ] **No raw SQL queries remain in daemon module** - all use trait methods
- [ ] Daemon works correctly with PostgreSQL backend (no regression)
- [ ] Proper error handling for missing backends or invalid queries

## Technical Requirements
- Replace `PgPool` with `Arc<dyn VectorStore>` in DaemonState
- Use `get_store()` factory in `run()` initialization
- Replace all `client.query_one()` and `client.query_opt()` calls with trait methods
- Use `store.search_chunks_fts()`, `store.search_chunks_vector()`, `store.search_chunks_hybrid()` for search operations
- Handle all search modes: "fts" (default), "vector", "hybrid"
- Format search results using SearchHit fields

## Implementation Notes

### Step 1: Update DaemonState struct
```rust
// Before (daemon/mod.rs)
struct DaemonState {
    pool: PgPool,
    embedding_service: EmbeddingService,
}

// After
struct DaemonState {
    store: Arc<dyn VectorStore>,
    embedding_service: EmbeddingService,
}
```

### Step 2: Update run() function
```rust
pub async fn run() -> Result<()> {
    // Before
    let pool = db::create_pool().await?;

    // After
    let store = db::factory::get_store().await?;
    let embedding_service = EmbeddingService::from_env().await?;
    let state = Arc::new(DaemonState { store, embedding_service });
    // ... rest unchanged
}
```

### Step 3: Replace execute_search() raw queries
```rust
// Before (lines 108-150)
let repo_row = client.query_one(
    "SELECT id FROM maproom.repos WHERE name = $1",
    &[&params.repo],
)?;
let chunk_row = client.query_opt(
    r#"SELECT c.start_line, c.end_line, ... FROM maproom.chunks c ..."#,
)?;

// After
async fn execute_search(state: Arc<DaemonState>, params: SearchParams) -> Result<Value> {
    let hits = match params.mode.as_deref() {
        Some("fts") | None => {
            state.store.search_chunks_fts(
                &params.repo,
                params.worktree.as_deref(),
                &params.query,
                params.limit.unwrap_or(10) as i64,
                false  // debug flag
            ).await?
        }
        Some("vector") => {
            state.store.search_chunks_vector(
                &params.repo,
                params.worktree.as_deref(),
                &params.query,
                params.limit.unwrap_or(10) as i64,
            ).await?
        }
        Some("hybrid") => {
            state.store.search_chunks_hybrid(
                &params.repo,
                params.worktree.as_deref(),
                &params.query,
                params.limit.unwrap_or(10) as i64,
            ).await?
        }
        Some(mode) => anyhow::bail!("Unknown search mode: {}", mode),
    };

    format_search_response(hits)
}
```

### Step 4: Format response using SearchHit
The `SearchHit` struct already contains all needed fields:
- `chunk_id`, `file_path`, `start_line`, `end_line`
- `symbol_name`, `kind`, `content`
- `score`

```rust
fn format_search_response(hits: Vec<SearchHit>) -> Result<Value> {
    let results: Vec<Value> = hits.iter().map(|hit| {
        json!({
            "chunk_id": hit.chunk_id,
            "file_path": hit.file_path,
            "start_line": hit.start_line,
            "end_line": hit.end_line,
            "symbol_name": hit.symbol_name,
            "kind": hit.kind,
            "content": hit.content,
            "score": hit.score,
        })
    }).collect();

    Ok(json!({ "hits": results }))
}
```

### Key Considerations
- The daemon needs to handle the case where embedding service is unavailable for vector search
- Error messages should be informative for debugging
- Connection pooling is handled internally by VectorStore implementations

## Dependencies
- **MAPCLI-1000**: BackendType enum must exist (for potential backend-specific error messages)
- Can be developed in parallel with MAPCLI-1003 (Detection)

## Risk Assessment
- **Risk**: Breaking existing PostgreSQL daemon functionality
  - **Mitigation**: Test with PostgreSQL backend before and after changes
- **Risk**: Search result format changes breaking clients
  - **Mitigation**: Keep same JSON response structure; SearchHit has all needed fields
- **Risk**: Performance regression from trait method overhead
  - **Mitigation**: Dynamic dispatch overhead is negligible for I/O-bound database operations

## Files/Packages Affected
- `crates/maproom/src/daemon/mod.rs` - Primary modification target
  - Update DaemonState struct
  - Update run() to use get_store()
  - Refactor execute_search() to use trait methods
  - Remove raw SQL queries
- `crates/maproom/src/daemon/types.rs` - Likely no changes needed (request/response types)

## Testing
```bash
# Start daemon with PostgreSQL
MAPROOM_DATABASE_URL="postgresql://..." cargo run --bin crewchief-maproom -- serve &

# Test ping
echo '{"jsonrpc":"2.0","method":"ping","id":1}' | nc localhost 9999

# Test search (fts mode)
echo '{"jsonrpc":"2.0","method":"search","params":{"repo":"test","query":"function"},"id":2}' | nc localhost 9999

# Test search (vector mode)
echo '{"jsonrpc":"2.0","method":"search","params":{"repo":"test","query":"function","mode":"vector"},"id":3}' | nc localhost 9999

# Start daemon with SQLite
MAPROOM_DATABASE_URL="sqlite:///tmp/test.db" cargo run --features sqlite --bin crewchief-maproom -- serve &

# Test same operations with SQLite backend
echo '{"jsonrpc":"2.0","method":"ping","id":1}' | nc localhost 9999
echo '{"jsonrpc":"2.0","method":"search","params":{"repo":"test","query":"function"},"id":2}' | nc localhost 9999

# Run all tests
cargo test
cargo test --features sqlite
```
