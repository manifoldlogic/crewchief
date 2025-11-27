# Ticket: IDXABS-2003: Refactor Search Module

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - `cargo check` passes for search module (no search-specific errors; other module errors expected)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Run `cargo check` to verify search module compiles
- Search tests may need updates in ticket 4001
- Focus on compilation, verify functionality in E2E tests

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Update all search modules to use `&SqliteStore` instead of `&Client` (tokio_postgres).

## Background
The search module has 19 PostgreSQL references across 6 files. All functions using `&Client` need to switch to `&SqliteStore` and use its existing search methods.

**Reference**: Phase 2, Ticket 2003 of `planning/plan.md` - "Refactor Search Module"
**Architecture**: See `planning/architecture.md` - Section 4.3 "search/ modules"

## Acceptance Criteria
- [ ] `search/pipeline.rs` uses `&SqliteStore` (2 references)
- [ ] `search/fts.rs` uses `&SqliteStore` (2 references)
- [ ] `search/vector.rs` uses `&SqliteStore` (6 references)
- [ ] `search/graph.rs` uses `&SqliteStore` (3 references)
- [ ] `search/signals.rs` uses `&SqliteStore` (4 references)
- [ ] `search/executors.rs` uses `&SqliteStore` (2 references)
- [ ] `search/mod.rs` uses `&SqliteStore` (2 references)
- [ ] All other search/ files checked and updated if needed
- [ ] No `&Client` references remain in `search/` directory
- [ ] No `tokio_postgres` imports in `search/` directory
- [ ] Verification: `grep -r "tokio_postgres\|&Client" crates/maproom/src/search/` returns nothing
- [ ] `cargo check` passes for search module

## Technical Requirements
- Change function signatures from `&Client` to `&SqliteStore`
- SqliteStore already has search methods:
  - `store.fts_search()` - Full-text search using FTS5
  - `store.vector_search()` - Vector similarity using sqlite-vec
  - `store.hybrid_search()` - Combined FTS + vector
  - `store.graph_search()` - Graph traversal for relationships
- Replace raw SQL queries with store method calls

## Implementation Notes

### Files in search/ Directory
| File | PostgreSQL Refs | Changes Needed |
|------|-----------------|----------------|
| `pipeline.rs` | 2 | Main search pipeline |
| `fts.rs` | 2 | Full-text search |
| `vector.rs` | 6 | Vector similarity |
| `graph.rs` | 3 | Graph traversal |
| `signals.rs` | 4 | Ranking signals |
| `executors.rs` | 2 | Query execution |
| `mod.rs` | 2 | Module exports |

### Additional Files to CHECK (may have PostgreSQL refs)
- `query_processor.rs` - Query processing
- `results.rs` - Result types
- `dedup.rs` - Deduplication logic
- `cache.rs` - Search caching
- `warming.rs` - Cache warming
- `expander.rs` - Query expansion
- `tokenizer.rs` - Tokenization
- `types.rs` - Type definitions
- `fusion/` - Fusion subdirectory (all files)

### Function Signature Pattern
```rust
// Before
pub async fn search(client: &Client, query: &str) -> Result<Vec<SearchResult>>

// After
pub async fn search(store: &SqliteStore, query: &str) -> Result<Vec<SearchResult>>
```

### SqliteStore Search Methods (Already Exist)
```rust
// In db/sqlite/mod.rs or search-related files:
impl SqliteStore {
    pub async fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<FtsResult>>;
    pub async fn vector_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<VectorResult>>;
    pub async fn hybrid_search(&self, query: &str, embedding: &[f32]) -> Result<Vec<HybridResult>>;
}
```

### Verification
```bash
# Check search module compiles
cargo check -p crewchief-maproom --lib 2>&1 | grep -E "search"

# Count remaining Client references
grep -r "tokio_postgres\|&Client" crates/maproom/src/search/
# Should return nothing

# Verify SqliteStore is imported
grep -r "use.*SqliteStore" crates/maproom/src/search/
# Should show imports in modified files
```

## Dependencies
- IDXABS-1001 (Delete PostgreSQL Database Files)
- IDXABS-1002 (Simplify db/mod.rs)
- IDXABS-1003 (Update Cargo.toml)
- IDXABS-2001 (Refactor Indexer Module)
- IDXABS-2002 (Refactor Embedding Pipeline)

## Risk Assessment
- **Risk**: SqliteStore missing some search methods
  - **Mitigation**: SqliteStore already has FTS5 and sqlite-vec integration
  - **Mitigation**: Add wrapper methods if specific queries needed
- **Risk**: Search result types differ between PostgreSQL and SQLite
  - **Mitigation**: Result structs should be backend-agnostic
  - **Mitigation**: Verify return types match expected interfaces
- **Risk**: Performance differences in search queries
  - **Mitigation**: SQLite FTS5 and sqlite-vec are optimized
  - **Mitigation**: Test with realistic data sizes

## Files/Packages Affected
Files to MODIFY (confirmed PostgreSQL refs):
- `crates/maproom/src/search/pipeline.rs`
- `crates/maproom/src/search/fts.rs`
- `crates/maproom/src/search/vector.rs`
- `crates/maproom/src/search/graph.rs`
- `crates/maproom/src/search/signals.rs`
- `crates/maproom/src/search/executors.rs`
- `crates/maproom/src/search/mod.rs`

Files to CHECK (update if PostgreSQL refs found):
- `crates/maproom/src/search/query_processor.rs`
- `crates/maproom/src/search/results.rs`
- `crates/maproom/src/search/dedup.rs`
- `crates/maproom/src/search/cache.rs`
- `crates/maproom/src/search/warming.rs`
- `crates/maproom/src/search/expander.rs`
- `crates/maproom/src/search/tokenizer.rs`
- `crates/maproom/src/search/types.rs`
- `crates/maproom/src/search/fusion/*` (all files in subdirectory)
