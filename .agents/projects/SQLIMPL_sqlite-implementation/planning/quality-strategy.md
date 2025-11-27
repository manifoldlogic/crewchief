# Quality Strategy: SQLite Implementation Completion

## Testing Philosophy

This project inherits 35 test files that reference PostgreSQL and don't compile. The strategy is pragmatic:

1. **Discover existing code first** - Audit SqliteStore methods before implementing
2. **Migrate tests first** - Tests become the validation for implementation
3. **Focus on behavior, not coverage** - Tests should verify functionality works
4. **Integration over unit** - Most stubs need database interactions to test properly

## Pre-Implementation Discovery

Before implementing any stub, verify existing functionality:

1. **Check SqliteStore methods** - `src/db/sqlite/mod.rs` has working implementations
2. **Check helper modules** - `src/db/sqlite/fts.rs`, `vector.rs`, `graph.rs`
3. **Delegate, don't reimplement** - Call existing methods, convert types

**Already Implemented (just wire up):**
- `search_chunks_fts()` - FTS5 search
- `search_chunks_vector()` - Vector similarity
- `find_callers()`, `find_callees()` - Graph traversal
- `normalize_fts_rank()`, `distance_to_similarity()` - Score normalization

## Test Categories

### Category 1: Test File Migration (35 files)

These tests currently reference PostgreSQL types (`tokio_postgres`, `PgPool`, `postgres::`) and must be migrated to SQLite.

**Migration Pattern:**
```rust
// Before (PostgreSQL)
use tokio_postgres::Client;
async fn setup() -> Client {
    let pool = PgPool::connect(&url).await?;
    // ...
}

// After (SQLite)
use rusqlite::Connection;
fn setup() -> Connection {
    let conn = Connection::open_in_memory()?;
    SqliteStore::run_migrations(&conn)?;
    conn
}
```

**Files to Migrate:**
| File | Test Focus | Migration Complexity |
|------|------------|---------------------|
| `common/mod.rs` | Test fixtures | High (foundation) |
| `ab_testing_test.rs` | A/B testing | Low |
| `dynamic_worktree_id_test.rs` | Worktree IDs | Low |
| `e2e_workflow_simple.rs` | End-to-end | Medium |
| `e2e_multi_provider.rs` | Multi-provider | Medium |
| `embedding_inheritance_test.rs` | Embeddings | Low |
| `fusion_integration_test.rs` | Search fusion | Medium |
| `fusion_quality_test.rs` | Fusion quality | Medium |
| `graph_test.rs` | Graph queries | High |
| `incremental_*.rs` (7 files) | Incremental | Medium |
| `index_state.rs` | Index state | Low |
| `integration/*.rs` (4 files) | Integration | Medium |
| `migration_*.rs` (2 files) | Migrations | Low |
| `python_pipeline_test.rs` | Python parsing | Medium |
| `relationship_test.rs` | Relationships | Medium |
| `rrf_fusion_test.rs` | RRF fusion | Low |
| `search_*.rs` (2 files) | Search | High |
| `signal_integration_test.rs` | Signals | Medium |
| `store_compat.rs` | Store compat | Medium |
| `unified_watch_test.rs` | Watch | High |
| `upsert_worktree.rs` | Upsert | Low |
| `vector_db_test.rs` | Vector DB | High |
| `watch_integration.rs` | Watch | High |
| `weighted_fusion_test.rs` | Fusion | Low |

### Category 2: Implementation Validation

After stubs are implemented, tests should validate:

**Search Executors:**
- FTS returns ranked results for keyword queries
- Vector search returns semantically similar chunks
- Graph importance propagates through edges
- Signals apply recency/churn scoring

**Context Assembly:**
- Cache stores and retrieves contexts
- Graph traversal finds related chunks
- Language detectors identify patterns
- Strategies expand context appropriately

