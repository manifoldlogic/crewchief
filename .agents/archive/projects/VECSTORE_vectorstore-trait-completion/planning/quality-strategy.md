# VECSTORE Quality Strategy

## Testing Philosophy

This project expands an existing, working trait. Quality strategy focuses on:
1. **Parity Testing**: Both backends produce equivalent results for same inputs
2. **Regression Prevention**: Existing trait methods continue working
3. **Contract Testing**: New methods fulfill their documented contracts

## Test Layers

### Layer 1: Unit Tests (Per Backend)

Each backend has isolated unit tests that verify implementation details.

**PostgreSQL** (`crates/maproom/src/db/postgres/tests/`):
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_search_chunks_vector() {
        // Requires PostgreSQL service
        let store = PostgresStore::connect().await.unwrap();
        // ... test PostgreSQL-specific behavior
    }
}
```

**SQLite** (`crates/maproom/src/db/sqlite/*.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_vector_search_with_sqlite_vec() {
        // In-memory SQLite, no external service
        let conn = Connection::open_in_memory().unwrap();
        // ... test SQLite-specific behavior
    }
}
```

### Layer 2: Trait Contract Tests

A single test suite that runs against any `VectorStore` implementation.

```rust
// crates/maproom/tests/vectorstore_contract.rs

async fn test_vectorstore_contract(store: Arc<dyn VectorStore>) {
    // Setup: create repo, worktree, commit
    let repo_id = store.get_or_create_repo("test", "/tmp/test").await.unwrap();
    let wt_id = store.get_or_create_worktree(repo_id, "main", "/tmp/test").await.unwrap();
    let commit_id = store.get_or_create_commit(repo_id, "abc123", None).await.unwrap();

    // Test: upsert file
    let file = FileRecord { repo_id, worktree_id: wt_id, commit_id, relpath: "test.rs".into(), ... };
    let file_id = store.upsert_file(&file).await.unwrap();
    assert!(file_id > 0);

    // Test: insert chunks
    let chunk = ChunkRecord { file_id, blob_sha: "sha256".into(), ... };
    let chunk_id = store.insert_chunk(&chunk).await.unwrap();
    assert!(chunk_id > 0);

    // Test: search
    let hits = store.search_chunks_fts("test", Some("main"), "test", 10, false).await.unwrap();
    assert!(!hits.is_empty());
}

#[tokio::test]
async fn test_postgres_contract() {
    let store = Arc::new(PostgresStore::connect().await.unwrap());
    test_vectorstore_contract(store).await;
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_sqlite_contract() {
    let store = Arc::new(SqliteStore::connect(":memory:").await.unwrap());
    test_vectorstore_contract(store).await;
}
```

### Layer 3: Parity Tests

Verify both backends return equivalent results for identical operations.

**IMPORTANT: Ranking Algorithm Differences**

PostgreSQL and SQLite use fundamentally different ranking algorithms:
- **PostgreSQL FTS**: `ts_rank_cd` with tsvector (cover density ranking)
- **SQLite FTS5**: BM25 (probabilistic relevance ranking)
- **PostgreSQL vector**: pgvector with cosine distance
- **SQLite vector**: sqlite-vec with cosine distance (similar)

**Parity Testing Strategy**: Compare **rank order**, not absolute scores.

```rust
// crates/maproom/tests/backend_parity.rs

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_fts_search_parity() {
    let pg_store = Arc::new(PostgresStore::connect().await.unwrap());
    let sqlite_store = Arc::new(SqliteStore::connect(":memory:").await.unwrap());

    // Setup identical data in both
    setup_test_data(&pg_store).await;
    setup_test_data(&sqlite_store).await;

    // Run same query
    let pg_hits = pg_store.search_chunks_fts("repo", None, "function", 10, false).await.unwrap();
    let sqlite_hits = sqlite_store.search_chunks_fts("repo", None, "function", 10, false).await.unwrap();

    // Verify same results found (may be in different order for FTS)
    assert_eq!(pg_hits.len(), sqlite_hits.len());

    // For FTS: verify same chunks returned (order may differ due to ranking algorithms)
    let pg_ids: HashSet<_> = pg_hits.iter().map(|h| &h.chunk_id).collect();
    let sqlite_ids: HashSet<_> = sqlite_hits.iter().map(|h| &h.chunk_id).collect();
    assert_eq!(pg_ids, sqlite_ids, "Both backends should return same chunks");

    // DO NOT compare absolute scores - algorithms are fundamentally different
    // Instead, verify relative ranking for top-3 matches (most relevant should be similar)
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_vector_search_parity() {
    // ... setup ...

    // Vector search with same embedding should return same results
    // Ranking SHOULD be similar since both use cosine similarity
    let pg_hits = pg_store.search_chunks_vector("repo", None, &embedding, 10, false).await.unwrap();
    let sqlite_hits = sqlite_store.search_chunks_vector("repo", None, &embedding, 10, false).await.unwrap();

    // Verify same results in same order (vector search should be deterministic)
    assert_eq!(pg_hits.len(), sqlite_hits.len());
    for (pg, sq) in pg_hits.iter().zip(sqlite_hits.iter()) {
        assert_eq!(pg.chunk_id, sq.chunk_id, "Chunk IDs should match");
        // Vector similarity scores should be close (same algorithm)
        assert!((pg.score - sq.score).abs() < 0.01, "Vector scores should be similar");
    }
}
```

**Known Differences to Document**:
1. FTS ranking order may differ (BM25 vs ts_rank_cd)
2. Hybrid search uses RRF fusion which normalizes ranking differences
3. Vector search should produce identical ranking (same cosine similarity)

### Layer 4: Integration Tests

End-to-end tests using `get_store()` factory.

```rust
// crates/maproom/tests/integration/vectorstore_integration.rs

#[tokio::test]
async fn test_get_store_returns_functional_store() {
    let store = crate::db::factory::get_store().await.unwrap();

    // Should work regardless of backend
    let repos = store.list_repos().await.unwrap();
    // Empty is fine for fresh database

    let repo_id = store.get_or_create_repo("integration-test", "/tmp").await.unwrap();
    assert!(repo_id > 0);
}
```

## Critical Test Paths

### Must Pass (Blocking)

1. **Migration Tests**
   - `test_migration_fresh_database` - Clean schema creation
   - `test_migration_idempotent` - Re-running migrations doesn't break
   - `test_migration_upgrade_path` - Upgrading from previous versions

2. **Search Tests**
   - `test_fts_search_returns_results` - Basic FTS works
   - `test_vector_search_cosine_similarity` - Vector search ranks correctly
   - `test_hybrid_search_combines_scores` - Hybrid fusion works

3. **Context Tests**
   - `test_get_chunk_context_returns_surrounding` - Context assembly works
   - `test_get_chunk_by_id_existing` - Chunk lookup works
   - `test_get_chunk_by_id_missing` - Returns None, not error

4. **Index State Tests**
   - `test_update_index_state_records_progress` - State persists
   - `test_get_last_indexed_tree` - Can retrieve saved state

5. **Parity Tests**
   - `test_fts_search_parity` - FTS results equivalent
   - `test_chunk_insertion_parity` - Insert/retrieve identical

### Should Pass (Important but Non-Blocking)

1. **Performance Tests**
   - `test_batch_insert_faster_than_individual` - Batching provides benefit
   - `test_search_under_50ms` - Search latency acceptable

2. **Edge Case Tests**
   - `test_empty_query_returns_empty` - Graceful empty handling
   - `test_special_characters_in_search` - SQL injection prevention
   - `test_unicode_in_symbol_names` - UTF-8 handling

### Nice to Have (Future Enhancement)

1. **Concurrency Tests**
   - `test_concurrent_inserts` - Thread safety
   - `test_concurrent_search_during_insert` - Read/write isolation

## Test Commands

```bash
# All PostgreSQL tests (requires PostgreSQL running)
cargo test --lib

# All SQLite tests (no external dependencies)
cargo test --features sqlite --lib

# Specific module tests
cargo test --features sqlite --lib db::sqlite::fts
cargo test --lib db::postgres

# Integration tests (requires PostgreSQL)
cargo test --test vectorstore_contract

# Integration tests with SQLite
cargo test --features sqlite --test vectorstore_contract

# Parity tests (requires both backends)
cargo test --features sqlite --test backend_parity
```

## Test Data Strategy

### Fixtures

```rust
// crates/maproom/tests/fixtures/mod.rs

pub fn sample_chunk() -> ChunkRecord {
    ChunkRecord {
        file_id: 1,
        blob_sha: "abc123".into(),
        symbol_name: Some("test_function".into()),
        kind: "func".into(),
        signature: Some("fn test_function()".into()),
        docstring: Some("Test function docstring".into()),
        start_line: 1,
        end_line: 10,
        preview: "fn test_function() { ... }".into(),
        ts_doc_text: "test function example".into(),
        recency_score: 0.5,
        churn_score: 0.3,
        metadata: None,
        worktree_id: 1,
    }
}

pub fn sample_embedding() -> Vec<f32> {
    vec![0.1; 1536]  // OpenAI dimension
}
```

### In-Memory Databases

SQLite tests use `:memory:` for speed and isolation:
```rust
SqliteStore::connect(":memory:").await
```

PostgreSQL tests use a dedicated test database:
```rust
// Set MAPROOM_DATABASE_URL_TEST for test database
PostgresStore::connect().await  // Uses env var
```

### PostgreSQL Test Database Setup

**Local Development**:
```bash
# Start PostgreSQL with pgvector using Docker
docker run -d --name maproom-postgres-test \
  -e POSTGRES_USER=maproom \
  -e POSTGRES_PASSWORD=maproom \
  -e POSTGRES_DB=maproom_test \
  -p 5433:5432 \
  ankane/pgvector:latest

# Set environment variable for tests
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5433/maproom_test"

# Run tests
cargo test --lib
```

**CI Environment** (GitHub Actions):
```yaml
services:
  postgres:
    image: ankane/pgvector:latest
    env:
      POSTGRES_USER: maproom
      POSTGRES_PASSWORD: maproom
      POSTGRES_DB: maproom_test
    ports:
      - 5432:5432
    options: >-
      --health-cmd pg_isready
      --health-interval 10s
      --health-timeout 5s
      --health-retries 5

env:
  MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5432/maproom_test
```

**Test Database Isolation**:
- Each test creates/drops its own tables via migrations
- Tests use unique repo/worktree names to avoid conflicts
- Consider using transactions with rollback for isolation (future enhancement)

## Risk Mitigation

### Risk: Trait Change Breaks Existing Code
**Mitigation**:
- Add new methods with default implementations first
- Run existing test suite before removing defaults
- CI runs all tests on every PR

### Risk: Backend Behavioral Differences
**Mitigation**:
- Parity tests catch divergent behavior
- Document known differences (e.g., FTS ranking algorithms)
- Use tolerance for floating-point comparisons

### Risk: Missing Edge Cases
**Mitigation**:
- Fuzzing with proptest for input validation
- Review PostgreSQL queries.rs for edge case handling
- Port all existing tests to contract test format

## CI Integration

```yaml
# .github/workflows/test.yml
test-postgres:
  services:
    postgres:
      image: ankane/pgvector:latest
  steps:
    - run: cargo test --lib
    - run: cargo test --test vectorstore_contract

test-sqlite:
  steps:
    - run: cargo test --features sqlite --lib
    - run: cargo test --features sqlite --test vectorstore_contract

test-parity:
  services:
    postgres:
      image: ankane/pgvector:latest
  steps:
    - run: cargo test --features sqlite --test backend_parity
```

## Definition of Done

A ticket is complete when:

1. ✅ All new trait methods have implementations in both backends
2. ✅ Contract tests pass for both backends
3. ✅ Parity tests pass (where applicable)
4. ✅ `cargo clippy --features sqlite` has no new warnings
5. ✅ Existing tests still pass (no regressions)
6. ✅ CI passes all test jobs
