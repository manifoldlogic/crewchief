# Ticket: SQLITE-6001: Integration Test Suite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration tests that validate the complete SQLite pipeline from indexing through search.

## Background
Individual components are tested, but we need end-to-end tests that verify the complete workflow: index files → store embeddings → search (FTS + vector + hybrid) → get results. This catches integration issues missed by unit tests.

Implements: Plan Phase 6 - Integration Testing

## Acceptance Criteria
- [ ] Test full index→embed→search cycle end-to-end
- [ ] Test multi-worktree scenarios (same file in multiple worktrees)
- [ ] Test embedding deduplication across files (same content, one embedding)
- [ ] Test graph traversal with real code relationships
- [ ] File-based integration test (not just :memory:)
- [ ] Performance sanity checks pass (not benchmarks, just "doesn't hang")
- [ ] All tests pass: `cargo test --features sqlite --test sqlite_integration`

## Technical Requirements
Create `crates/maproom/tests/sqlite_integration.rs`:

```rust
use maproom::db::sqlite::SqliteStore;
use tempfile::tempdir;

/// Complete indexing + search cycle
#[tokio::test]
async fn test_full_index_search_cycle() {
    let store = setup_test_store_memory().await;

    // 1. Create repo/worktree/commit
    let repo_id = store.ensure_repo("test-repo", "/path/to/repo").await.unwrap();
    let worktree_id = store.ensure_worktree(repo_id, "main", "/path/to/repo").await.unwrap();
    let commit_id = store.ensure_commit(repo_id, "abc123", None).await.unwrap();

    // 2. Index files with chunks
    let file_id = store.upsert_file(
        repo_id, worktree_id, commit_id,
        "src/lib.rs", "rust", "hash1", 1000, None
    ).await.unwrap();

    let chunk_id = store.insert_chunk(file_id, &ChunkRecord {
        blob_sha: "sha1".to_string(),
        symbol_name: Some("authenticate".to_string()),
        kind: "function".to_string(),
        preview: "pub fn authenticate(user: &str) -> Result<Token>".to_string(),
        // ...
    }, &[worktree_id]).await.unwrap();

    // 3. Store embedding
    let embedding = generate_test_embedding("authenticate function");
    store.upsert_embedding("sha1", &embedding, "test-model").await.unwrap();

    // 4. Search - FTS
    let fts_results = store.search_chunks_fts("test-repo", None, "authenticate", 10).await.unwrap();
    assert!(!fts_results.is_empty());

    // 5. Search - Vector
    let query_embedding = generate_test_embedding("login authentication");
    let vec_results = store.search_vector("test-repo", None, &query_embedding, 10).await.unwrap();
    // May be empty if extension not loaded in test

    // 6. Search - Hybrid
    let hybrid_results = store.search_hybrid(
        "test-repo", None, "authenticate", &query_embedding, 10,
        HybridWeights::default()
    ).await.unwrap();
    assert!(!hybrid_results.is_empty());
}

/// Multi-worktree scenario
#[tokio::test]
async fn test_multi_worktree_index() {
    let store = setup_test_store_memory().await;

    // Create repo with 2 worktrees
    let repo_id = store.ensure_repo("test-repo", "/path").await.unwrap();
    let wt1 = store.ensure_worktree(repo_id, "main", "/path/main").await.unwrap();
    let wt2 = store.ensure_worktree(repo_id, "feature", "/path/feature").await.unwrap();

    // Index same file in both worktrees
    // ... setup ...

    // Search in specific worktree
    let results_main = store.search_chunks_fts("test-repo", Some("main"), "query", 10).await.unwrap();
    let results_feature = store.search_chunks_fts("test-repo", Some("feature"), "query", 10).await.unwrap();
    let results_all = store.search_chunks_fts("test-repo", None, "query", 10).await.unwrap();

    // Verify filtering works
    assert!(!results_all.is_empty());
}

/// Embedding deduplication across files
#[tokio::test]
async fn test_embedding_dedup_cross_file() {
    let store = setup_test_store_memory().await;

    // Insert two files with identical content (same blob_sha)
    // ... setup two files with same content ...

    // Store embedding once
    let embedding = generate_test_embedding("test content");
    store.upsert_embedding("same_sha", &embedding, "test").await.unwrap();

    // Verify only one embedding stored
    let has = store.has_embedding("same_sha").await.unwrap();
    assert!(has);

    // Search should return both chunks
    // ... verify search finds both files ...
}

/// File-based integration (real temp file, not :memory:)
#[tokio::test]
async fn test_file_based_integration() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test maproom.db");  // Intentional space

    // 1. Create store with file path
    let store = SqliteStore::connect(db_path.to_str().unwrap()).await.unwrap();
    store.migrate().await.unwrap();

    // 2. Insert data
    let repo_id = store.ensure_repo("test", "/test").await.unwrap();
    let wt_id = store.ensure_worktree(repo_id, "main", "/test").await.unwrap();

    // 3. Close connection
    drop(store);

    // 4. Reopen and verify data persisted
    let store = SqliteStore::connect(db_path.to_str().unwrap()).await.unwrap();
    let repo = store.get_repo("test").await.unwrap();
    assert!(repo.is_some());

    // 5. Verify file permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::metadata(&db_path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600, "File should be 0600");
    }
}

/// Performance sanity check
#[tokio::test]
async fn test_batch_insert_performance() {
    let store = setup_test_store_memory().await;

    // Insert 1000 chunks
    let start = std::time::Instant::now();

    // ... batch insert ...

    let elapsed = start.elapsed();
    assert!(elapsed.as_secs() < 5, "Batch insert took {:?}", elapsed);
}

fn generate_test_embedding(text: &str) -> Vec<f32> {
    // Simple deterministic embedding for tests
    // Real embedding would use actual embedding model
    let hash = text.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
    (0..1536).map(|i| ((hash + i) % 1000) as f32 / 1000.0).collect()
}
```

## Implementation Notes
- Separate integration tests from unit tests
- Use real tempfile for file-based tests to catch path/permission issues
- Generate deterministic fake embeddings for testing (real embedding model not needed)
- Performance tests are sanity checks, not benchmarks
- Test names should be descriptive for easy failure diagnosis

## Dependencies
- All Phase 0-5 tickets complete

## Risk Assessment
- **Risk**: Integration tests slow in CI
  - **Mitigation**: Keep tests focused; use :memory: where possible
- **Risk**: File permission tests fail on some systems
  - **Mitigation**: Use #[cfg(unix)] guard; document Windows behavior

## Files/Packages Affected
- `crates/maproom/tests/sqlite_integration.rs` (NEW)
- `crates/maproom/Cargo.toml` (may need test dependencies)
