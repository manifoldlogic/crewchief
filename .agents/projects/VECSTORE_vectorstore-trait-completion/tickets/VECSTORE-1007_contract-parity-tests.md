# Ticket: VECSTORE-1007: Contract and Parity Test Suite

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
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create a comprehensive test suite verifying that both `PostgresStore` and `SqliteStore` correctly implement the `VectorStore` trait. This includes contract tests (verify trait behavior) and parity tests (verify both backends produce equivalent results).

## Background
The `VectorStore` trait now has many methods implemented by both backends. We need systematic testing to ensure:
1. Both backends fulfill the trait contract
2. Both backends produce equivalent results for the same inputs
3. Neither backend has regressions as the codebase evolves

**Reference**: Plan Phase 6 - Integration Testing (VECSTORE-1007)

## Acceptance Criteria
- [ ] `tests/vectorstore_contract.rs` created with backend-agnostic tests
- [ ] `tests/backend_parity.rs` created comparing both backends
- [ ] Contract tests pass for PostgresStore
- [ ] Contract tests pass for SqliteStore (with `--features sqlite`)
- [ ] Parity tests verify equivalent results
- [ ] Tests for both 768-dim and 1536-dim embeddings
- [ ] CI updated to run both test suites

## Technical Requirements

### Contract Test Suite

**File: `crates/maproom/tests/vectorstore_contract.rs`**

A single test suite that runs against any `VectorStore` implementation:

```rust
use maproom::db::{VectorStore, FileRecord, ChunkRecord};
use std::sync::Arc;

/// Run all contract tests against a VectorStore implementation
async fn test_vectorstore_contract(store: Arc<dyn VectorStore>) {
    // === Repository Tests ===
    test_repo_creation(&store).await;
    test_repo_lookup(&store).await;
    test_list_repos(&store).await;

    // === Worktree Tests ===
    test_worktree_creation(&store).await;
    test_worktree_lookup(&store).await;
    test_list_worktrees(&store).await;

    // === Indexing Tests ===
    test_file_upsert(&store).await;
    test_chunk_insert(&store).await;
    test_batch_chunk_insert(&store).await;

    // === Embedding Tests ===
    test_embedding_upsert_1536(&store).await;
    test_embedding_upsert_768(&store).await;
    test_batch_embedding_upsert(&store).await;

    // === Search Tests ===
    test_fts_search(&store).await;
    test_vector_search(&store).await;
    test_hybrid_search(&store).await;

    // === Context Tests ===
    test_get_chunk_by_id(&store).await;
    test_get_file_chunks(&store).await;
    test_get_chunk_context(&store).await;

    // === Index State Tests ===
    test_index_state_persistence(&store).await;

    // === Cleanup Tests ===
    test_delete_chunks_by_file(&store).await;
    test_get_chunks_by_blob_sha(&store).await;
}

// Individual test implementations
async fn test_repo_creation(store: &Arc<dyn VectorStore>) {
    let repo_id = store.get_or_create_repo("test-repo", "/tmp/test").await.unwrap();
    assert!(repo_id > 0);

    // Idempotent - same name returns same ID
    let repo_id2 = store.get_or_create_repo("test-repo", "/tmp/test").await.unwrap();
    assert_eq!(repo_id, repo_id2);
}

async fn test_repo_lookup(store: &Arc<dyn VectorStore>) {
    store.get_or_create_repo("lookup-test", "/tmp/lookup").await.unwrap();

    let found = store.get_repo_by_name("lookup-test").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "lookup-test");

    let not_found = store.get_repo_by_name("nonexistent").await.unwrap();
    assert!(not_found.is_none());
}

// ... more test implementations ...

// Entry points for each backend
#[tokio::test]
async fn test_postgres_contract() {
    let store = Arc::new(maproom::db::postgres::PostgresStore::connect().await.unwrap());
    test_vectorstore_contract(store).await;
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_sqlite_contract() {
    let store = Arc::new(maproom::db::sqlite::SqliteStore::connect(":memory:").await.unwrap());
    test_vectorstore_contract(store).await;
}
```

### Parity Test Suite

**File: `crates/maproom/tests/backend_parity.rs`**

Compare both backends with identical data:

