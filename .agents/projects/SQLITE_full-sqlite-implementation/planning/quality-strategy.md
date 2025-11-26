# Quality Strategy: Full SQLite Implementation

## Testing Philosophy

Focus on **confidence over coverage**. Tests should:
1. Prevent regressions on critical paths
2. Validate integration between components
3. Catch edge cases that cause data corruption
4. Be fast enough to run on every change

## Test Categories

### 1. Unit Tests (per module)

Each module gets focused unit tests:

#### crud.rs Tests
```rust
#[cfg(test)]
mod tests {
    // CRUD idempotency
    #[tokio::test]
    async fn test_repo_create_idempotent();
    #[tokio::test]
    async fn test_worktree_create_idempotent();
    #[tokio::test]
    async fn test_commit_create_idempotent();

    // File/chunk operations
    #[tokio::test]
    async fn test_upsert_file_insert();
    #[tokio::test]
    async fn test_upsert_file_update();
    #[tokio::test]
    async fn test_insert_chunk();
    #[tokio::test]
    async fn test_insert_chunks_batch();

    // Junction table
    #[tokio::test]
    async fn test_chunk_worktree_association();
    #[tokio::test]
    async fn test_chunk_multi_worktree();
}
```

#### fts.rs Tests
```rust
#[cfg(test)]
mod tests {
    // Query building
    #[test]
    fn test_build_fts_query_simple();
    #[test]
    fn test_build_fts_query_multi_word();
    #[test]
    fn test_build_fts_query_special_chars();
    #[test]
    fn test_build_fts_query_empty();

    // Search functionality
    #[tokio::test]
    async fn test_fts_search_basic();
    #[tokio::test]
    async fn test_fts_search_prefix();
    #[tokio::test]
    async fn test_fts_search_phrase();
    #[tokio::test]
    async fn test_fts_search_no_results();
    #[tokio::test]
    async fn test_fts_search_worktree_filter();
}
```

#### vector.rs Tests
```rust
#[cfg(test)]
mod tests {
    // Extension verification
    #[tokio::test]
    async fn test_sqlite_vec_extension_loaded();
    #[tokio::test]
    async fn test_extension_missing_graceful_degradation();

    // Vector search
    #[tokio::test]
    async fn test_vector_search_basic();
    #[tokio::test]
    async fn test_vector_search_similarity_ordering();
    #[tokio::test]
    async fn test_vector_search_empty_index();
}
```

#### hybrid.rs Tests
```rust
#[cfg(test)]
mod tests {
    // RRF fusion
    #[test]
    fn test_rrf_score_calculation();
    #[test]
    fn test_rrf_with_missing_fts();
    #[test]
    fn test_rrf_with_missing_vector();

    // Hybrid search
    #[tokio::test]
    async fn test_hybrid_search_basic();
    #[tokio::test]
    async fn test_hybrid_search_fts_only_fallback();
    #[tokio::test]
    async fn test_hybrid_weights();
}
```

#### embeddings.rs Tests
```rust
#[cfg(test)]
mod tests {
    // Deduplication
    #[tokio::test]
    async fn test_embedding_deduplication();
    #[tokio::test]
    async fn test_embedding_update_existing();
    #[tokio::test]
    async fn test_has_embedding();

    // Batch operations
    #[tokio::test]
    async fn test_batch_upsert_embeddings();
    #[tokio::test]
    async fn test_batch_with_duplicates();
}
```

#### graph.rs Tests
```rust
#[cfg(test)]
mod tests {
    // Graph traversal
    #[tokio::test]
    async fn test_find_callers_direct();
    #[tokio::test]
    async fn test_find_callers_transitive();
    #[tokio::test]
    async fn test_find_callees();
    #[tokio::test]
    async fn test_find_imports();
    #[tokio::test]
    async fn test_graph_cycle_handling();
}
```

#### migration.rs Tests
```rust
#[cfg(test)]
mod tests {
    // Migration system (fresh database only - no data migration needed)
    #[tokio::test]
    async fn test_migration_version_tracking();
    #[tokio::test]
    async fn test_migration_idempotent();
    #[tokio::test]
    async fn test_fresh_database_creation();
}
```

### 2. Integration Tests

Full pipeline tests in `tests/sqlite_integration.rs`:

#### File-Based Integration Test (REQUIRED)

At least one integration test MUST use a real temp file (not `:memory:`) to catch:
- File permission issues (0600 on Unix)
- WAL file handling and cleanup
- Path edge cases (spaces, unicode)
- Database recovery after crash

```rust
/// File-based integration test - catches issues :memory: misses
#[tokio::test]
async fn test_file_based_integration() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test maproom.db");  // intentional space

    // 1. Create store with file path
    let store = SqliteStore::connect(db_path.to_str().unwrap()).await.unwrap();
    store.migrate().await.unwrap();

    // 2. Insert data, close connection
    // ... insert test data ...
    drop(store);

    // 3. Verify WAL file exists during transaction
    // 4. Reopen and verify data persisted
    let store = SqliteStore::connect(db_path.to_str().unwrap()).await.unwrap();
    // ... verify data ...

    // 5. Verify file permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::metadata(&db_path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }
}

/// Unicode path handling
#[tokio::test]
async fn test_unicode_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("数据库.db");  // Chinese characters

    let store = SqliteStore::connect(db_path.to_str().unwrap()).await.unwrap();
    store.migrate().await.unwrap();
    // Verify basic operations work
}
```