**Incremental Updates:**
- File hashes are stored and retrieved
- Changed files are detected
- Chunks are updated on file changes
- Edges are recomputed after changes

### Category 3: Integration Tests

End-to-end workflows that should pass:

```bash
# Index a repository
cargo run --bin crewchief-maproom -- scan --path ./test-repo

# Search returns results
cargo run --bin crewchief-maproom -- search "function" | grep -q "results"

# Modify a file, upsert, verify update
echo "// new content" >> ./test-repo/file.rs
cargo run --bin crewchief-maproom -- upsert --paths ./test-repo/file.rs

# Watch command starts without error
cargo run --bin crewchief-maproom -- watch --repo test-repo &
sleep 2 && kill %1  # Should run for 2 seconds without crashing
```

## Test Infrastructure

### In-Memory SQLite

All tests should use in-memory SQLite for speed:

```rust
fn test_db() -> SqliteStore {
    let conn = Connection::open_in_memory().unwrap();
    SqliteStore::run_migrations(&conn).unwrap();
    SqliteStore::new(conn)
}
```

### Test Fixtures

Create reusable fixtures:

```rust
mod fixtures {
    pub fn sample_chunks() -> Vec<Chunk> { /* ... */ }
    pub fn sample_embeddings() -> Vec<Embedding> { /* ... */ }
    pub fn sample_edges() -> Vec<Edge> { /* ... */ }
}
```

### Assertion Helpers

```rust
fn assert_non_empty_results(results: &RankedResults) {
    assert!(!results.is_empty(), "Expected non-empty search results");
}

fn assert_scores_descending(results: &RankedResults) {
    let scores: Vec<_> = results.iter().map(|r| r.score).collect();
    let sorted = scores.clone().into_iter().sorted().rev().collect::<Vec<_>>();
    assert_eq!(scores, sorted, "Results should be sorted by score descending");
}
```

## Quality Gates

### Phase 1: Test Migration
**Gate:** `cargo test -p crewchief-maproom` compiles without errors

### Phase 2: Search Wiring (Low Complexity)
**Gate:** Search tests pass, including:
- `fts_returns_results`
- `vector_returns_similar`
- `graph_propagates_importance`
- `signals_apply_scoring`

### Phase 3: Incremental Updates
**Gate:** Incremental tests pass, including:
- `detects_file_changes`
- `stores_file_hashes`
- `updates_chunks_on_change`
- `removes_deleted_file_chunks`

### Phase 4: Context Assembly (OPTIONAL)
**Gate:** Context tests pass, including:
- `cache_stores_and_retrieves`
- `graph_finds_related`
- `language_detection_works`

**Note:** This phase is optional for MVP. Context assembly may require tree-sitter integration, not just SQL queries. Some detectors (JSX, hooks) need AST analysis. Defer if timeline pressure.

### Phase 5: Watch Command
**Gate:** Watch integration tests pass:
- `watch_starts_without_error`
- `watch_detects_file_changes`

## Risk Mitigation

### Risk: Test Migration Reveals Missing Features
**Mitigation:** Triage tests first (Phase 1 Ticket 1001). Classify as migrate/delete/defer. Track discovered issues, don't block migration.

### Risk: Reimplementing Existing Code
**Mitigation:** Audit SqliteStore methods before implementing. Use delegation pattern.

### Risk: Tests Brittle After Migration
**Mitigation:** Use in-memory databases, avoid time-sensitive assertions.

### Risk: Implementation Passes Tests But Fails in Practice
**Mitigation:** Include integration tests with real filesystem operations.

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test pass rate | 100% | `cargo test` exit code |
| Compilation warnings | 0 related to stubs | `cargo check 2>&1 | grep TODO` |
| Search result quality | Non-empty, ranked | Manual verification |
| Incremental efficiency | <1s for single file | Timing in tests |

## Out of Scope

- Performance benchmarking (future project)
- Fuzzing or property-based testing
- Coverage percentage targets
- Load testing