```rust
use maproom::db::{VectorStore, postgres::PostgresStore, sqlite::SqliteStore};
use std::sync::Arc;
use std::collections::HashSet;

/// Setup identical test data in both backends
async fn setup_test_data(store: &Arc<dyn VectorStore>) {
    let repo_id = store.get_or_create_repo("parity-test", "/tmp/parity").await.unwrap();
    let wt_id = store.get_or_create_worktree(repo_id, "main", "/tmp/parity").await.unwrap();
    let commit_id = store.get_or_create_commit(repo_id, "abc123", None).await.unwrap();

    // Insert test files and chunks...
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_fts_search_parity() {
    let pg_store: Arc<dyn VectorStore> = Arc::new(PostgresStore::connect().await.unwrap());
    let sqlite_store: Arc<dyn VectorStore> = Arc::new(SqliteStore::connect(":memory:").await.unwrap());

    setup_test_data(&pg_store).await;
    setup_test_data(&sqlite_store).await;

    // Run same FTS query
    let pg_hits = pg_store.search_chunks_fts("parity-test", None, "function", 10, false).await.unwrap();
    let sqlite_hits = sqlite_store.search_chunks_fts("parity-test", None, "function", 10, false).await.unwrap();

    // Verify same chunks returned (order may differ due to ranking algorithms)
    assert_eq!(pg_hits.len(), sqlite_hits.len(), "Same number of results");

    let pg_ids: HashSet<_> = pg_hits.iter().map(|h| h.chunk_id).collect();
    let sqlite_ids: HashSet<_> = sqlite_hits.iter().map(|h| h.chunk_id).collect();
    assert_eq!(pg_ids, sqlite_ids, "Same chunks returned");

    // DO NOT compare absolute scores - algorithms differ (BM25 vs ts_rank_cd)
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_vector_search_parity() {
    let pg_store: Arc<dyn VectorStore> = Arc::new(PostgresStore::connect().await.unwrap());
    let sqlite_store: Arc<dyn VectorStore> = Arc::new(SqliteStore::connect(":memory:").await.unwrap());

    setup_test_data(&pg_store).await;
    setup_test_data(&sqlite_store).await;

    let embedding = vec![0.1f32; 1536];

    let pg_hits = pg_store.search_chunks_vector("parity-test", None, &embedding, 10, false).await.unwrap();
    let sqlite_hits = sqlite_store.search_chunks_vector("parity-test", None, &embedding, 10, false).await.unwrap();

    // Vector search with same embedding should return same results in same order
    assert_eq!(pg_hits.len(), sqlite_hits.len());
    for (pg, sq) in pg_hits.iter().zip(sqlite_hits.iter()) {
        assert_eq!(pg.chunk_id, sq.chunk_id, "Same chunk IDs in same order");
        // Scores should be similar (both use cosine similarity)
        assert!((pg.score - sq.score).abs() < 0.01, "Similar scores");
    }
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_context_retrieval_parity() {
    // Test that get_chunk_by_id returns equivalent data
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_cleanup_parity() {
    // Test that cleanup operations return equivalent counts
}
```

### Dimension Tests

**Test both 768-dim (Ollama) and 1536-dim (OpenAI) embeddings:**

```rust
#[tokio::test]
async fn test_embedding_768_dim() {
    let store = get_test_store().await;
    let embedding_768 = vec![0.1f32; 768];

    // Should not error
    store.upsert_embeddings(chunk_id, Some(&embedding_768), None, 768).await.unwrap();

    // Search should work
    let hits = store.search_chunks_vector("repo", None, &embedding_768, 10, false).await.unwrap();
    assert!(!hits.is_empty());
}

#[tokio::test]
async fn test_embedding_1536_dim() {
    let store = get_test_store().await;
    let embedding_1536 = vec![0.1f32; 1536];

    store.upsert_embeddings(chunk_id, Some(&embedding_1536), None, 1536).await.unwrap();

    let hits = store.search_chunks_vector("repo", None, &embedding_1536, 10, false).await.unwrap();
    assert!(!hits.is_empty());
}
```

### Test Fixtures

**File: `crates/maproom/tests/fixtures/mod.rs`**

```rust
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

pub fn sample_embedding_768() -> Vec<f32> {
    vec![0.1; 768]
}

pub fn sample_embedding_1536() -> Vec<f32> {
    vec![0.1; 1536]
}
```

## Implementation Notes

### Test Commands

```bash
# PostgreSQL contract tests (requires PostgreSQL running)
cargo test --test vectorstore_contract

# SQLite contract tests
cargo test --features sqlite --test vectorstore_contract

# Parity tests (requires both backends)
cargo test --features sqlite --test backend_parity

# All tests
cargo test --features sqlite
```

### CI Configuration

Update `.github/workflows/test.yml`:

```yaml
test-postgres:
  services:
    postgres:
      image: ankane/pgvector:latest
      env:
        POSTGRES_USER: maproom
        POSTGRES_PASSWORD: maproom
        POSTGRES_DB: maproom_test
  steps:
    - run: cargo test --test vectorstore_contract

test-sqlite:
  steps:
    - run: cargo test --features sqlite --test vectorstore_contract

test-parity:
  services:
    postgres:
      image: ankane/pgvector:latest
  steps:
    - run: cargo test --features sqlite --test backend_parity
```

### Known Parity Differences

Document in test comments:
1. FTS ranking order may differ (BM25 vs ts_rank_cd)
2. Hybrid search uses RRF which normalizes differences
3. Vector search should be identical (same cosine similarity)

## Dependencies
- **All VECSTORE tickets (1000-1006)**: Must complete before comprehensive testing

## Risk Assessment
- **Risk**: Tests flaky due to database state
  - **Mitigation**: Each test creates own data, use unique names
- **Risk**: PostgreSQL not available in CI
  - **Mitigation**: Use GitHub Actions service containers
- **Risk**: SQLite tests pass but PostgreSQL fails (or vice versa)
  - **Mitigation**: Parity tests catch divergent behavior

## Files/Packages Affected
- `crates/maproom/tests/vectorstore_contract.rs` (NEW)
- `crates/maproom/tests/backend_parity.rs` (NEW)
- `crates/maproom/tests/fixtures/mod.rs` (NEW)
- `.github/workflows/test.yml` (UPDATE)