#### Memory-Based Integration Tests

```rust
/// Complete indexing + search cycle
#[tokio::test]
async fn test_full_index_search_cycle() {
    // 1. Create repo/worktree/commit
    // 2. Index multiple files with chunks
    // 3. Generate and store embeddings
    // 4. Run hybrid search
    // 5. Verify results contain expected chunks
}

/// Multi-worktree scenario
#[tokio::test]
async fn test_multi_worktree_index() {
    // 1. Create repo with 2 worktrees
    // 2. Index same file in both
    // 3. Verify chunk appears in both worktree queries
    // 4. Verify single search across repo works
}

/// Embedding deduplication across files
#[tokio::test]
async fn test_embedding_dedup_cross_file() {
    // 1. Index file A with chunk content X
    // 2. Index file B with identical chunk content X
    // 3. Verify only one embedding stored
    // 4. Verify search returns both chunks
}

/// Graph traversal accuracy
#[tokio::test]
async fn test_caller_callee_chain() {
    // 1. Index: A calls B calls C
    // 2. Verify find_callers(C) returns [B, A]
    // 3. Verify find_callees(A) returns [B, C]
}
```

### 3. Edge Case Tests

Critical edge cases that could cause corruption or crashes:

```rust
/// Empty database queries
#[tokio::test]
async fn test_search_empty_database();

/// Unicode in queries and content
#[tokio::test]
async fn test_unicode_handling();

/// Very long symbol names
#[tokio::test]
async fn test_long_symbol_names();

/// Concurrent read/write
#[tokio::test]
async fn test_concurrent_access();

/// Database file locked
#[tokio::test]
async fn test_busy_timeout();

/// Extension missing - should gracefully fall back to FTS-only
#[tokio::test]
async fn test_vec_extension_missing_graceful();

/// FTS rebuild after desync
#[tokio::test]
async fn test_fts_rebuild();

/// Large graph traversal (100+ nodes)
#[tokio::test]
async fn test_graph_traversal_100_nodes();
```

### 4. Performance Tests

Basic performance validation (not benchmarks):

```rust
/// Batch insert performance
#[tokio::test]
async fn test_batch_insert_1000_chunks() {
    // Should complete in < 5 seconds
}

/// Search latency
#[tokio::test]
async fn test_search_latency_10k_chunks() {
    // Setup: 10k chunks indexed
    // Search should complete in < 500ms
}
```

## Test Infrastructure

### In-Memory Database

All tests use `:memory:` for speed and isolation:

```rust
async fn setup_test_store() -> SqliteStore {
    let store = SqliteStore::connect(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}
```

### Test Fixtures

Common test data in `tests/fixtures/`:

```rust
mod fixtures {
    pub fn sample_chunk() -> ChunkRecord { ... }
    pub fn sample_file() -> FileRecord { ... }
    pub fn sample_embedding() -> Vec<f32> { ... }
}
```

### Assertion Helpers

```rust
mod test_utils {
    pub fn assert_search_contains(results: &[SearchHit], symbol: &str);
    pub fn assert_search_order(results: &[SearchHit], expected: &[&str]);
    pub fn assert_embedding_count(store: &SqliteStore, expected: usize);
}
```

## Test Execution

### Run All SQLite Tests
```bash
cargo test --features sqlite
```

### Run Specific Module
```bash
cargo test --features sqlite fts::tests
cargo test --features sqlite vector::tests
```

### Run Integration Tests
```bash
cargo test --features sqlite --test sqlite_integration
```

### Run with Output
```bash
cargo test --features sqlite -- --nocapture
```

## Critical Path Coverage

These paths MUST have test coverage:

| Path | Risk if Broken | Tests |
|------|----------------|-------|
| Migration system | Schema not created | `test_migration_*`, `test_fresh_database_*` |
| Extension loading | Vector search broken | `test_sqlite_vec_*` |
| Chunk insertion | Index corruption | `test_insert_chunk*` |
| FTS indexing | Search broken | `test_fts_*` |
| Vector indexing | Semantic search broken | `test_vector_*` |
| Embedding dedup | Storage bloat | `test_embedding_dedup*` |
| Worktree tracking | Wrong results | `test_chunk_worktree*` |
| Graph edges | Missing relationships | `test_graph_*` |
| Graceful degradation | Crash if extension missing | `test_extension_missing_*` |
| File-based storage | Permission/WAL issues | `test_file_based_*` |

## Acceptance Criteria

Before marking implementation complete:

1. **All unit tests pass**: `cargo test --features sqlite`
2. **Integration tests pass**: Full cycle works
3. **No panics**: All error paths return Result
4. **No data corruption**: Multiple runs produce consistent results
5. **Reasonable performance**: Basic operations complete in expected time

## What We Don't Test

- PostgreSQL compatibility (out of scope)
- UI/extension integration (separate project)
- Network scenarios (SQLite is local-only)
- Benchmark precision (basic perf checks only)
